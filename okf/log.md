---
# SPDX-License-Identifier: AGPL-3.0-only
# SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
type: Log
title: Change log
description: Chronological change log of Golem from the plan of 2026-11-06 on.
tags: [log, history]
timestamp: 2026-11-06
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
  process, reached `Done` in 26 s, stopped it through the server thread and saw `DestroyJavaVM`
  return 0. A second spike proved that a Forge mod calls a native method of a class on the module
  path that the host registered. See [Bridge](bridge.md).
- The Cargo workspace of the first milestone went in, with the crates `golem`, `golem-jvm`,
  `golem-bridge`, `golem-plugin`, `golem-permissions` and `golem-devtool`.
