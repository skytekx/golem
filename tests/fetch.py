# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
"""Golem harness: download the JDK, the Forge server and Gradle that the notebooks run against.

    python3 -I tests/fetch.py
    python3 -I tests/fetch.py --only jdk
    python3 -I tests/fetch.py --cache path/to/cache
    python3 -I tests/fetch.py --only modpack
    python3 -I tests/fetch.py --only modpack --server-repo path/to/skytekx3-1201-server

Every download is checked by its SHA-256, or a jar of the pack by the SHA-512 of its index, before
it is used, and lands under tests/.cache/, which git ignores. The pins are these.

1. Temurin 17.0.21+9 of Eclipse Adoptium, the JDK the Mac mini runs the server on, unpacked to
   tests/.cache/jdk-17.0.21+9. On macOS the Java home is the Contents/Home of the bundle, which
   the script moves to that name, without the rest of the bundle.
2. The installer of Forge 1.20.1-47.4.18 from the Forge maven, which installs the server into
   tests/.cache/forge-server. It downloads the server jar of Minecraft from Mojang and the
   libraries, as the installer does for every Forge server.
3. Gradle 8.14.3, which builds the bridge under bridge/ with ForgeGradle, unpacked to
   tests/.cache/gradle-8.14.3. The bridge carries no Gradle wrapper, whose jar is a binary under
   another license.
4. The SkyTekx 3 modpack as the live server runs it, from the repository of the server at one
   commit, only with --only modpack, since it is about 1 GB. The script fetches that commit into
   tests/.cache/skytekx3-1201-server without a work tree, or reads it from the checkout that
   --server-repo names and leaves alone. Every file comes from the commit itself, not from the work
   tree of a checkout. It downloads the files of modrinth.index.json that the server requires
   into tests/.cache/modpack/mods, each checked by the SHA-512 and the size the index gives,
   from cdn.modrinth.com and github.com, which may redirect over https to the host of its
   release assets alone, and reads no more of an answer than the size allows. It adds the jars
   of local-mods/, and copies config/, defaultconfigs/, kubejs/, world/datapacks/,
   server.properties.example, the systemd unit of the live server, the Arclight jar with its
   four files of settings and tools/patch_mods.py beside them. Into libraries/ it downloads the
   libraries that the installer.json of the Arclight jar names, from Maven Central and the
   repository of Sponge, each pinned by its SHA-256 and checked against the SHA-1 of
   installer.json, so Arclight downloads nothing at its start. The jars stay as their authors
   published them, since tools/patch_mods.py rewrites 14 of them, which would then miss their
   hashes, so a notebook patches its own copy as the live server patches its mods/. The file
   tests/.cache/modpack/modpack.json, written last, names the commit, every jar and the Arclight
   jar with its SHA-256 and its libraries. The pack holds what the pin names alone, so a file of
   an earlier pin is removed. A download that broke off goes on where it stopped, and a jar whose
   hash already matches stays.

The script accepts the EULA of Minecraft in that test server, with eula=true in its eula.txt,
on behalf of the owner of SkyTekx, who runs the live server under it. The test server binds
localhost only and runs in offline mode, so no account of Mojang is involved.

A pin that is already in place is kept, so a second run downloads nothing. The JDK, the Forge
server and Gradle are installed into a folder beside their place, which takes the name of the pin
once the install is complete, so a run that failed or broke off leaves nothing that a later run
takes for a pin in place. The exit status is 1 when a download or an install fails or a hash
differs.

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
import time
import unicodedata
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
#: The hosts that the first download of an entry of the index may name. The format of Modrinth packs
#: allows more, so this is the own list of Golem, the two hosts of the pinned index, and a new pin
#: that names another host is a change to look at.
MODPACK_HOSTS = ("cdn.modrinth.com", "github.com")
#: The hosts that a download of the pack may be redirected to, over https alone: those above and the
#: host of the release assets, where github.com sends the download of a release. The SHA-512 of the
#: index guards the bytes wherever they come from.
MODPACK_REDIRECT_HOSTS = (*MODPACK_HOSTS, "release-assets.githubusercontent.com")
#: The directories of the server repository that the pack takes beside its mods.
MODPACK_TREES = ("config", "defaultconfigs", "kubejs", "world/datapacks")
#: The jar of Arclight that the live server runs, which arclight.jar of the repository links to.
ARCLIGHT_JAR = "arclight-forge-1.20.1-1.0.6-6de9fec-safeiter2.jar"
ARCLIGHT_SHA256 = "c55ad477b92250ef31baf965aae3158ffa99239b9b44a60b3a9dd5206dc1557f"
#: The libraries that the installer.json of that Arclight jar names. Arclight downloads them at its
#: first start from mirrors of its own and checks them by SHA-1 alone, so the script fetches them
#: into libraries/ of the pack, from Maven Central and the two of Configurate from the repository
#: of Sponge, which alone holds them, each pinned by its SHA-256 and size and checked against the
#: SHA-1 of installer.json. A copy that holds them makes Arclight download nothing.
MAVEN_CENTRAL = "https://repo.maven.apache.org/maven2/"
SPONGE = "https://repo.spongepowered.org/maven/"
ARCLIGHT_LIBRARIES = {
    "org.yaml:snakeyaml:2.0":
        (MAVEN_CENTRAL, "880c9d896e4b74a06c549c15ca496450165d6909fa15d7e662bee8f6a66d7afa", 334803),
    "org.xerial:sqlite-jdbc:3.42.0.0":
        (MAVEN_CENTRAL, "53174d76087bb73cc29db9c02766fb921fd7fc652f7952f3609e0018e3dd5ded", 13030515),
    "com.mysql:mysql-connector-j:8.0.33":
        (MAVEN_CENTRAL, "e2a3b2fc726a1ac64e998585db86b30fa8bf3f706195b78bb77c5f99bf877bd9", 2481560),
    "com.googlecode.json-simple:json-simple:1.1.1":
        (MAVEN_CENTRAL, "4e69696892b88b41c55d49ab2fdcc21eead92bf54acc588c0050596c3b75199c", 23931),
    "org.apache.logging.log4j:log4j-jul:2.17.2":
        (MAVEN_CENTRAL, "cec8d204efa87dfc759faf43d6affde1d18fa25b7c6d98ea5ef0737a56ff195c", 30948),
    "net.md-5:SpecialSource:1.11.0":
        (MAVEN_CENTRAL, "a47cd9a3db46a0cc3b769bd19a2a91d4ad0e9d019153d186f099bbaf5db16445", 88737),
    "org.jline:jline-terminal-jansi:3.12.1":
        (MAVEN_CENTRAL, "5aefd139ebfd05fe7792bab8a0dc6d551d0e05ff0d6972d83fbdf4a4ad256cb3", 39048),
    "org.fusesource.jansi:jansi:1.18":
        (MAVEN_CENTRAL, "109e64fc65767c7a1a3bd654709d76f107b0a3b39db32cbf11139e13a6f5229b", 287352),
    "jline:jline:2.12.1":
        (MAVEN_CENTRAL, "c31f1dcf2cef46f8a6f8aea8ae5c295cc96910e8f8ea4c40291695d64d2e940f", 213911),
    "org.apache.maven:maven-resolver-provider:3.8.5":
        (MAVEN_CENTRAL, "4b9f827efb393696431978dc0354651f1a95d4c52ba07c92d830efe5dc9b1012", 67173),
    "org.apache.maven.resolver:maven-resolver-connector-basic:1.7.3":
        (MAVEN_CENTRAL, "29beb02541adb82873d7bdeed2aaea03eaf3123720dbd1b5d96d058849364c66", 45777),
    "org.apache.maven.resolver:maven-resolver-transport-http:1.7.3":
        (MAVEN_CENTRAL, "558cbdf4d76f7cf3699b59fb22fd65614870e19b42983585b42ce84f336b348f", 42580),
    "org.apache.maven:maven-model:3.8.5":
        (MAVEN_CENTRAL, "7c9eeb30ac1941ea9bdf76c08c162ed98c005913e5bff66de3f1027a304546dc", 215280),
    "org.codehaus.plexus:plexus-utils:3.3.0":
        (MAVEN_CENTRAL, "76d174792540e2775af94d03d10fb2d3c776e2cd0ac0ebf427d3e570072bb9ce", 263253),
    "org.apache.maven:maven-model-builder:3.8.5":
        (MAVEN_CENTRAL, "6bcd688b4700bc0d9046c531c316e75674f53b345d4432d6a7f62812b2424bef", 195825),
    "org.codehaus.plexus:plexus-interpolation:1.26":
        (MAVEN_CENTRAL, "b3b5412ce17889103ea564bcdfcf9fb3dfa540344ffeac6b538a73c9d7182662", 85348),
    "org.eclipse.sisu:org.eclipse.sisu.inject:0.3.4":
        (MAVEN_CENTRAL, "8c0e6aa7f35593016f2c5e78b604b57f023cdaca3561fe2fe36f2b5dbbae1d16", 379197),
    "org.apache.maven:maven-builder-support:3.8.5":
        (MAVEN_CENTRAL, "a205eaa76609ae0039bf38fe9a9c024c646eded05d176effbef524f04a2c2761", 14298),
    "org.apache.maven:maven-repository-metadata:3.8.5":
        (MAVEN_CENTRAL, "24d89a1b23dc0427ac67895a741802fd23897bdf135d87fe586d718b38a3e924", 28416),
    "org.apache.maven.resolver:maven-resolver-api:1.6.3":
        (MAVEN_CENTRAL, "d0b28ed944058ba4f9be4b54c25d6d5269cc4f3f3c49aa450d4dc2f7e0d552f6", 149251),
    "org.apache.maven.resolver:maven-resolver-spi:1.6.3":
        (MAVEN_CENTRAL, "17441a39045ac19bc4a8068fb7284facebf6337754bf2bf8f26a76b5f98ed108", 38411),
    "org.apache.maven.resolver:maven-resolver-util:1.6.3":
        (MAVEN_CENTRAL, "cdcad9355b625743f40e4cead9a96353404e010c39c808d23b044be331afa251", 168856),
    "org.apache.maven.resolver:maven-resolver-impl:1.6.3":
        (MAVEN_CENTRAL, "17aaebe6e3e59df8cb5b4ec210196f7084637312b9bc4ff14cb77ad1ae3c381b", 179555),
    "org.apache.httpcomponents:httpclient:4.5.13":
        (MAVEN_CENTRAL, "6fe9026a566c6a5001608cf3fc32196641f6c1e5e1986d1037ccdbd5f31ef743", 780321),
    "org.apache.httpcomponents:httpcore:4.4.14":
        (MAVEN_CENTRAL, "f956209e450cb1d0c51776dfbd23e53e9dd8db9a1298ed62b70bf0944ba63b28", 328436),
    "commons-codec:commons-codec:1.15":
        (MAVEN_CENTRAL, "b3e9f6d63a790109bf0d056611fbed1cf69055826defeb9894a71369d246ed63", 353793),
    "org.slf4j:jcl-over-slf4j:1.7.30":
        (MAVEN_CENTRAL, "71e9ee37b9e4eb7802a2acc5f41728a4cf3915e7483d798db3b4ff2ec8847c50", 16537),
    "org.spongepowered:configurate-hocon:3.6.1":
        (SPONGE, "3e6223540525fc9e783d48c897a3b2ac56f24d9990d0a56a46d5b962815f197b", 8072),
    "org.spongepowered:configurate-core:3.6.1":
        (SPONGE, "3bcf3d6e62cc24e1a8d8a963e2ef8ac98e886abc10b2d65846ddd3d20d3d497b", 134754),
    "com.typesafe:config:1.3.1":
        (MAVEN_CENTRAL, "e6fadfc6108220d3a6b86aa7e4e16c9e7bb857ba58955886308bb13972264af0", 282549),
    "javax.inject:javax.inject:1":
        (MAVEN_CENTRAL, "91c77044a50c481636c32d916fd89c9118a72195390452c81065080f957de7ff", 2497),
}
#: The files of the server repository that the pack takes: the settings of the live server and the
#: systemd unit with the options of its JVM, the jar of Arclight with the files of settings it
#: reads, and the script that patches jars of mods/ on the live server.
MODPACK_FILES = ("server.properties.example", "systemd/skytekx3.service", ARCLIGHT_JAR, "arclight.conf",
                 "bukkit.yml", "spigot.yml", "commands.yml", "tools/patch_mods.py")
#: The downloads that run at once, and the tries of one download before the script gives up, with a
#: pause that grows after each try that broke off, since Maven Central answers 429 to a burst.
MODPACK_WORKERS = 4
MODPACK_TRIES = 5

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
    arguments = Path("libraries") / "net" / "minecraftforge" / "forge" / FORGE_VERSION / "unix_args.txt"
    if not (server / arguments).is_file():
        installer = cache / ("forge-%s-installer.jar" % FORGE_VERSION)
        download(FORGE_INSTALLER, installer, FORGE_SHA256)
        # The installer writes unix_args.txt before its processors build the patched jars of the
        # server, so it installs into a folder beside, which takes the name of the server once the
        # installer succeeded. The paths of the install are relative, so it may move.
        staged = cache / ("install-" + FORGE_DIRECTORY)
        shutil.rmtree(staged, ignore_errors=True)
        staged.mkdir(parents=True)
        print("fetch: installing Forge %s" % FORGE_VERSION, flush=True)
        result = subprocess.run([str(java_home / "bin" / "java"), "-jar", str(installer), "--installServer", "."],
                                cwd=staged, capture_output=True, text=True, check=False)
        if result.returncode != 0 or not (staged / arguments).is_file():
            raise Failure("the Forge installer failed:\n%s" % result.stdout[-4000:])
        shutil.rmtree(server, ignore_errors=True)
        staged.rename(server)
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
    # The zip holds bin/gradle long before the last jar of lib/, so it unpacks into a folder beside,
    # whose Gradle takes its name once every file and mode is in place.
    unpacked = cache / ("unpack-" + GRADLE_DIRECTORY)
    shutil.rmtree(unpacked, ignore_errors=True)
    unpacked.mkdir(parents=True)
    with zipfile.ZipFile(archive) as bundle:
        bundle.extractall(unpacked)
    inner = unpacked / GRADLE_DIRECTORY
    # The zip format of the standard library keeps no file modes, so the scripts need theirs back.
    for script in (inner / "bin").iterdir():
        script.chmod(0o755)
    shutil.rmtree(home, ignore_errors=True)
    inner.rename(home)
    shutil.rmtree(unpacked, ignore_errors=True)
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
    # The script reads the commit through git show and git archive alone, so the clone needs no
    # work tree, and a branch keeps the commit through a gc.
    git(["update-ref", "refs/heads/pin", "FETCH_HEAD"], repository)
    return repository


#: A SHA-512 as the index writes it, in lower case hex.
SHA512 = re.compile(r"[0-9a-f]{128}")
#: What env.server of an entry may say, as the format of Modrinth packs defines it.
SERVER_ENV = ("required", "optional", "unsupported")


def odd_character(text: str) -> bool:
    """Whether `text` holds a control or format character, which no name of a jar needs."""
    return any(unicodedata.category(char) in ("Cc", "Cf") for char in text)


def folded(name: str) -> str:
    """`name` as a file system that ignores case and the form of Unicode compares it."""
    return unicodedata.normalize("NFC", name).casefold()


def server_files(index) -> tuple:
    """The entries of `index` that the server requires, and the names of those it leaves out.

    Every entry must name a jar in mods/ without a control character, an env that the format
    knows, a list of downloads, a SHA-512 in lower case hex and a positive size, or the index is
    refused with a Failure that names the entry.
    """
    if not isinstance(index, dict) or index.get("formatVersion") != 1 or index.get("game") != "minecraft":
        raise Failure("modrinth.index.json is no index of a Minecraft pack in format 1")
    pins = index.get("dependencies")
    pins = pins if isinstance(pins, dict) else {}
    if "%s-%s" % (pins.get("minecraft"), pins.get("forge")) != FORGE_VERSION:
        raise Failure("the pack names Forge %s, the pin is %s" % (pins.get("forge"), FORGE_VERSION))
    if not isinstance(index.get("files"), list):
        raise Failure("modrinth.index.json holds no list of files")
    required, left_out = [], []
    for entry in index["files"]:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            raise Failure("the index holds the entry %r, which names no path" % (entry,))
        if odd_character(entry["path"]):
            raise Failure("the index names the file %r, which holds a control or format character" % entry["path"])
        path = PurePosixPath(entry["path"])
        name = path.name
        if path.parts != ("mods", name) or name.startswith(".") or "\\" in name or not name.endswith(".jar"):
            raise Failure("the index names the file %r, which is no jar in mods/" % entry["path"])
        env = entry.get("env", {})
        if not isinstance(env, dict) or env.get("server", "required") not in SERVER_ENV:
            raise Failure("the index gives %s the env %r, which the format does not know" % (name, env))
        downloads = entry.get("downloads")
        if not isinstance(downloads, list) or not downloads or not all(isinstance(url, str) for url in downloads):
            raise Failure("the index names %s without a list of downloads" % name)
        hashes = entry.get("hashes")
        sha512 = hashes.get("sha512") if isinstance(hashes, dict) else None
        if not isinstance(sha512, str) or not SHA512.fullmatch(sha512):
            raise Failure("the index names %s without a SHA-512 of 128 lower case hex digits" % name)
        size = entry.get("fileSize")
        if not isinstance(size, int) or isinstance(size, bool) or size <= 0:
            raise Failure("the index names %s with the size %r, which is no positive number of bytes" % (name, size))
        # The format makes env optional, and a file without it belongs on both sides.
        if env.get("server", "required") != "required":
            left_out.append(name)
            continue
        url = downloads[0]
        parts = urllib.parse.urlsplit(url)
        if parts.scheme != "https" or parts.hostname not in MODPACK_HOSTS:
            raise Failure("the index downloads %s from %s, which is none of %s" % (name, parts.hostname, MODPACK_HOSTS))
        required.append({"file": name, "url": url, "sha512": sha512, "size": size})
    return required, left_out


class Redirects(urllib.request.HTTPRedirectHandler):
    """Follows a redirect of a download of the pack only over https to a host it may reach."""

    def redirect_request(self, request, stream, code, message, headers, url):
        parts = urllib.parse.urlsplit(url)
        if parts.scheme != "https" or parts.hostname not in MODPACK_REDIRECT_HOSTS:
            raise Failure("%s redirects to %s on %s, which is none of https on %s" % (
                request.full_url, parts.scheme, parts.hostname, ", ".join(MODPACK_REDIRECT_HOSTS)))
        return super().redirect_request(request, stream, code, message, headers, url)


#: The opener of the downloads of the pack, with the redirects above.
OPENER = urllib.request.build_opener(Redirects)


def copy_bounded(response, stream, limit: int) -> None:
    """Copy `response` into `stream`, but no more than `limit` bytes."""
    while limit > 0:
        block = response.read(min(1 << 20, limit))
        if not block:
            return
        stream.write(block)
        limit -= len(block)


def continues(response, offset: int) -> bool:
    """Whether `response` answers a range request with the bytes from `offset` on."""
    if response.status != 206:
        return False
    found = re.fullmatch(r"bytes (\d+)-\d+/(\d+|\*)", (response.headers.get("Content-Range") or "").strip())
    return found is not None and int(found[1]) == offset


#: The names of the hashes that a download may be checked by.
HASHES = {"sha256": "SHA-256", "sha512": "SHA-512"}


def fetch_jar(entry: dict, folder: Path) -> bool:
    """Download one jar into `folder`, answering whether it had to be downloaded.

    The entry names the file, a path below `folder`, its URL, its size and its SHA-512 or its
    SHA-256. A jar whose size and hash match stays. A partial download goes on with a range
    request where it stopped, but only when the server answers with the bytes from that point
    on, and starts over from the first byte when it answers with anything else. A download that
    went on from a partial one and misses the hash held wrong bytes before that point, so the
    next try starts over from the first byte. Only a download from the first byte that misses
    fails at once.
    """
    algorithm = next(name for name in HASHES if name in entry)
    target = folder / entry["file"]
    if target.is_file() and target.stat().st_size == entry["size"] and digest(target, algorithm) == entry[algorithm]:
        return False
    target.parent.mkdir(parents=True, exist_ok=True)
    partial = target.with_name(target.name + ".part")
    reason = "broke off"
    for attempt in range(1, MODPACK_TRIES + 1):
        if attempt > 1 and reason == "broke off":
            time.sleep(min(2 ** (attempt - 2), 8))
        offset = partial.stat().st_size if partial.is_file() else 0
        if offset >= entry["size"]:
            partial.unlink(missing_ok=True)
            offset = 0
        headers = {"User-Agent": "golem-harness"}
        if offset:
            headers["Range"] = "bytes=%d-" % offset
        print("fetch: %s%s" % (entry["url"], " from byte %d" % offset if offset else ""), flush=True)
        try:
            request = urllib.request.Request(entry["url"], headers=headers)
            with OPENER.open(request, timeout=300) as response:
                resumed = offset > 0 and continues(response, offset)
                # One byte more than the jar can hold shows an answer that is too large.
                with open(partial, "ab" if resumed else "wb") as stream:
                    copy_bounded(response, stream, entry["size"] - (offset if resumed else 0) + 1)
        except (OSError, http.client.HTTPException) as broken:
            if attempt == MODPACK_TRIES:
                raise Failure("%s: %s" % (entry["url"], broken)) from broken
            reason = "broke off"
            continue
        size = partial.stat().st_size
        if size < entry["size"]:
            reason = "broke off"
            continue
        if size == entry["size"]:
            found = digest(partial, algorithm)
            if found == entry[algorithm]:
                partial.replace(target)
                return True
            problem = "has %s %s, the pin is %s" % (HASHES[algorithm], found, entry[algorithm])
        else:
            problem = "is larger than the %d bytes of its pin" % entry["size"]
        partial.unlink()
        if not offset:
            raise Failure("%s %s" % (entry["url"], problem))
        reason = "went on from byte %d and %s" % (offset, problem)
    raise Failure("%s: all %d tries failed, the last %s" % (entry["url"], MODPACK_TRIES, reason))


def arclight_libraries(named) -> list:
    """The downloads of the libraries that the installer.json of Arclight names, by their pins."""
    if not isinstance(named, dict) or set(named) != set(ARCLIGHT_LIBRARIES):
        others = sorted(set(named) ^ set(ARCLIGHT_LIBRARIES)) if isinstance(named, dict) else named
        raise Failure("the installer.json of %s and the pins name other libraries: %s" % (ARCLIGHT_JAR, others))
    entries = []
    for coordinate, sha1 in sorted(named.items()):
        group, artifact, version = coordinate.split(":")
        path = "%s/%s/%s/%s-%s.jar" % (group.replace(".", "/"), artifact, version, artifact, version)
        base, sha256, size = ARCLIGHT_LIBRARIES[coordinate]
        entries.append({"coordinate": coordinate, "file": path, "url": base + path, "sha256": sha256, "size": size,
                        "sha1": sha1})
    return entries


def replace(source: Path, target: Path) -> None:
    """Put `source` at `target`, removing what stood there before."""
    if target.is_dir() and not target.is_symlink():
        shutil.rmtree(target)
    elif target.exists() or target.is_symlink():
        target.unlink()
    target.parent.mkdir(parents=True, exist_ok=True)
    source.rename(target)


def prune(folder: Path, wanted: set, prefix: PurePosixPath = PurePosixPath()) -> None:
    """Remove from `folder` whatever is neither a path of `wanted` nor a folder on the way to one.

    The paths of `wanted` are relative to the folder the first call names.
    """
    for path in folder.iterdir():
        relative = prefix / path.name
        if str(relative) in wanted:
            continue
        directory = path.is_dir() and not path.is_symlink()
        if directory and any(PurePosixPath(name).is_relative_to(relative) for name in wanted):
            prune(path, wanted, relative)
        elif directory:
            shutil.rmtree(path)
        else:
            path.unlink()


def fetch_modpack(cache: Path, given) -> Path:
    """The pack of the live server under `cache`, a server directory but for Forge itself."""
    repository = fetch_server(cache, given)
    pack = cache / MODPACK_DIRECTORY
    stamp = pack / "modpack.json"
    if stamp.exists():
        stamp.unlink()
    try:
        index = json.loads(git(["show", SERVER_COMMIT + ":modrinth.index.json"], repository))
    except ValueError as broken:
        raise Failure("modrinth.index.json of %s is no JSON: %s" % (SERVER_COMMIT[:12], broken)) from broken
    required, left_out = server_files(index)
    tree = cache / "unpack-modpack"
    shutil.rmtree(tree, ignore_errors=True)
    tree.mkdir(parents=True)
    archive = cache / "modpack-tree.tar"
    with open(archive, "wb") as stream:
        git(["archive", "--format=tar", SERVER_COMMIT, "--", "local-mods", *MODPACK_TREES, *MODPACK_FILES],
            repository, output=stream)
    try:
        with tarfile.open(archive) as bundle:
            bundle.extractall(tree, filter="data")
    except tarfile.TarError as broken:
        raise Failure("the files of %s do not unpack as plain files: %s" % (SERVER_COMMIT[:12], broken)) from broken
    archive.unlink()
    if digest(tree / ARCLIGHT_JAR, "sha256") != ARCLIGHT_SHA256:
        raise Failure("%s of the commit differs from its pin %s" % (ARCLIGHT_JAR, ARCLIGHT_SHA256))
    with zipfile.ZipFile(tree / ARCLIGHT_JAR) as jar:
        try:
            installer = json.loads(jar.read("META-INF/installer.json"))
        except (KeyError, ValueError) as broken:
            raise Failure("%s holds no installer.json to read: %s" % (ARCLIGHT_JAR, broken)) from broken
    libraries = arclight_libraries(installer.get("libraries") if isinstance(installer, dict) else None)
    local = sorted(path for path in (tree / "local-mods").iterdir() if path.suffix == ".jar")
    names = [entry["file"] for entry in required]
    # Two names that differ in case or in the form of Unicode alone are one file on the disk of a
    # Mac, so they count as the same jar.
    groups = {}
    for name in names + [path.name for path in local]:
        groups.setdefault(folded(name), []).append(name)
    twice = sorted(group for group in groups.values() if len(group) > 1)
    if twice:
        raise Failure("the index and local-mods/ name some jars twice: %s" % ", ".join(" and ".join(group) for group in twice))
    mods = pack / "mods"
    mods.mkdir(parents=True, exist_ok=True)
    # A file of an earlier pin, such as the Arclight jar it named, would stay beside the new ones,
    # so the pack holds what this pin names alone.
    prune(pack, {"mods", "libraries", "modpack.json", *MODPACK_TREES, *MODPACK_FILES})
    (pack / "libraries").mkdir(exist_ok=True)
    prune(pack / "libraries", {name for entry in libraries for name in (entry["file"], entry["file"] + ".part")})
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
        downloaded += sum(workers.map(lambda entry: fetch_jar(entry, pack / "libraries"), libraries))
    for entry in libraries:
        found = digest(pack / "libraries" / entry["file"], "sha1")
        if found != entry["sha1"]:
            raise Failure("%s has SHA-1 %s, but installer.json of Arclight names %s, so Arclight would download it "
                          "again" % (entry["file"], found, entry["sha1"]))
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
        "schema": "golem-modpack-2",
        "repository": SERVER_REPOSITORY,
        "commit": SERVER_COMMIT,
        "pack": {"name": index.get("name"), "version": index.get("versionId"), **index["dependencies"]},
        "arclight": {"file": ARCLIGHT_JAR, "sha256": ARCLIGHT_SHA256,
                     "libraries": [{key: entry[key] for key in ("coordinate", "file", "sha256", "sha1")}
                                   for entry in libraries]},
        "mods": sorted(jars, key=lambda jar: jar["file"].lower()),
        "left_out": sorted(left_out, key=str.lower),
    }
    stamp.write_text(json.dumps(record, indent=1) + "\n", encoding="utf-8")
    print("fetch: %d jars of the index and %d libraries of Arclight, %d of them downloaded now, %d of local-mods, "
          "%d left out as not for the server" % (len(required), len(libraries), downloaded, len(local), len(left_out)))
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
    except (Failure, OSError, ValueError, tarfile.TarError) as broken:
        print("fetch: %s" % broken, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
