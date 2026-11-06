<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com> -->
# Golem

Golem is the Rust hybrid server of SkyTekx, built on [Aosabi](https://github.com/netsnek/aosabi).
It owns the server process, starts the JVM inside itself and boots Minecraft 1.20.1 with Forge
47.4.18 and every mod of the pack unchanged. Around the game it adds a GraphQL API, a plugin
runtime in Rust and native code for the hot paths of world generation. It replaces Arclight.

Nothing of the server is built yet. This repository holds its license and its test harness so far.

## License

Golem is licensed under the GNU Affero General Public License 3.0 only,
`AGPL-3.0-only`, whose text is in [LICENSE](LICENSE). Whoever runs a changed Golem for players
must offer those players its source. Every file names its license on its first line with an SPDX
identifier, and [Documentation/licensing.md](Documentation/licensing.md) explains the rules,
the texts under [LICENSES/](LICENSES/) and the work of others that [NOTICE](NOTICE) names.

## Tests

Every test of Golem is a Jupyter notebook under [tests/notebooks/](tests/notebooks/), run by
papermill through `tests/run.py`, as in Aosabi. [Documentation/testing.md](Documentation/testing.md)
says how to run them and how to write one.

## Contributing

[CONTRIBUTING.md](CONTRIBUTING.md) is the rulebook of a change. Every commit is pushed at once,
signed off, and written as a Conventional Commit in the form of the Linux kernel, which
[.cz.toml](.cz.toml) and the commit lint check.
