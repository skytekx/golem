# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: download the JDK, the Forge server and Gradle that the notebooks run against.

    python3 -I tests/fetch.py
    python3 -I tests/fetch.py --only jdk
    python3 -I tests/fetch.py --cache path/to/cache
    python3 -I tests/fetch.py --only modpack
    python3 -I tests/fetch.py --only modpack --server-repo path/to/skytekx3-1201-server

Every download is checked by its SHA-256, or by the SHA-512 of the index of the pack, before it is
used, and lands under tests/.cache/, which git ignores. The pins are these.

1. Temurin 17.0.21+9 of Eclipse Adoptium, the JDK the Mac mini runs the server on, unpacked to
   tests/.cache/jdk-17.0.21+9. On macOS the Java home is the Contents/Home of the bundle, and
   the script links it to the same name.
2. The installer of Forge 1.20.1-47.4.18 from the Forge maven, which installs the server into
   tests/.cache/forge-server. It downloads the server jar of Minecraft from Mojang and the
   libraries, as the installer does for every Forge server.
3. Gradle 8.14.3, which builds the bridge under bridge/ with ForgeGradle, unpacked to
   tests/.cache/gradle-8.14.3. The bridge carries no Gradle wrapper, whose jar is a binary under
   another license.
4. The SkyTekx 3 modpack as the live server runs it, from the repository of the server at one
   commit, only with --only modpack, since it is about 1 GB. The script clones that commit into
   tests/.cache/skytekx3-1201-server, or reads it from a checkout that --server-repo names and
   leaves that checkout as it is. Every file comes from the commit itself, not from the work
   tree of a checkout. It downloads the files of modrinth.index.json that the server requires
   into tests/.cache/modpack/mods, each checked by the SHA-512 and the size the index gives,
   adds the jars of local-mods/, and copies config/, defaultconfigs/, kubejs/, world/datapacks/,
   server.properties.example, the systemd unit of the live server, the Arclight jar with its
   four files of settings and tools/patch_mods.py beside them. The jars stay as their authors
   published them, since tools/patch_mods.py rewrites 14 of them, which would then miss their
   hashes, so a notebook patches its own copy as the live server patches its mods/. The file
   tests/.cache/modpack/modpack.json, written last, names the commit and every jar. A download
   that broke off goes on where it stopped, and a jar whose hash already matches stays.

The script accepts the EULA of Minecraft in that test server, with eula=true in its eula.txt,
on behalf of the owner of SkyTekx, who runs the live server under it. The test server binds
localhost only and runs in offline mode, so no account of Mojang is involved.

A pin that is already in place is kept, so a second run downloads nothing. The exit status is 1
when a download fails or its hash differs.

