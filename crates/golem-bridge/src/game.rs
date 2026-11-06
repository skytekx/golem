// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game of golem-bridge: the places of the world as the mod reports them, such as the
 * location of a player when they die.
 */

use std::fmt;

/// A place in the world with a rotation, as the game keeps it for a player.
///
/// The coordinates are those of the feet. The rotation is in degrees: `yaw` around the vertical
/// axis, where 0 faces south and 90 west, and `pitch` up and down, where -90 looks straight up.
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    /// The id of the dimension, such as minecraft:overworld.
    pub dimension: String,
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate, the height.
    pub y: f64,
    /// The z coordinate.
    pub z: f64,
    /// The rotation around the vertical axis, in degrees.
    pub yaw: f32,
    /// The rotation up and down, in degrees.
    pub pitch: f32,
}

impl Location {
    /// Answers whether every coordinate and angle is a finite number, which the game needs.
    pub fn is_finite(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.z.is_finite()
            && self.yaw.is_finite()
            && self.pitch.is_finite()
    }
}

impl fmt::Display for Location {
    /// Writes the dimension, the coordinates with two decimals and the rotation with one, as
    /// the log and the answers of a command show a location.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {:.2} {:.2} {:.2}, yaw {:.1}, pitch {:.1}",
            self.dimension, self.x, self.y, self.z, self.yaw, self.pitch
        )
    }
}
