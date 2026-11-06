// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The build script of golem: names the commit the binary is built from, for golem source and
 * the core plugin, since section 13 of the AGPL asks the server to offer its exact source.
 *
 * The commit is the variable GOLEM_COMMIT when it is set, which a build outside a git checkout
 * uses, else HEAD of the checkout as git rev-parse names it, else unknown. Cargo runs the script
 * again when the variable, HEAD, a ref or the packed refs change, so a new commit never keeps
 * the name of the old one.
 */

//! The build script of golem, which names the commit of the binary.

use std::{
    path::{
        Path,
        PathBuf, //
    },
    process::Command,
};

/// The output of a git command in `directory`, trimmed, or None when git fails or is missing.
fn git(directory: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    println!("cargo::rerun-if-env-changed=GOLEM_COMMIT");
    // A work tree keeps HEAD apart from the refs, so git names each path itself.
    for name in ["HEAD", "refs", "packed-refs"] {
        if let Some(path) = git(&manifest, &["rev-parse", "--git-path", name]) {
            let path = manifest.join(path);
            if path.exists() {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
    }
    let commit = std::env::var("GOLEM_COMMIT")
        .ok()
        .map(|commit| commit.trim().to_owned())
        .filter(|commit| !commit.is_empty())
        .or_else(|| git(&manifest, &["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo::rustc-env=GOLEM_COMMIT={commit}");
}