The jars of the pack are the work of their authors under their own licenses, the files of the
server repository are the work of SkyTekx. The script only puts them where the notebooks find
them, in the ignored cache, and Golem ships none of them.
"""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import urllib.parse
import urllib.request
import zipfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path, PurePosixPath

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

#: The repository of the SkyTekx 3 server and the commit whose pack the notebooks boot.
SERVER_REPOSITORY = "https://github.com/skytekx/skytekx3-1201-server"
SERVER_COMMIT = "3fa5ae681d12233fe81003fbfe5a2d3c529747fc"
SERVER_DIRECTORY = "skytekx3-1201-server"
MODPACK_DIRECTORY = "modpack"
#: The hosts that a download of the index may name, as the format of Modrinth packs allows them.
MODPACK_HOSTS = ("cdn.modrinth.com", "github.com")
#: The directories of the server repository that the pack takes beside its mods.
MODPACK_TREES = ("config", "defaultconfigs", "kubejs", "world/datapacks")
#: The jar of Arclight that the live server runs, which arclight.jar of the repository links to.
ARCLIGHT_JAR = "arclight-forge-1.20.1-1.0.6-6de9fec-safeiter2.jar"
ARCLIGHT_SHA256 = "c55ad477b92250ef31baf965aae3158ffa99239b9b44a60b3a9dd5206dc1557f"
#: The files of the server repository that the pack takes: the settings of the live server and the
#: systemd unit with the options of its JVM, the jar of Arclight with the files of settings it
#: reads, and the script that patches jars of mods/ on the live server.
MODPACK_FILES = ("server.properties.example", "systemd/skytekx3.service", ARCLIGHT_JAR, "arclight.conf",
                 "bukkit.yml", "spigot.yml", "commands.yml", "tools/patch_mods.py")
#: The downloads that run at once, and the tries of one download before the script gives up.
MODPACK_WORKERS = 4
MODPACK_TRIES = 3

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


def digest(path: Path, algorithm: str) -> str:
    """The hex digest of the file at `path` by `algorithm` of hashlib."""
    state = hashlib.new(algorithm)
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            state.update(block)
    return state.hexdigest()


def git(arguments: list, cwd: Path, output=subprocess.PIPE) -> bytes:
    """Run git with `arguments` in `cwd` and answer what it printed, raising on a failure."""
    result = subprocess.run(["git", "-c", "advice.detachedHead=false", *arguments], cwd=cwd, stdout=output,
                            stderr=subprocess.PIPE, check=False)
    if result.returncode != 0:
        raise Failure("git %s failed: %s" % (arguments[0], result.stderr.decode(errors="replace").strip()))
    return result.stdout


def holds_commit(repository: Path) -> bool:
    """Whether the git repository at `repository` holds the pinned commit of the server."""
    if not (repository / ".git").exists():
        return False
    try:
        git(["cat-file", "-e", SERVER_COMMIT + "^{commit}"], repository)
    except Failure:
        return False
    return True


def fetch_server(cache: Path, given) -> Path:
    """The server repository at the pinned commit: the checkout `given`, or a clone in `cache`."""
    if given is not None:
        repository = given.resolve()
        if not holds_commit(repository):
            raise Failure("%s holds no commit %s of the server" % (repository, SERVER_COMMIT))
        return repository
    repository = cache / SERVER_DIRECTORY
    if holds_commit(repository):
        return repository
    shutil.rmtree(repository, ignore_errors=True)
    repository.mkdir(parents=True)
    print("fetch: %s at %s" % (SERVER_REPOSITORY, SERVER_COMMIT), flush=True)
    git(["init", "--quiet"], repository)
    git(["fetch", "--quiet", "--depth", "1", "--no-tags", SERVER_REPOSITORY, SERVER_COMMIT], repository)
    git(["checkout", "--quiet", "--detach", "FETCH_HEAD"], repository)
    return repository


def server_files(index: dict) -> tuple:
    """The entries of `index` that the server requires, and the names of those it leaves out."""
    if index.get("formatVersion") != 1 or index.get("game") != "minecraft":
        raise Failure("modrinth.index.json is no index of a Minecraft pack in format 1")
    pins = index.get("dependencies", {})
    if "%s-%s" % (pins.get("minecraft"), pins.get("forge")) != FORGE_VERSION:
        raise Failure("the pack names Forge %s, the pin is %s" % (pins.get("forge"), FORGE_VERSION))
    required, left_out = [], []
    for entry in index["files"]:
        path = PurePosixPath(entry["path"])
        name = path.name
        if path.parts != ("mods", name) or name.startswith(".") or "\\" in name or not name.endswith(".jar"):
            raise Failure("the index names the file %r, which is no jar in mods/" % entry["path"])
        if entry.get("env", {}).get("server") != "required":
            left_out.append(name)
            continue
        url = entry["downloads"][0]
        parts = urllib.parse.urlsplit(url)
        if parts.scheme != "https" or parts.hostname not in MODPACK_HOSTS:
            raise Failure("the index downloads %s from %s, which is none of %s" % (name, parts.hostname, MODPACK_HOSTS))
        required.append({"file": name, "url": url, "sha512": entry["hashes"]["sha512"], "size": entry["fileSize"]})
    return required, left_out


def continues(response, offset: int) -> bool:
    """Whether `response` answers a range request with the bytes from `offset` on."""
    if response.status != 206:
        return False
    found = re.fullmatch(r"bytes (\d+)-\d+/(\d+|\*)", (response.headers.get("Content-Range") or "").strip())
    return found is not None and int(found[1]) == offset


def fetch_jar(entry: dict, mods: Path) -> bool:
    """Download one jar of the index into `mods`, answering whether it had to be downloaded.

    A jar whose size and SHA-512 match stays. A partial download goes on with a range request
    where it stopped, but only when the server answers with the bytes from that point on, and
    starts over from the first byte when it answers with anything else. A download that went on
    from a partial one and misses the hash held wrong bytes before that point, so the next try
    starts over from the first byte. Only a download from the first byte that misses fails at
    once.
    """
    target = mods / entry["file"]
    if target.is_file() and target.stat().st_size == entry["size"] and digest(target, "sha512") == entry["sha512"]:
        return False
    partial = target.with_name(target.name + ".part")
    reason = "broke off"
    for attempt in range(1, MODPACK_TRIES + 1):
        offset = partial.stat().st_size if partial.is_file() else 0
        if offset >= entry["size"]:
            partial.unlink()
            offset = 0
        headers = {"User-Agent": "golem-harness"}
        if offset:
            headers["Range"] = "bytes=%d-" % offset
        print("fetch: %s%s" % (entry["url"], " from byte %d" % offset if offset else ""), flush=True)
        try:
            request = urllib.request.Request(entry["url"], headers=headers)
            with urllib.request.urlopen(request, timeout=300) as response:
                resumed = offset > 0 and continues(response, offset)
                with open(partial, "ab" if resumed else "wb") as stream:
                    shutil.copyfileobj(response, stream)
        except (OSError, http.client.HTTPException) as broken:
            if attempt == MODPACK_TRIES:
                raise Failure("%s: %s" % (entry["url"], broken)) from broken
            reason = "broke off"
            continue
        if partial.stat().st_size < entry["size"]:
            reason = "broke off"
            continue
        found = digest(partial, "sha512")
        if partial.stat().st_size == entry["size"] and found == entry["sha512"]:
            partial.replace(target)
            return True
        partial.unlink()
        if not offset:
            raise Failure("%s has SHA-512 %s, the index names %s" % (entry["url"], found, entry["sha512"]))
        reason = "went on from byte %d and missed the SHA-512" % offset
    raise Failure("%s: all %d tries failed, the last %s" % (entry["url"], MODPACK_TRIES, reason))


def replace(source: Path, target: Path) -> None:
    """Put `source` at `target`, removing what stood there before."""
    if target.is_dir() and not target.is_symlink():
        shutil.rmtree(target)
    elif target.exists() or target.is_symlink():
        target.unlink()
    target.parent.mkdir(parents=True, exist_ok=True)
    source.rename(target)


def fetch_modpack(cache: Path, given) -> Path:
    """The pack of the live server under `cache`, a server directory but for Forge itself."""
    repository = fetch_server(cache, given)
    pack = cache / MODPACK_DIRECTORY
    stamp = pack / "modpack.json"
    if stamp.exists():
        stamp.unlink()
    index = json.loads(git(["show", SERVER_COMMIT + ":modrinth.index.json"], repository))
    required, left_out = server_files(index)
    tree = cache / "unpack-modpack"
    shutil.rmtree(tree, ignore_errors=True)
    tree.mkdir(parents=True)
    archive = cache / "modpack-tree.tar"
    with open(archive, "wb") as stream:
        git(["archive", "--format=tar", SERVER_COMMIT, "--", "local-mods", *MODPACK_TREES, *MODPACK_FILES],
            repository, output=stream)
    with tarfile.open(archive) as bundle:
        bundle.extractall(tree, filter="data")
    archive.unlink()
    if digest(tree / ARCLIGHT_JAR, "sha256") != ARCLIGHT_SHA256:
        raise Failure("%s of the commit differs from its pin %s" % (ARCLIGHT_JAR, ARCLIGHT_SHA256))
    local = sorted(path for path in (tree / "local-mods").iterdir() if path.suffix == ".jar")
    names = [entry["file"] for entry in required]
    clash = sorted(set(names) & {path.name for path in local})
    if clash or len(set(names)) != len(names):
        raise Failure("the index and local-mods/ name some jars twice: %s" % clash)
    mods = pack / "mods"
    mods.mkdir(parents=True, exist_ok=True)
    # A jar of an earlier pin would load beside the new one, so the folder holds the pack alone.
    wanted = set(names) | {path.name for path in local} | {name + ".part" for name in names}
    for stale in mods.iterdir():
        if stale.name not in wanted:
            if stale.is_dir() and not stale.is_symlink():
                shutil.rmtree(stale)
            else:
                stale.unlink()
    with ThreadPoolExecutor(MODPACK_WORKERS) as workers:
        downloaded = sum(workers.map(lambda entry: fetch_jar(entry, mods), required))
    for path in local:
        shutil.copyfile(path, mods / path.name)
    for part in (*MODPACK_TREES, *MODPACK_FILES):
        replace(tree / part, pack / part)
    shutil.rmtree(tree)
    jars = [{"file": entry["file"], "source": "modrinth.index.json", "sha512": entry["sha512"], "size": entry["size"]}
            for entry in required]
    jars += [{"file": path.name, "source": "local-mods", "sha512": digest(mods / path.name, "sha512"),
              "size": (mods / path.name).stat().st_size} for path in local]
    record = {
        "schema": "golem-modpack-1",
        "repository": SERVER_REPOSITORY,
        "commit": SERVER_COMMIT,
        "pack": {"name": index.get("name"), "version": index.get("versionId"), **index["dependencies"]},
        "mods": sorted(jars, key=lambda jar: jar["file"].lower()),
        "left_out": sorted(left_out, key=str.lower),
    }
    stamp.write_text(json.dumps(record, indent=1) + "\n", encoding="utf-8")
    print("fetch: %d jars of the index, %d of them downloaded now, %d of local-mods, %d left out as not for the "
          "server" % (len(required), downloaded, len(local), len(left_out)))
    return pack


def shown(path: Path) -> str:
    """A path relative to the root where it lies below it, so no machine path is printed."""
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return path.name


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description="Download the JDK and the Forge server of the notebooks.")
    parser.add_argument("--only", choices=["jdk", "forge", "gradle", "modpack"],
                        help="Fetch one of the three alone, or the modpack, which only this fetches.")
    parser.add_argument("--cache", type=Path, default=CACHE, help="The directory, by default tests/.cache.")
    parser.add_argument("--server-repo", type=Path,
                        help="A checkout of the server repository that holds the pinned commit, read and not cloned.")
    args = parser.parse_args(argv)
    if args.server_repo is not None and args.only != "modpack":
        parser.error("--server-repo belongs to --only modpack")
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
        if args.only == "modpack":
            pack = fetch_modpack(cache, args.server_repo)
            print("fetch: the modpack of %s at %s" % (SERVER_COMMIT[:12], shown(pack)))
    except (Failure, OSError) as broken:
        print("fetch: %s" % broken, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
