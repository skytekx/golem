---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Log
title: Change log
description: Chronological change log of Golem from the plan of 2026-11-06 on.
tags: [log, history]
timestamp: 2026-11-07
---

# Change log

Oldest first. Times are UTC.

## 2026-11-06

- The owner set the plan of Golem and the repository `skytekx/golem`. See [Plan](plan.md).
- The license went in: the AGPL 3 only, every file with its SPDX lines, the holder Florian Kleber,
  REUSE, and the license lint with its notebook `01_license_lint`.
- The commit rules went in: Commitizen in `.cz.toml`, the commit lint for the rules of the kernel,
  the hooks under `tools/hooks/`, the notebook `02_commit_lint` and CONTRIBUTING.md. The history
  of the branch was rewritten into that form.
- A spike booted Forge 47.4.18 on Temurin 17.0.21 through the JNI invocation API inside a Rust
  process in the cloud container of the session. The server logged `Done` after 26 s by its own
  count, and the spike stopped it through the server thread and saw `DestroyJavaVM` return 0. A
  second spike proved that a Forge mod calls a native method of a class on the module path that the
  host registered. See [Bridge](bridge.md).
- The Cargo workspace of the first milestone went in, with the crates `golem`, `golem-jvm`,
  `golem-bridge`, `golem-plugin`, `golem-permissions` and `golem-devtool`.
- Four agents built the parts of the first milestone in parallel, each in a work tree of its own,
  and their commits went onto the branch one part after the other:
  - The JVM host: `golem-jvm` reads the argument files and the launch of a Forge server as the java
    launcher of JDK 17 does and runs it in a JVM on the thread `golem-jvm`, and `golem run` boots
    the server of a directory with `-Xrs` and the host jar. Notebook `03_launch_args`. See
    [JVM host](jvm-host.md).
  - The natives and the plugins: `golem-bridge` answers the 14 natives and registers them,
    `golem-plugin` runs the plugins with budgets, and the core plugin answers `golem`. Notebook
    `04_plugins`. See [Plugins](plugins/index.md) and [Core plugin](plugins/core.md).
  - The Java bridge: the Gradle build of `golem-host.jar` and of the Forge mod, which stays off
    without Golem. Notebook `05_bridge_build`, which found that Forge 47 calls `chat()` on a worker
    thread and `commandNames()` on the main thread at startup. See [Bridge](bridge.md).
  - The permissions: `golem-permissions` with its strict format and its rules, and the seed of
    SkyTekx in `config/permissions.toml`. Notebook `06_permissions`. See
    [Permissions](plugins/permissions.md).
- The merge showed that a chat message on the worker thread passed every plugin while a tick held
  the lock of the handler. The natives now wait for a call of another thread and refuse only a
  call from inside the handler, and `04_plugins` holds both.
- A boot without the bridge mod showed that no signal stopped the server. A signal without a
  connected bridge now ends the JVM through `System.exit` and the shutdown hook of the server, as
  the java launcher does.
- `07_forge_boot` boots Forge under `golem run` with the real natives, a player of the harness and
  both stop paths. It passed on the first run.
- The commit lint now refuses trailers outside the last paragraph and looks up the commit that a
  Fixes trailer names, after a Fixes hash that named no commit went out. The history was rewritten
  again so that every trailer stands in the last block.
- CI builds and lints the crates, runs cargo deny and runs all seven notebooks, with the downloads
  of `tests/fetch.py` and caches for cargo, the downloads and Gradle. The first green run took six
  minutes for the notebooks.
