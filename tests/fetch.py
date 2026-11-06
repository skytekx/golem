# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: download the JDK, the Forge server and Gradle that the notebooks run against.

    python3 -I tests/fetch.py
    python3 -I tests/fetch.py --only jdk
    python3 -I tests/fetch.py --cache path/to/cache

Every download is checked by its SHA-256 before it is used, and lands under tests/.cache/, which
git ignores. The pins are these.

1. Temurin 17.0.21+9 of Eclipse Adoptium, the JDK the Mac mini runs the server on, unpacked to
   tests/.cache/jdk-17.0.21+9. On macOS the Java home is the Contents/Home of the bundle, and
   the script links it to the same name.
2. The installer of Forge 1.20.1-47.4.18 from the Forge maven, which installs the server into
   tests/.cache/forge-server. It downloads the server jar of Minecraft from Mojang and the
   libraries, as the installer does for every Forge server.
3. Gradle 8.14.3, which builds the bridge under bridge/ with ForgeGradle, unpacked to
   tests/.cache/gradle-8.14.3. The bridge carries no Gradle wrapper, whose jar is a binary under
   another license.

The script accepts the EULA of Minecraft in that test server, with eula=true in its eula.txt,
on behalf of the owner of SkyTekx, who runs the live server under it. The test server binds
localhost only and runs in offline mode, so no account of Mojang is involved.

