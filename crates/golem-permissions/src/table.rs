// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The table of one source, a group or the nodes of a player, built once at load time so that a
 * check costs one lookup per level of the node it names and allocates nothing.
 */

use std::collections::HashMap;

use crate::node::{
    Pattern,
    Rule, //
};

/// What one source says about one pattern. A source may grant and deny the same pattern, and
/// then the deny wins, since both stand at the same specificity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Value {
    pub grant: bool,
    pub deny: bool,
}

impl Value {
    /// The value of the pattern, false when the source denies it at all.
    pub fn granted(self) -> bool {
        !self.deny
    }

    fn add(&mut self, deny: bool) {
        if deny {
            self.deny = true;
        } else {
            self.grant = true;
        }
    }
}

/// The most specific pattern of a source that matches a node, borrowed from that node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Hit<'a> {
    pub value: Value,
    pub pattern: Pattern<'a>,
}

/// The patterns of one source, split by kind so that a check never builds a string.
#[derive(Clone, Debug, Default)]
pub(crate) struct Table {
    exact: HashMap<Box<str>, Value>,
    below: HashMap<Box<str>, Value>,
    every: Option<Value>,
}

impl Table {
    /// Adds a node of the configuration.
    pub fn add(&mut self, rule: Rule<'_>) {
        let value = match rule.pattern {
            Pattern::Exact(node) => self.exact.entry(node.into()).or_default(),
            Pattern::Below(prefix) => self.below.entry(prefix.into()).or_default(),
            Pattern::Every => self.every.get_or_insert_with(Value::default),
        };
        value.add(rule.deny);
    }

    /// Finds the most specific pattern that matches `node`: the node itself, then the wildcard
    /// with the longest prefix, then `*`. A wildcard matches the nodes below its prefix and not
    /// the prefix itself, as in `LuckPerms`.
    pub fn find<'a>(&self, node: &'a str) -> Option<Hit<'a>> {
        if let Some(&value) = self.exact.get(node) {
            return Some(Hit {
                value,
                pattern: Pattern::Exact(node),
            });
        }
        if !self.below.is_empty() {
            let mut end = node.len();
            while let Some(dot) = node[..end].rfind('.') {
                let prefix = &node[..dot];
                if let Some(&value) = self.below.get(prefix) {
                    return Some(Hit {
                        value,
                        pattern: Pattern::Below(prefix),
                    });
                }
                end = dot;
            }
        }
        self.every.map(|value| Hit {
            value,
            pattern: Pattern::Every,
        })
    }
}