- The issues [#1](https://github.com/skytekx/golem/issues/1) to
  [#7](https://github.com/skytekx/golem/issues/7) record the quirks that the first milestone fixes:
  three of the permissions, the stop path of Arclight and three of the java launcher.
- An adversarial review of the merged milestone, six reviewers with a skeptic each, confirmed 20
  findings and refuted 4. Each confirmed one is fixed with a notebook case where it had a behaviour:
  the lock of the handler, `SIGSEGV` and `SIGBUS` chained to the handler of Rust, `SIGQUIT` that
  ended the server, `SIGHUP` under nohup, the answers of plugin commands that `sendCommandFeedback`
  hid, `-J-server` and a source file in upper case, a merge commit at the edge of a shallow clone,
  the target directory of cargo, the include of macOS, the missing steps of CI and the pages that
  said what was no longer true.
- The owner ruled that a Golem plugin that ports a Bukkit plugin keeps the license of that plugin
  and names its holders. `ports.toml` names every port with its path, its upstream, its source, its
  license and the holders of the upstream, and rule 9 of the license lint holds every file under the
  path of a port to that license and to a copyright line of each holder beside the one of Florian
  Kleber, the `Cargo.toml` of the port to the license and NOTICE to a section on the upstream.
  Notebook `01_license_lint` holds a good port and one break of each part. The port of LuckPerms
  5.5.71, `golem-permissions`, went under the MIT license of LuckPerms. The crates
  `golem-essentials` and `golem-essentials-spawn` went in as the places of the ports of EssentialsX
  2.22.0 and EssentialsXSpawn 2.22.0, so that rule 9 holds every file of them from the first one on.
  They are under the GPL 3 or later, whose text `LICENSES/GPL-3.0-or-later.txt` is the LICENSE file
  of EssentialsX 2.22.0 byte for byte, and `deny.toml` lets the two crates carry that license.
  NOTICE has a section on LuckPerms, on EssentialsX and on EssentialsXSpawn and says how the ports
  combine with the AGPL 3 in the program golem. See [Decisions](decisions.md) and
  [Licensing](../Documentation/licensing.md#6-the-ports-and-what-golem-never-carries).
- Three parts went onto the branch `integrate/foundation`, each built by an agent of its own:
  - The check of the modpack: `tests/fetch.py --only modpack` builds the SkyTekx 3 modpack of the
    live server from a pinned commit of its repository, and `08_modpack_boot` boots it under java,
    golem and Arclight and then under golem on the world that Arclight wrote. Golem booted the 289
    mods as java did, so nothing of Golem needed a fix. See [Modpack](modpack.md).
  - The game access: the bridge passes the permission level of the source of a command, reports
    where a player died, teleports players and names them in the player list, answers the
    PermissionAPI of Forge through the handler `golem:permissions`, and lets the plugins ask the
    game on the server thread, with the natives `connect` and `permission`, 16 in all. The core
    plugin gives operators the tools of `/golem` that ask the game. See [Bridge](bridge.md) and
    [Core plugin](plugins/core.md).
  - The store: `golem-store` keeps the data of the plugins in one SQLite file, `golem-devtool
    store` drives it from a JSON script, and `09_store` holds it to its schema, its refusals and
    its crashes. See [Store](store.md).
- The owner ruled that plugins call the Golem API in process through the executor of Aosabi, and
  that Bukkit and Spigot plugins run unchanged through a Bukkit layer on the API. See
  [Decisions](decisions.md). A design panel on Opus and Fable wrote the plugin API and the format
  of its coverage check. See [Plugin API](api.md) and [API coverage](api-coverage.md).
- A survey of 503 plugins that run on Spigot 1.20.1 measured the three levels of the Bukkit layer,
  and `tools/plugin-survey/` rebuilds it. See [Bukkit layer](bukkit-layer.md).
- An adversarial review of the three parts, reviewers on Opus and on Fable and a judge for each
  part, confirmed 29 findings of the check of the modpack, 22 of the game access and 26 of the
  store, and refuted 4, 5 and 4.
- The fixes of the check of the modpack went in, each with its notebook case: the fetch starts a
  broken resume over, refuses a malformed index with one line, follows redirects only over https
  to known hosts, keeps only what the pin names and fetches the 31 libraries of Arclight, so its
  boot downloads nothing, and CI saves the downloads and the pack right after the fetch. Notebook
  08 asserts that java loads every jar, pins the lines of golem itself and its warnings on the
  world of Arclight, takes a fixed count of samples of the TPS, reads a decimal comma, gives the
  voice chat of each copy a free port, keeps the logs of a failed boot, scans the coremods and
  mixin configs, finds the native handlers of signals, fails every boot on a failure of the
  storage of the world and checks the stop of every boot by its saves.
- The issues [#99](https://github.com/skytekx/golem/issues/99) to
  [#104](https://github.com/skytekx/golem/issues/104) record the quirks that the check found: the
  two conflicts of mixins of Arclight with Apothic Attributes and Apotheosis, the libraries that
  Arclight downloads from a third party mirror, the pack of the server repository that differs
  from the mods of the live server, the save of Arclight that skips the tail of vanilla, and the
  key of TiCEX that Forge corrects at every start.

## 2026-11-07

- The fixes of the game access went in, each with its notebook case: the search for a safe place
  reads loaded chunks alone and keeps out portals, cobwebs, the bedrock roof of the Nether and the
  edge of the world border, a teleport runs as the command `tp` runs it, an infinite number
  travels as Java reads it, a call of another thread waits at most one second for the handler, a
  failing question prints its stack trace once, a command run as a player is judged by the source
  that ran it, the game answers nothing from `serverStopped` on, an operator level beyond 0 to 4
  reads as the nearest one, a player whose answer throws leaves the others, and the mod warns when
  Forge takes another permission handler. The core plugin says that the game gives no answer
  instead of refusing, and `/golem listname` keeps the text as the operator typed it. The
  scripted game of the devtool answers as the game of the mod. The pages name the respawn point
  as the point that the player holds, say that a permission answer holds in every context and
  record that Arclight hands every Boolean node of a mod to LuckPerms.
- The fixes of the store went in, each with its notebook case: the permissions and their
  generation are one read, a check copies the cache and queues its read under one lock, copies
  pointers instead of rows and sums the log of an account without an overflow, every close answers
  as the first, a second open in the same process is named, the open reads what a file is before
  it may write to it, a coordinate of -0.0 is 0.0, the changes that wait for the writer are
  bounded, a failed writer says which changes it loses and gives its reason to every later call,
  `PermissionSet::from_toml` refuses a key that `golem-permissions` refuses, and
  `libsqlite3-sys` is pinned exactly. The devtool refuses a weight that is no integer, stops
  without a panic when its output closes and closes a store once. The page states how the
  migration carries the balances and the names of EssentialsX and what a change costs the server
  thread, a burst that waits for the writer included.
- Open in the game access stays that `/golem status` does not name the permission handler that
  Forge took. See [Open items](open-items.md).
- The issues [#105](https://github.com/skytekx/golem/issues/105) to
  [#107](https://github.com/skytekx/golem/issues/107) record the quirks that the reviews of the
  game access and of the store found: Arclight hands every Boolean permission of a mod to
  LuckPerms, and EssentialsX keeps balances with fractions of a cent and below zero and folds the
  names of homes, warps and spawn groups, which the store holds otherwise. The issues #102 to #104
  now say what notebook 08 checks.
- The shared pages follow the merge: [Testing](../Documentation/testing.md) names what every
  notebook needs and asks, and the index, the open items and this log name the parts that are
  built.
- The spec of the port of EssentialsX 2.22.0 and EssentialsXSpawn 2.22.0 went into the wiki, under
  the GPL 3 or later of EssentialsX: [EssentialsX teleport port](plugins/essentials/teleport.md),
  [EssentialsX social port](plugins/essentials/social.md) and
  [EssentialsX bridge needs](plugins/essentials/bridge-needs.md), held against the bridge as it is
  after the fixes of the game access.
- The docs lint, `.github/scripts/docs_lint.py`, holds the wiki, `Documentation/`, CONTRIBUTING.md
  and README.md to the prose rules of CONTRIBUTING.md and checks that the index links every page
  and that [Testing](../Documentation/testing.md) names every notebook. CI runs it in the job of
  the licenses, and `10_docs_lint` holds it to each of its seven rules.
- A bug hunt of 2026-11-06 went through the tree in five areas: the Rust host, the bridge and the
  plugin runtime, the licensing and the permissions, the tooling with CI, the hooks and
  `tests/fetch.py`, and the docs. Finders on Opus and on Fable searched each area, skeptics of both
  models tried to refute each finding, and a judge on Opus ruled on each area. The judges confirmed
  66 findings and refuted 27, and 58 remained once the findings that named the same defect were
  merged. The conflict of rustup on `bin/cargo-fmt` that failed the notebooks job of CI joined them
  as the 59th. Fixers on Opus fixed each area on top of `b0f0d8a`, each fix with its notebook case
  where it has a behaviour, checks on Fable held the fixes of each area to their findings, and the
  points that the checks raised were fixed or ruled on before the merge. What stays open, among it
  the decisions that the fixers left to the owner, stands in [Open items](open-items.md). The fixes,
  by area:
  - The Rust host: a thread dump on `SIGQUIT` frees its local references of the JNI, and the signal
    thread attaches under its name `golem-signals`. `System.exit` runs on a thread of its own,
    `golem-exit`, so a dump and a later signal are still served while the shutdown hooks save the
    worlds. A signal queues the stop only while the server ticks, from its first tick until
    `ServerStoppedEvent`, and else ends the JVM through `System.exit` with 128 plus the signal, as
    java does, so a signal stops the server at every stage. A `SIGQUIT` of a tool whose attach
    socket is gone logs one line instead of a dump. golem reads its command line as strings of the
    system, so a command or an argument of `main` that is not UTF-8 is an error instead of a panic,
    refuses a host jar that is no file before the JVM starts, says that the JVM could not start when
    HotSpot aborts inside `JNI_CreateJavaVM`, and runs the Forge version that `run.sh` names when
    the server holds several. `build.rs` names HEAD only in the checkout of Golem itself, with
    `-dirty` when the files of the binary differ from it, and `/golem source` links the repository
    alone for such a commit. The page of the JVM host states the counts of notebook 03, whose
    verdict reads them off the page, and names an argument file that is not UTF-8 among the
    differences on purpose, and the answers of the agents on a signal, on `SIGQUIT` and on the
    commit of a build follow the page. Notebooks 03 and 07 hold each fix, and notebook 07 sees a
    stop that saves the worlds and holds a stop that waits on a hung server thread to java.
  - The bridge and the plugin runtime: a plugin command named like a command of the game or of a mod
    takes the plain name, and the command it replaces stays as `minecraft:<name>`, but for
    `setblock`, `summon`, `testforblock` and `tellraw`, which keep the command of the game as
    `commands.replace-commands` of Spigot does. The lines that the natives and the plugins log while
    a native runs reach `logs/latest.log` through the new method `log` of `Game`, with the logger
    `golem`. The runtime logs the panics of a plugin at powers of ten, behind a panic hook that
    stays silent while a call of a plugin runs. A command that Golem queues informs no operator, the
    MSPT holds the work of the bridge at the end of a tick one tick late, an address in a line of
    Golem opens on a click, and `respawn` says whether the player left the End, which changes the
    descriptor of the native and `Event::Respawn`. `/golem` names an unknown subcommand as it was
    typed, and the docs say what `golem source` prints and on which thread each native runs.
    Notebooks 04, 05 and 07 hold each fix, notebook 04 checks a line that the runtime writes,
    notebook 07 runs commands of golem in a function, and notebook 08 reads the lines of a clean
    boot where they stand now.
  - The licensing and the permissions: rule 9 of the license lint holds every port to files under
    its path, a `Cargo.toml` with the name of the port, the text of its license and a section of
    NOTICE with its Version, License and Files lines, and rule 6 holds NOTICE to its annotation in
    `REUSE.toml` with the precedence `override`. Outside git the lint skips what `.gitignore` names,
    and notebook 01 judges a copy of the tree. `LICENSES/MIT.txt` is the generic text that REUSE
    downloads, `ports.toml` pins LuckPerms 5.5.71 to the commit `8fa4c244927c` and names
    EssentialsXSpawn 2.22.0, and licensing.md and NOTICE describe the ports in every section that
    they change and say that golem links no port yet. The README, the plan, licensing.md and the
    page of the plugins name the GraphQL API on Aosabi, its subscriptions, the native code of world
    generation and the offer of the source in the server list as work to come, and section 7 of
    licensing.md says what cargo deny checks in CI. The group admin of the seed denies 22 of the
    nodes that EssentialsX marks `default: false`, which an operator of the live server never held,
    `Permissions::value` allocates nothing, which notebook 06 counts, and the page of the
    permissions says that a group without a weight shares the rank of `default`. Notebooks 01 and 06
    hold each fix, and notebook 09 now reads the group admin from the seed, as it reads the group
    `default`, so it holds the store to the seed with those denies.
  - The tooling: the commit lint judges a commit of a range as git stored it and the file of the
    hook as git will store it, with the comment character of git, its mode of cleanup and the cut
    line only where git cuts, and the hook leaves out a merge. The lint and Commitizen refuse the
    subjects `fixup!`, `squash!`, `amend!` and `Revert` that git writes, rule 4 spares only the
    trailers of the last paragraph, `Co-authored-by` passes as GitHub spells it, and so does a
    summary of one character. `cz bump` raises the major version for a breaking change, and
    `cz commit` takes the body and the trailers on several lines. The hook and CONTRIBUTING.md say
    that `git revert` and `git cherry-pick` skip the hook, and post-commit retries only a push that
    failed on the network and gives a push that a rule of origin refuses advice of its own. The job
    `commits` of CI takes its range from the event, so CI checks the commits of a push to main and
    no longer fails after a merge. Every job that runs cargo installs the toolchain of
    `rust-toolchain.toml` with its components in one step before its first cargo, so no proxy of
    rustup adds a component on the fly, where two of them raced on `bin/cargo-fmt` and failed runs
    47, 60 and 67, and the new notebook `11_toolchain` holds every such job to that step.
    `tests/fetch.py` installs Forge and Gradle beside their place and renames them once they are
    whole, its docstring says that it moves the Java home of macOS, and the comment of
    `tests/pyproject.toml` and the page on testing say that only the eight named packages are
    pinned. Notebooks 02, 08 and 11 hold each fix.
  - The docs: the README and the plan name the ports and their licenses where they state the license
    of Golem, and name the files whose tags stand in `REUSE.toml` or nowhere, and the plan and
    section 4 of CONTRIBUTING.md leave the place of the SPDX tags to licensing.md. The decisions say
    that the commit lint checks the form of a sign off and not whose it is, the pages of the bridge,
    the plugins and the permissions say that no plugin checks a node of `golem-permissions` yet, and
    section 9 of CONTRIBUTING.md names the two addresses of others that a license or a rule
    requires. Notebook 10 holds each of these pages to the tree.
- Notebook 08 judges the budget of a golem boot of the pack by the counts of `golem status`, after
  a single call of golem-core over the budget, which the machine of CI caused by taking the thread
  away, failed run 69. A panic still fails the boot at once.
- The work moves to another agent. `HANDOVER.md` names the rules of the owner, the state, the
  unfinished wiring of the store and the permissions and the unfinished gaps of the bridge, kept
  as series of patches in `handover/`, the items of Aosabi that Golem needs, the branch of Aosabi
  that adds an embedding entry, the feature list the owner asks for, the next steps and the
  decisions still open with their defaults.
