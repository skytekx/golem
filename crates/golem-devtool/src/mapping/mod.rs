// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of the mapping, the files under api/ that hold the plugin API to its
 * reference, the jar of spigot-api that tests/fetch.py pins. api inventory writes the inventory
 * of that jar, and api numbers counts an inventory.
 */

//! The primitives of the mapping of the plugin API.
//!
//! `api inventory --jar <jar> --sources <sources jar> --javap <javap> --out <file>` reads the
//! public members of every class of the jar with `javap -public` and the names of the
//! parameters from the sources jar, writes the inventory to the file and prints its numbers,
//! with `written`, `bytes`, `classesOfJar`, the classes of the jar, and `leftOut`, the classes
//! named `package-info`, which the inventory does not hold.
//!
//! `api numbers <inventory>` prints the numbers of an inventory that is written: its types,
//! methods and fields, the types that are public and those that are anonymous, the packages,
//! the simple names that more than one package uses, the interfaces of the entity tree with its
//! leaves, the event classes with those that are concrete, that a listener can cancel and that
//! a listener can name, their handler lists and their setters, and how many methods with
//! parameters carry the names of them.

mod archive;
pub mod inventory;
mod javap;
mod sha256;
mod sources;

use std::{
    collections::BTreeMap,
    fs,
    path::Path, //
};

use serde_json::{
    Value,
    json, //
};

use self::inventory::Inventory;
use crate::bridge::finish;

/// Reads `arguments` as pairs of a flag and its value, each of `flags` once and no other.
fn options<'a>(
    name: &str,
    flags: &[&'static str],
    arguments: &'a [String],
) -> Result<BTreeMap<&'static str, &'a Path>, String> {
    let usage = || format!("{name} takes {}", flags.join(" <path> ") + " <path>");
    let mut found = BTreeMap::new();
    for pair in arguments.chunks(2) {
        let [flag, value] = pair else {
            return Err(usage());
        };
        let known = flags
            .iter()
            .find(|known| **known == flag.as_str())
            .ok_or_else(usage)?;
        if found.insert(*known, Path::new(value)).is_some() {
            return Err(usage());
        }
    }
    if found.len() != flags.len() {
        return Err(usage());
    }
    Ok(found)
}

/// `api inventory`: writes the inventory of a jar and prints its numbers.
pub fn inventory(arguments: &[String]) -> i32 {
    finish("api inventory", write_inventory(arguments))
}

fn write_inventory(arguments: &[String]) -> Result<Value, String> {
    let given = options(
        "api inventory",
        &["--jar", "--sources", "--javap", "--out"],
        arguments,
    )?;
    let (inventory, written) =
        Inventory::build(given["--jar"], given["--sources"], given["--javap"])?;
    let text = inventory.text();
    let out = given["--out"];
    fs::write(out, &text).map_err(|error| format!("{}: {error}", out.display()))?;
    let mut numbers = inventory.numbers();
    numbers["written"] = json!(out.display().to_string());
    numbers["bytes"] = json!(text.len());
    numbers["classesOfJar"] = json!(written.classes);
    numbers["leftOut"] = json!(written.left_out);
    Ok(numbers)
}

/// `api numbers`: prints the numbers of an inventory that is written.
pub fn numbers(arguments: &[String]) -> i32 {
    let numbers = match arguments {
        [path] => Inventory::read(Path::new(path)).map(|inventory| inventory.numbers()),
        _ => Err(String::from("api numbers takes one inventory")),
    };
    finish("api numbers", numbers)
}
