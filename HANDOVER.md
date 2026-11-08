<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->

# Hand over

This file hands Golem, and the work on Aosabi that Golem needs, to the next agent. It was written on
2026-11-07 at the pushed head `5fe5a38` of the branch `claude/aosabi-license-setup-9185l4` of
`skytekx/golem`, whose CI run 71 is green. Read it whole, then the pages it names, before the first
change. Delete it, and the folder `handover/`, once the work it describes is merged.

## 1. What Golem is and where it stands

Golem is the Rust host of the SkyTekx 3 server. It starts a JVM inside itself, boots Forge 47.4.18
for Minecraft 1.20.1 and every mod unchanged, and replaces Arclight, which runs the live server
today with EssentialsX 2.22.0, EssentialsXSpawn and LuckPerms 5.5.71. The live server is the
public repository `skytekx/skytekx3-1201-server`. The owner is Florian Kleber.

The wiki of Golem is `okf/`. Start with these pages, in this order:

| Page | What it gives you |
| --- | --- |
| `okf/index.md` | The state of every part on its day and the map of every page |
| `okf/decisions.md` | Every ruling of the owner, and the answers the agents gave to their own questions |
| `okf/plan.md` | The plan of the owner with the later rulings |
| `okf/open-items.md` | What is open, the decisions left to the owner among it |
| `okf/log.md` | What changed, oldest first |
| `CONTRIBUTING.md` | The rules of code, prose, commits, tests and licensing |
| `Documentation/testing.md` | The notebooks, what each asks, and how to run them |

What is built and held by notebooks, in short: the JVM host (`golem run`), the bridge mod with 16
natives and the game access of the plugins, the plugin runtime with the core plugin `golem`, the
permissions crate `golem-permissions` (the port of LuckPerms, MIT), the SQLite store `golem-store`,
the check of the real modpack (notebook 08 boots it under java, golem and Arclight), the license,
commit and docs lints, and CI that runs all eleven notebooks. What is designed and not built: the
GraphQL plugin API (`okf/api.md`, `okf/api-coverage.md`) and the Bukkit layer
(`okf/bukkit-layer.md`). The spec of the port of EssentialsX is `okf/plugins/essentials/`.

## 2. How the owner wants the work done

These rules stand in `okf/decisions.md` and `CONTRIBUTING.md`. The ones an agent breaks most easily:

1. **Answer your own blocking questions** with the choice that fits the plan and the code best, and
   record each answer in the table Answers of the agents of `okf/decisions.md`. Ask the owner only
   what only the owner can decide, and keep working on the default meanwhile.
2. **Two models on everything.** Every design is written and judged by two models, every build is
   checked by the second model before it is merged, and the points where they disagree are settled
   on their evidence. The owner wants every part correct and every quirk and bug found and
   recorded.
3. **Record every quirk and every bug.** A quirk of the old server, Forge or Minecraft gets an open
   issue with the label `quirk` and the sections Old behaviour, Evidence, Golem and, where needed,
   Decision of the owner. A bug of Golem gets an issue with the label `bug` and the sections
   Behaviour, Evidence and Fix, closed with the fixing commit once it is pushed. The issues #1 to
   #229 show the form. List the issues first and never file one twice.
4. **No public word on a live exploit.** The repository is public. A security issue that a player
   can use on the live server today goes to the owner for a private advisory, never into an issue,
   a commit or a page. Six such items exist. The owner holds their text, and six rows of the spec of
   EssentialsX are held back for them. Keep them held back.
5. **Commits.** Commitizen subjects in the form of the Linux kernel, `type(scope): summary` in lower
   case, at most 72 columns, no full stop. A body wrapped at 72 columns that says the problem first,
   then why the change solves it, then how. A fix of a pushed commit ends with the trailer
   `Fixes: <12 hex> ("<subject>")` as `git log -1 --abbrev=12 --format='%h ("%s")'` prints it, and
   the commit lint looks the hash up. Every commit is authored and committed as the owner, with the
   name and the address that `git log -1 --format='%an <%ae>' 5fe5a38` prints, and ends with the
   trailers `Co-Authored-By` of the model, `Claude-Session` of your session and
   `Signed-off-by: Florian Kleber <opensource@skytekx.com>`, in one last paragraph. One logical
   change per commit, and every commit builds on its own. Push right after a verified change, only
   to the branch `claude/aosabi-license-setup-9185l4`. Pushed history is not rewritten.
