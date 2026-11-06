// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * golem: the Rust hybrid server of SkyTekx.
 *
 * golem run starts the JVM inside this process, boots the Forge server of a directory unchanged
 * and runs the plugins around it. A signal never stops the server: the first SIGINT, SIGTERM or
 * SIGHUP queues save-all flush and stop on the server thread, and golem waits until the JVM has
 * returned. golem source names the repository and the commit of the running binary, as section
 * 13 of the AGPL asks.
 */

//! The program golem.

fn main() {
    todo!("golem: main")
}