A pin that is already in place is kept, so a second run downloads nothing. The exit status is 1
when a download fails or its hash differs.
"""

from __future__ import annotations

import argparse
import hashlib
import platform
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

#: The root of the tree, one level above this file.
ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / "tests" / ".cache"

JDK_VERSION = "17.0.21+9"
JDK_DIRECTORY = "jdk-17.0.21+9"
JDK_BASE = "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.21%2B9/"
#: The archive and its SHA-256 per system and machine, as the API of Adoptium names them.
JDK_PACKAGES = {
    ("Linux", "x86_64"): ("OpenJDK17U-jdk_x64_linux_hotspot_17.0.21_9.tar.gz",
                          "8b9b611d917e3dfd8419d3566b5949226f69b052fed7e109c1368add1dfe8711"),
    ("Linux", "aarch64"): ("OpenJDK17U-jdk_aarch64_linux_hotspot_17.0.21_9.tar.gz",
                           "3e4eca7ead088f080af1476007decf010a240a1eb4e1e78c54fa32fbcc347084"),
    ("Darwin", "x86_64"): ("OpenJDK17U-jdk_x64_mac_hotspot_17.0.21_9.tar.gz",
                           "868f5c672b3389fdbe48e44f94ae170f01668b1401360944fc7159d7cc9772cc"),
    ("Darwin", "arm64"): ("OpenJDK17U-jdk_aarch64_mac_hotspot_17.0.21_9.tar.gz",
                          "78b08c7f5a0598242d5719c022e19479cb4a3aa45c37c19726a757a469ed1c15"),
}

FORGE_VERSION = "1.20.1-47.4.18"
FORGE_INSTALLER = ("https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.18/"
                   "forge-1.20.1-47.4.18-installer.jar")
FORGE_SHA256 = "24a530c2b5cc277ee5fc8af2b87ca03b373aa7ce75946c54b430384146f44099"
FORGE_DIRECTORY = "forge-server"

GRADLE_VERSION = "8.14.3"
GRADLE_URL = "https://services.gradle.org/distributions/gradle-8.14.3-bin.zip"
GRADLE_SHA256 = "bd71102213493060956ec229d946beee57158dbd89d0e62b91bca0fa2c5f3531"
GRADLE_DIRECTORY = "gradle-8.14.3"

#: The settings of the test server: offline, on localhost, a small world without spawn
#: protection. A notebook that boots a copy sets its own port in the copy.
SERVER_PROPERTIES = """\
online-mode=false
server-ip=127.0.0.1
server-port=25599
spawn-protection=0
enforce-secure-profile=false
view-distance=4
simulation-distance=4
motd=Golem test server
"""


class Failure(Exception):
    """A failure of a download, printed without a traceback."""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def download(url: str, target: Path, expected: str) -> None:
    """Download `url` to `target` and check its SHA-256, keeping a file that already matches."""
    if target.is_file() and sha256(target) == expected:
        return
    partial = target.with_name(target.name + ".part")
    print("fetch: %s" % url, flush=True)
    request = urllib.request.Request(url, headers={"User-Agent": "golem-harness"})
    with urllib.request.urlopen(request, timeout=300) as response, open(partial, "wb") as stream:
        shutil.copyfileobj(response, stream)
    found = sha256(partial)
    if found != expected:
        partial.unlink()
        raise Failure("%s has SHA-256 %s, the pin is %s" % (url, found, expected))
    partial.replace(target)


def fetch_jdk(cache: Path) -> Path:
    """Temurin 17 under `cache`, answering its Java home."""
    key = (platform.system(), platform.machine())
    if key not in JDK_PACKAGES:
        raise Failure("no pin of Temurin %s for %s on %s" % (JDK_VERSION, key[1], key[0]))
    name, expected = JDK_PACKAGES[key]
    home = cache / JDK_DIRECTORY
    if (home / "bin" / "java").exists():
        return home
    archive = cache / name
    download(JDK_BASE + name, archive, expected)
    unpacked = cache / ("unpack-" + JDK_DIRECTORY)
    shutil.rmtree(unpacked, ignore_errors=True)
    unpacked.mkdir(parents=True)
    with tarfile.open(archive) as bundle:
        bundle.extractall(unpacked, filter="data")
    inner = unpacked / JDK_DIRECTORY
    if (inner / "Contents" / "Home").is_dir():
        inner = inner / "Contents" / "Home"
    inner.rename(home)
    shutil.rmtree(unpacked, ignore_errors=True)
    return home


def fetch_forge(cache: Path, java_home: Path) -> Path:
    """The Forge server under `cache`, installed by the installer on Temurin 17."""
    server = cache / FORGE_DIRECTORY
    marker = server / "libraries" / "net" / "minecraftforge" / "forge" / FORGE_VERSION / "unix_args.txt"
    if not marker.is_file():
        installer = cache / ("forge-%s-installer.jar" % FORGE_VERSION)
        download(FORGE_INSTALLER, installer, FORGE_SHA256)
        shutil.rmtree(server, ignore_errors=True)
        server.mkdir(parents=True)
        print("fetch: installing Forge %s" % FORGE_VERSION, flush=True)
        result = subprocess.run([str(java_home / "bin" / "java"), "-jar", str(installer), "--installServer", "."],
                                cwd=server, capture_output=True, text=True, check=False)
        if result.returncode != 0 or not marker.is_file():
            raise Failure("the Forge installer failed:\n%s" % result.stdout[-4000:])
    (server / "eula.txt").write_text(
        "# Accepted for this test server on behalf of the owner of SkyTekx, see tests/fetch.py.\n"
        "eula=true\n", encoding="utf-8")
    (server / "server.properties").write_text(SERVER_PROPERTIES, encoding="utf-8")
    return server


def fetch_gradle(cache: Path) -> Path:
    """Gradle under `cache`, answering the directory whose bin/ holds the gradle script."""
    home = cache / GRADLE_DIRECTORY
    if (home / "bin" / "gradle").is_file():
        return home
    archive = cache / ("gradle-%s-bin.zip" % GRADLE_VERSION)
    download(GRADLE_URL, archive, GRADLE_SHA256)
    with zipfile.ZipFile(archive) as bundle:
        bundle.extractall(cache)
    # The zip format of the standard library keeps no file modes, so the scripts need theirs back.
    for script in (home / "bin").iterdir():
        script.chmod(0o755)
    return home


def shown(path: Path) -> str:
    """A path relative to the root where it lies below it, so no machine path is printed."""
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return path.name


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Download the JDK and the Forge server of the notebooks.")
    parser.add_argument("--only", choices=["jdk", "forge", "gradle"], help="Fetch one of the three alone.")
    parser.add_argument("--cache", type=Path, default=CACHE, help="The directory, by default tests/.cache.")
    args = parser.parse_args(argv)
    cache = args.cache.resolve()
    cache.mkdir(parents=True, exist_ok=True)
    try:
        if args.only in (None, "jdk", "forge"):
            java_home = fetch_jdk(cache)
            print("fetch: Temurin %s at %s" % (JDK_VERSION, shown(java_home)))
        if args.only in (None, "forge"):
            server = fetch_forge(cache, java_home)
            print("fetch: Forge %s at %s" % (FORGE_VERSION, shown(server)))
        if args.only in (None, "gradle"):
            gradle = fetch_gradle(cache)
            print("fetch: Gradle %s at %s" % (GRADLE_VERSION, shown(gradle)))
    except (Failure, OSError) as broken:
        print("fetch: %s" % broken, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