6. **Checks before a push.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --
   -D warnings`, `python3 -I .github/scripts/commit_lint.py --range <base>..HEAD`,
   `python3 -I .github/scripts/license_lint.py`, `python3 -I .github/scripts/docs_lint.py`,
   `reuse lint`, a build of every new commit on its own, and the notebooks that the change touches,
   all eleven before a merge of a large part.
7. **Tests are Jupyter notebooks**, never `#[test]`. A notebook asks one question in its first
   cell, holds its answers with asserts and is committed without outputs, with a `.license` file
   beside it, an entry in `tests/matrix.yaml` and a row in `Documentation/testing.md`. The
   generators that wrote the notebooks so far lived outside the repository and are gone, so edit
   the notebooks as JSON or write a generator of your own.
8. **Licenses.** Golem is AGPL 3 only. A port of a Bukkit plugin keeps the license of its upstream:
   `golem-essentials` and `golem-essentials-spawn` are GPL 3 or later, `golem-permissions` is MIT,
   and `ports.toml` with rule 9 of the license lint holds every port to it. Every file carries its
   SPDX tags.
9. **Prose** in code, pages and messages is plain English in the present tense, with no dashes as
   punctuation, no semicolons and no exclamation marks, they and them for people, and Markdown at
   100 columns. The docs lint checks the wiki, `Documentation/`, the README and CONTRIBUTING.md.
10. **Plugins of Golem are native Rust on Aosabi**, with no TypeScript and no WebAssembly. Bukkit
    and Spigot plugins run unchanged through the Bukkit layer in the JVM of Forge, and every plugin
    must work, as `okf/plan.md` section Bukkit plugins says.

## 3. The unfinished work in `handover/`

The last work on Golem wired the store and the permissions into `golem run` and closed the gaps of
the bridge for the port of EssentialsX. It stopped for this hand over before its notebooks ran and
before the check of the second model. Its commits are kept in the git bundle
`handover/port.bundle`, two branches on `5fe5a38`.

| Branch in the bundle | Commits | State |
| --- | --- | --- |
| `port/wiring` | 9 | The exit hook callback of `golem-jvm`, the store in the context of the plugins, `golem run` opening, seeding, flushing and closing the store, the permissions of the store resolved for every plugin with the answer that LuckPerms gave on Bukkit, the PermissionAPI handler, the commands of LuckPerms 5.5.71 in `golem-permissions`, and the pages. The cases of notebooks 04, 06 and 07 for it are not written yet, and its ninth commit is an untested change of notebook 09 |
| `port/bridge` | 4 | The block of notebook 05 set where the chunk is loaded, an id and an end event for every teleport, styled text built in Rust and sent as a component, and an unfinished fourth commit for the small queries and the quit position, which may not build |

Take them with `git fetch handover/port.bundle port/wiring:port/wiring port/bridge:port/bridge`.
Both branches start at `5fe5a38`. They conflict with each other in `Documentation/testing.md` and
`crates/golem-devtool/src/bridge.rs`, so merge the second after the first. What is left of the
bridge branch is items 4, 5 and 6 of `okf/plugins/essentials/bridge-needs.md` section What the
comparison shows: the small queries, the quit position, chat to a subset of players and kill with a
damage type, then the core plugin tools, notebook 07 and the pages. Then the second model checks
both parts, its findings are settled, all notebooks run, and the work is pushed and recorded. Each
part is written out in the section What is planned below.

## 4. Aosabi

Aosabi is `netsnek/aosabi`, the GraphQL framework that the API of Golem is built on, a project of
its own with its own rules and releases. Golem takes a tagged release of it and changes nothing in
it. What the design of the API asks of the framework, and what Golem does until each item is met,
stands in `okf/api.md` section 4.5 and in the section What Golem asks of Aosabi of
`okf/features.md`. The switch of the live server from Arclight needs nothing of Aosabi.

## 5. The feature list

The feature list is `okf/features.md`, one row per feature with its source, its state and the
notebook that holds it, kept current as features land. It gathers the parts of the API of
`okf/api.md`, the parts of the Bukkit layer of `okf/bukkit-layer.md`, which make 498 of the 503
surveyed plugins work, the specs of the port of EssentialsX, the plan, the open items and the open
issues of `skytekx/golem`.

## 6. What is planned

- The work of section 3, landed: the store opens beside the world and closes on every stop path,
  plugins check nodes for every kind of sender, the PermissionAPI answers as LuckPerms did through
  Arclight until the owner rules on #105, the commands of LuckPerms work in game and on the
  console, and items 1, 2, 4, 5 and 6 of the bridge needs are met.
- The port of the teleport side of EssentialsX and of EssentialsXSpawn after
  `okf/plugins/essentials/teleport.md`: tpa and its kin, back, homes, warps, spawn, with the player
  matching that the spec asks for, exact name or UUID and never a substring, and the fixes of its
  quirk issues, with the defaults of section 7 where the owner has not ruled.
