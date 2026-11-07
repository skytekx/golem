<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Licensing

Golem is licensed under the GNU Affero General Public License 3.0 only, SPDX `AGPL-3.0-only`,
whose text is in [LICENSE](../LICENSE). This page explains why, who holds the copyright, how every
file of the repository states its license, where the license texts live, and which files carry
work of other projects.

Golem states licenses in [SPDX](https://spdx.dev/) identifiers first of all, and its tree is
compliant with [REUSE](https://reuse.software/), the specification of the FSFE that builds on
SPDX. Where REUSE leaves a choice open, Golem follows
[Aosabi](https://github.com/netsnek/aosabi/blob/main/Documentation/licensing.md) and the Linux
kernel, for example in where the SPDX line stands.

This page is a reading of the licenses and not legal advice.

## 1. Why the AGPL 3 only

The owner chose the AGPL on 2026-11-06 after a comparison with the GPL 2, the GPL 3 and the LGPL.

- The AGPL is the only one of them that reaches a server that is run but never distributed. Its
  section 13 says that whoever runs a changed Golem for players must offer those players its
  source. Under the GPL 2 or 3 the changes could stay private.
- The AGPL 3 takes Apache-2.0 code, so Aosabi works as it is. Aosabi is MIT, but some of its files
  are Apache-2.0, and its TLS stack uses `ring`, which is under Apache-2.0 and ISC terms. A license
  that is GPL 2 only could not take those parts.
- Forge is LGPL-2.1 and LuckPerms is MIT. Both may be combined with AGPL code.
- "Only" keeps Golem on the AGPL 3 that the owner read and chose. A later AGPL that the Free
  Software Foundation might publish does not apply to Golem by itself. A move to it would be a
  decision of its own, with the consent of every holder of code in Golem at that time. The owner
  ruled this on 2026-11-06, the same day as the choice of the AGPL.

## 2. The copyright holder

The copyright holder of Golem is Florian Kleber, reached at `opensource@skytekx.com`. Every file
of Golem names the holder in the line after its SPDX license line (section 3):

<!-- REUSE-IgnoreStart -->
```
SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
```

A file derived from another project keeps the holder of that project in NOTICE, as the license
lint does for Aosabi. A contributor who holds the copyright of their own change may add a line of
their own below, and the line of the holder stays. NOTICE names the holder as well.

The year is the one in which the file was first written, and it is not moved on later changes.

## 3. The SPDX lines

Every file states its license on its first line with an SPDX identifier, after the placement
rules of the Linux kernel, and its copyright holders on the lines right after, with the
`SPDX-FileCopyrightText` tag of REUSE. It carries no other license text. A script that starts
with a `#!` line, and a page that starts with the `---` of a front matter, move both down by one
line. The comment style follows the language of the file:

```
// SPDX-License-Identifier: AGPL-3.0-only                              Rust, Java, C, JavaScript
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>

# SPDX-License-Identifier: AGPL-3.0-only                               TOML, YAML, shell, Python
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>

<!-- SPDX-License-Identifier: AGPL-3.0-only -->                        Markdown outside the wiki
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
```

Unlike Aosabi, Golem has no default license for a file without the lines: every file carries
them. The exceptions are these.

- A page of the wiki under `okf/` opens with its front matter, so it carries both tags as YAML
  comments right after the opening `---`. A YAML reader skips them, and REUSE reads them:

  ```
  ---
  # SPDX-License-Identifier: AGPL-3.0-only
  # SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
  type: Plan
  title: ...
  ---
  ```

- A file that cannot hold a comment, such as JSON or an image, takes both tags from a file
  beside it whose name adds `.license`, as REUSE specifies. The tags may stand on any line of
  that file.

  ```
  config/example.json
  config/example.json.license
      # SPDX-License-Identifier: AGPL-3.0-only
      # SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
  ```

<!-- REUSE-IgnoreEnd -->

- [REUSE.toml](../REUSE.toml) gives a file its tags when the file itself must not carry them.
  That is `NOTICE` alone, whose text quotes the copyright lines of other projects. Every
  annotation there says why it exists.
- `LICENSE` and the texts under `LICENSES/` carry no tags, since they are the license texts
  themselves, and REUSE leaves them out.

A manifest also states the license in its own field. The `license` field of every `Cargo.toml`
says `AGPL-3.0-only`, in the workspace once and inherited by every crate with
`license.workspace = true`, and the Gradle build of the bridge mod sets the same identifier in the
license of its published metadata.

## 4. The license texts

The folder [LICENSES](../LICENSES) holds the text of every license a file of the repository
names, one file per license named by its SPDX identifier, as REUSE specifies.

| Text | Why it is here |
| --- | --- |
| `LICENSES/AGPL-3.0-only.txt` | The license of Golem. A new file always takes it |
| `LICENSES/MIT.txt` | Aosabi, from which the license lint is derived. No new file takes it alone |

`LICENSES/AGPL-3.0-only.txt` and `LICENSE` are the same bytes, the file `agpl-3.0.txt` of gnu.org
unchanged. `LICENSE` stays at the root so that GitHub reads the license of the repository.
`LICENSES/MIT.txt` is the LICENSE file of Aosabi at the commit Golem follows.

No text declares `AGPL-3.0-or-later`, so both lints refuse a file that names it. A license enters
`LICENSES/` in the same commit as the first file that names it, and NOTICE names the project in
that commit as well. A text that no file names any more leaves in the same way.

## 5. Code derived from other projects

A file written after the code of another project says so in a short comment in its header, in
the form `Derived from <project> <version>, <file>, <license>. Changed by the Golem authors.`
Its SPDX line names the AGPL of Golem together with the license of that project, joined by
`AND`, since the part it takes stays under that license, as REUSE asks. [NOTICE](../NOTICE) names
every such project with its version, the files that derive from it, its license and the copyright
lines of its license file, and keeps the notice its license asks to retain.

| File | SPDX line | Derived from |
| --- | --- | --- |
| `.github/scripts/license_lint.py` | `AGPL-3.0-only AND MIT` | Aosabi 0.1.0, `.github/scripts/license_lint.py`, MIT |

## 6. The ports, and what Golem never carries

A Golem plugin that ports a Bukkit plugin keeps the license of that plugin and names its holders,
as the owner ruled on 2026-11-06. [ports.toml](../ports.toml) lists every port with its path, its
upstream, the source of that version, its license and the holders of the upstream.

| Port | Upstream | License | Holders of the upstream |
| --- | --- | --- | --- |
| `crates/golem-permissions` | LuckPerms 5.5.71 | `MIT` | lucko (Luck), LuckPerms contributors |
| `crates/golem-essentials` | EssentialsX 2.22.0 | `GPL-3.0-or-later` | Essentials Team, EssentialsX contributors |
| `crates/golem-essentials-spawn` | EssentialsXSpawn 2.22.0 | `GPL-3.0-or-later` | Essentials Team, EssentialsX contributors |

The license of EssentialsX is the GPL 3 or any later version, as the header of `Essentials.java`
of release 2.22.0 states, with the LICENSE file of that release, which is
`LICENSES/GPL-3.0-or-later.txt` byte for byte. LuckPerms is under the MIT license of its
`LICENSE.txt`. Every file of a port carries the license of its upstream on its first line and a
copyright line for each holder of the upstream beside the one of Florian Kleber, for example in
`golem-essentials`:

<!-- REUSE-IgnoreStart -->

```text
// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
// SPDX-FileCopyrightText: 2011 Essentials Team
// SPDX-FileCopyrightText: EssentialsX contributors
```

<!-- REUSE-IgnoreEnd -->

Its `Cargo.toml` states the license itself instead of the one of the workspace, and
[NOTICE](../NOTICE) has a section on its upstream with the notice its license asks to keep. Since
a port stays under the license of its upstream, it may translate the code, the messages and the
texts of that upstream into Golem. The code of Golem itself, the crates `golem`, `golem-jvm`,
`golem-bridge`, `golem-plugin`, `golem-store` and `golem-devtool` and the bridge mod, takes no code
of an upstream and stays under the AGPL 3 only.

The GPL 3 and the AGPL 3 each allow, in their section 13, to combine a work under one with a work
under the other into one program, and the MIT license combines with both. The program `golem`
links its own crates with the ports that way. Each part keeps its license, and since the
combination holds AGPL code, whoever runs it for players must offer them the source of all of it,
which `/golem source` does.

Golem never carries these:

- Minecraft. The server jar comes from Mojang's download, as it does today, and Golem never ships
  it.
- Code of Arclight and CraftBukkit, which Golem replaces instead of porting.

## 7. Dependencies

A dependency that the build fetches, such as the Aosabi crates or a crate of crates.io, is not
carried in this tree, so it adds no text to `LICENSES/`. Once the first crate lands, CI runs
`cargo deny check licenses` with an allow list of the licenses that may be combined with the AGPL
3, so that no dependency brings an incompatible one.

## 8. Making the source available

Golem follows its own license on the SkyTekx server. The command `/golem source`, the text of the
server list and the field `server.source` of the GraphQL API name this repository and the exact
commit that runs, so every player can get the source of the running version, as section 13 of
the AGPL asks.

## 9. The lints

CI runs two checks on every push and every pull request, and both must pass.

```
python3 -I .github/scripts/license_lint.py
reuse lint
```

- `reuse lint`, the tool of REUSE, checks that every file has a license and a copyright, that
  every license it names has its text under `LICENSES/`, and that no text is unused or malformed.
  CI pins its version.
- [.github/scripts/license_lint.py](../.github/scripts/license_lint.py) checks what REUSE leaves
  open: that the SPDX line stands on the first line, that `LICENSE` is the same bytes as
  `LICENSES/AGPL-3.0-only.txt`, that every file names Florian Kleber as a holder, that
  `REUSE.toml` gives NOTICE its tags with the precedence `override`, that every `Cargo.toml`
  states a license, that every package under `third_party/` keeps its own license file, that
  NOTICE exists once a file names a license other than the AGPL, and that every port of
  `ports.toml` keeps the license and the holders of its upstream. Its rules are written in the
  docstring of the script.

## 10. Contributing

A contribution is licensed under the license of the file it changes, and a new file takes the
AGPL 3 only with its SPDX lines. A file derived from another project keeps its credit, and the
project stays in NOTICE.

Every commit carries a `Signed-off-by` line under the
[Developer Certificate of Origin](https://developercertificate.org/):

```
Signed-off-by: Florian Kleber <opensource@skytekx.com>
```

The owner ruled on 2026-11-06 that every commit is signed off, also a commit that an AI agent
writes, and that the line names the holder. The author of a commit is the person or the account
that made it, and an agent that helped names itself in a `Co-Authored-By` line.
