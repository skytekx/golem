// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The build script of golem: names the commit the binary is built from, for golem source and
 * the core plugin, since section 13 of the AGPL asks the server to offer its exact source.
 *
 * The commit is the variable GOLEM_COMMIT when it is set, which a build outside a git checkout
 * uses. Else it is HEAD of the git checkout of Golem itself, and only when the work tree of that
 * checkout is the workspace and tracks this script: a copy of Golem inside the checkout of
 * another project would name a commit of that project. When the files that make the binary, the
 * crates, the manifest, the lock file and the toolchain file, differ from HEAD, the commit ends
 * in -dirty, since the tree of HEAD is not the source that runs, and the core plugin then gives
 * no link to that tree. Else the commit is unknown.
 *
 * Cargo runs the script again when the variable, HEAD, a ref, the packed refs, the index or a
 * file that makes the binary change, so a new commit or a change never keeps the old name.
 */

//! The build script of golem, which names the commit of the binary.

use std::{
    path::{
        Path,
        PathBuf, //
    },
    process::Command,
};

/// The files and directories of the workspace that make the binary.
const BUILT_FROM: [&str; 4] = ["crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"];

/// The output of a git command in `directory`, trimmed, or None when git fails or is missing.
fn git_output(directory: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_owned())
}

/// The output of a git command in `directory` as [`git_output`] answers it, or None when it is
/// empty.
fn git(directory: &Path, arguments: &[&str]) -> Option<String> {
    git_output(directory, arguments).filter(|text| !text.is_empty())
}

/// The commit of the git checkout of Golem whose work tree is `root`, with -dirty when the files
/// that make the binary differ from it, or None when `root` is no such work tree.
fn checkout_commit(root: &Path) -> Option<String> {
    let top = PathBuf::from(git(root, &["rev-parse", "--show-toplevel"])?);
    if top.canonicalize().ok()? != root.canonicalize().ok()? {
        return None;
    }
    git(
        root,
        &["ls-files", "--error-unmatch", "crates/golem/build.rs"],
    )?;
    // A work tree keeps HEAD and the index apart from the refs, so git names each path itself.
    for name in ["HEAD", "refs", "packed-refs", "index"] {
        if let Some(path) = git(root, &["rev-parse", "--git-path", name]) {
            let path = root.join(path);
            if path.exists() {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
    }
    let head = git(root, &["rev-parse", "HEAD"])?;
    // Without optional locks git status leaves the index alone, which would run this script again.
    let mut status = vec!["--no-optional-locks", "status", "--porcelain", "--"];
    status.extend(BUILT_FROM);
    let changes = git_output(root, &status)?;
    Some(if changes.is_empty() {
        head
    } else {
        format!("{head}-dirty")
    })
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    // The workspace holds this crate as crates/golem.
    let root = manifest.join("..").join("..");
    let root = root.canonicalize().unwrap_or(root);
    println!("cargo::rerun-if-env-changed=GOLEM_COMMIT");
    for path in BUILT_FROM {
        let path = root.join(path);
        if path.exists() {
            println!("cargo::rerun-if-changed={}", path.display());
        }
    }
    let commit = std::env::var("GOLEM_COMMIT")
        .ok()
        .map(|commit| commit.trim().to_owned())
        .filter(|commit| !commit.is_empty())
        .or_else(|| checkout_commit(&root))
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo::rustc-env=GOLEM_COMMIT={commit}");
}