- The port of the social, info, status, economy and admin side after `social.md`: messages, mail,
  ignore, AFK, suicide, balance, pay, worth, eco, gamemode, kick, ban.
- The migration tool from the data of EssentialsX and the export of LuckPerms into the store, with
  the rules of `okf/plan.md` section Data migration.
- Golem on the Mac mini under launchd, the comparison of Arclight and Golem on a copy of the live
  world, and the switch.
- The GraphQL API after `okf/api.md`, on a tagged release of Aosabi, with its notebooks numbered
  20 to 38, since 10 and 11 exist and 12 to 19 hold the switch.

## 7. Decisions open with the owner, and the default in force

The owner has not answered these yet. Work on the default and change course when they rule. The
issue numbers are of `skytekx/golem`.

| # | Question | Default |
| --- | --- | --- |
| 1 | #105: do Boolean permission nodes of mods keep the answers of LuckPerms through Arclight | Yes |
| 2 | #45: /home without a home and without a bed | The bed, then the point of /spawn |
| 3 | #48: homes addressed by number in an order nobody sees | No addressing by number |
| 4 | #59: /setspawn and the world spawn of vanilla | /setspawn in the Overworld sets both |
| 5 | #61: teleports out of a fight and 4 s of immunity | No combat tag, as on the live server |
| 6 | #66: the compass points at a home | Dropped, the compass points at the spawn |
| 7 | #75: helpop and mute | Mute applies to helpop |
| 8 | #87: automatic AFK | Off, as on the live server |
| 9 | #31: dark red names of operators | Off, and never in stored data |
| 10 | #96: help lists Bukkit commands alone | Help lists the Golem commands a player may use |
| 11 | #97: suicide and then back | A death by suicide sets no back point |
| 12 | `okf/api.md` 22.1: may a method of the client skip the executor if the first measurements show it is too slow | No, unless the owner allows it |
| 13 | `okf/api.md` 22.2 to 22.7 | As the page proposes |
| 14 | Paper plugins and the Paper API in the Bukkit layer | Yes, after the work of Spigot |
| 15 | Java 21 and 25 class files of plugins on the Java 17 of Forge 47 | Boot the pack on Java 21 first, else downgrade at load |
| 16 | The injection of the permissible of LuckPerms early | No, LuckPerms is ported |
| 17 | Golem's checks ask a Bukkit permission plugin that owns the decision | Yes |
| 18 | #175: drop `-Xrs` so that jcmd attaches again | No, keep `-Xrs`, with the line of `tmpfiles.d` on Linux, and a touch of the socket by golem once a day on every host once it is built, since macOS has no `tmpfiles.d` |
| 19 | #186: a plugin command takes the name of a command of a mod | Yes, as for vanilla |
| 20 | #206: a group without a weight ties with default | Keep, as documented |
| 21 | #216: pin every package of the harness with hashes | Yes |
| 22 | #217: CI runs twice on a branch with an open pull request | Keep |
| 23 | #223: the commit lint holds the sign off to the holder | No |
| 24 | #228: the width of a tab for the 100 columns of the Java of the bridge | 8, and rewrap the 65 lines |
| 25 | #102: skytekx-compat and Nostalgic Tweaks in the server repository | The pack keeps skytekx-compat 2.1.3, as the server repository records it, and a place for jars that the server alone loads takes the newer release. Whether Nostalgic Tweaks leaves the server stays with the owner |

The owner also has to file the six private advisories of section 2, point 4. The issue tool strips
the footer of Claude Code from the bodies of issues, and the owner ruled that it is not needed then.

## 8. Working notes

- `python3 -I tests/fetch.py` downloads the JDK, the Forge server and Gradle, and
  `python3 -I tests/fetch.py --only modpack` the pack of the live server for notebook 08, about
  1.4 GiB. A pack fetched before `5fe5a38` must be fetched again, notebook 08 says so.
- The harness of the notebooks is the Python of `tests/pyproject.toml`. Install it in a venv and
  run `python tests/run.py --only <id>`, or all of `tests/matrix.yaml`. A scratch copy of the matrix
  can override the server port of a notebook. Notebook 08 runs about 19 minutes in the cloud
  container, with four boots of up to 7.5 GiB each, the CI job about 35 minutes in all.
- The cloud container has 4 CPUs and 16 GB, a small disk allowance and restarts without warning.
  Commit each finished piece at once, run one Minecraft server at a time, give parallel agents their
  own ports and worktrees, and delete scratch directories and build output you no longer need.
- Use the GitHub tools for issues and CI logs. The issue tool strips the footer and wraps some links
  in backticks. Do not work around it.
- The tools that rebuild the survey of plugins are `tools/plugin-survey/`. The jars they download
  are untrusted data, which the tools read and never run.
