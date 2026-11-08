<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Golem

Golem is the Rust hybrid server of SkyTekx. It owns the server process, starts the JVM inside
itself and boots Minecraft 1.20.1 with Forge 47.4.18 and every mod of the pack unchanged. It
replaces Arclight. Around the game it adds a plugin runtime in Rust, and it will add a GraphQL API
built on [Aosabi](https://github.com/netsnek/aosabi) and native code for the hot paths of world
generation.

`golem run` boots the Forge server of a directory unchanged in its embedded JVM and stops it
through the server thread on a signal. A Forge mod in `bridge/` reports the game to the natives of
Golem, which hand it to the plugins in Rust, the first of them the command `golem` with the offer
of the source. The permissions that replace LuckPerms are a crate with the seed of SkyTekx. The
ports of EssentialsX and EssentialsXSpawn on the plugin runtime, the switch of the SkyTekx server
from Arclight to Golem and the GraphQL API are planned. The [wiki](okf/index.md) holds the state
and the design, and [Features](okf/features.md) lists what is built and what is planned.

## License

Golem is licensed under the GNU Affero General Public License 3.0 only, `AGPL-3.0-only`, whose
text is in [LICENSE](LICENSE). The ports of Bukkit plugins that [ports.toml](ports.toml) lists
keep the licenses of the plugins they port, `MIT` for the port of LuckPerms and `GPL-3.0-or-later`
for the ports of EssentialsX and EssentialsXSpawn. Whoever runs a changed Golem for players must
offer those players its source. Every file but [NOTICE](NOTICE) and the license texts states its
license in an SPDX tag at its head or in a `.license` file beside it. NOTICE takes its tags from
[REUSE.toml](REUSE.toml), and `LICENSE` and the texts under [LICENSES/](LICENSES/) carry none.
[Documentation/licensing.md](Documentation/licensing.md) explains these rules, the license texts
and the work of others that NOTICE names.

## Tests

Every test of Golem is a Jupyter notebook under [tests/notebooks/](tests/notebooks/), run by
papermill through `tests/run.py`. [Documentation/testing.md](Documentation/testing.md) says how to
run them and how to write one.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) is the rulebook of a change. Every commit is pushed at once,
signed off, and written as a Conventional Commit in the form of the Linux kernel, which
[.cz.toml](.cz.toml) and the commit lint check.
