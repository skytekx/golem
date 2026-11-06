// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The tick statistics of golem-bridge: the window of the last ticks that tick() records, and the
 * TPS and MSPT it answers.
 */

use std::{
    collections::VecDeque,
    time::Instant, //
};

/// The ticks the statistics look back on.
pub const WINDOW: usize = 100;

/// The ticks per second of a server that keeps up, which the TPS never exceeds.
pub const MAX_TPS: f64 = 20.0;

/// The tick statistics of the running server.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TickStats {
    /// The ticks since the start.
    pub ticks: u64,
    /// The mean milliseconds per tick over the last 100 ticks.
    pub mspt: f64,
    /// The ticks per second over the last 100 ticks, at most 20.
    pub tps: f64,
}

/// The last [`WINDOW`] ticks, each with its duration and the instant it ended.
///
/// The MSPT is the mean of the durations. The TPS is the number of intervals between the ticks
/// of the window divided by the wall clock time from the first to the last, capped at
/// [`MAX_TPS`], since the server waits out the rest of a tick that ends early. It is 0 while the
/// window holds fewer than two ticks, which span no interval.
#[derive(Clone, Debug, Default)]
pub struct TickWindow {
    ticks: u64,
    window: VecDeque<(u64, Instant)>,
}

impl TickWindow {
    /// An empty window.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a tick that took `nanos` and ended at `at`.
    ///
    /// The instants come in the order of the ticks, as the server thread calls `tick()`.
    pub fn record(&mut self, nanos: u64, at: Instant) {
        self.ticks = self.ticks.saturating_add(1);
        if self.window.len() == WINDOW {
            self.window.pop_front();
        }
        self.window.push_back((nanos, at));
    }

    /// Answers the statistics of the window.
    pub fn stats(&self) -> TickStats {
        let (Some(&(_, first)), Some(&(_, last))) = (self.window.front(), self.window.back())
        else {
            return TickStats::default();
        };
        let count = self.window.len();
        let total: u128 = self
            .window
            .iter()
            .map(|&(nanos, _)| u128::from(nanos))
            .sum();
        // The count is at most 100 and the total is a sum of 100 durations, so an f64 holds
        // both closely enough for a mean in milliseconds.
        #[allow(clippy::cast_precision_loss)]
        let (count, total) = (count as f64, total as f64);
        let mspt = total / count / 1_000_000.0;
        let tps = if count < 2.0 {
            0.0
        } else {
            let elapsed = last.saturating_duration_since(first).as_secs_f64();
            if elapsed > 0.0 {
                ((count - 1.0) / elapsed).min(MAX_TPS)
            } else {
                MAX_TPS
            }
        };
        TickStats {
            ticks: self.ticks,
            mspt,
            tps,
        }
    }
}
