// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The subcommands of /golem that ask the game: whoami for everyone, and player, world, teleport
 * and listname for the console and the operators of level 2 and more.
 *
 * They are the tools of an operator who wants to see what Golem sees: who runs a command and at
 * which level, where a player is and how they play, the spawn and the heights of a dimension, a
 * teleport to the nearest safe place across dimensions, and the name of a player in the player
 * list. Each reads the game through the context of its call and changes it through actions
 * alone, as every plugin does.
 */

use golem_bridge::{
    Game,
    Location,
    OnlinePlayer,
    SafePlace,
    Sender, //
};

use crate::Context;

/// The level from which a player or another source may run the subcommands of operators, the
/// level of a game master in Minecraft.
pub const OPERATOR_LEVEL: u8 = 2;

/// How far from its target a teleport of /golem teleport looks for a safe place, in blocks.
pub const SAFE_RADIUS: u32 = 3;

/// The dimension of /golem world without an argument.
const OVERWORLD: &str = "minecraft:overworld";

/// The line of a question that the game did not answer.
const NO_ANSWER: &str = "The game gives no answer now, so try again.";

/// Answers the lines of `/golem whoami`: who Golem sees run the command, and at which level.
pub fn whoami(sender: &Sender, game: &dyn Game) -> Vec<String> {
    let line = match sender {
        Sender::Console => "You run /golem as the console.".to_owned(),
        Sender::Other { name, level } => format!("You run /golem as {name} at the level {level}."),
        Sender::Player { player, level } => match game.player(&player.uuid) {
            Some(Some(online)) if online.level == *level => format!(
                "You run /golem as the player {} at the operator level {}.",
                player.name, online.level
            ),
            Some(Some(online)) => format!(
                "You run /golem as the player {} at the operator level {}, from a source at the level {level}.",
                player.name, online.level
            ),
            _ => format!(
                "You run /golem as the player {} from a source at the level {level}.",
                player.name
            ),
        },
    };
    vec![line]
}

/// Answers the refusal of the subcommand `name` for a sender below the operator level, or
/// `None` when the sender may run it. A player counts at the lower of the level of the source
/// and their operator level, and a player whom the game does not answer reads that the game
/// gives no answer, since that says nothing about their level.
pub fn refusal(name: &str, sender: &Sender, game: &dyn Game) -> Option<Vec<String>> {
    let allowed = match sender {
        Sender::Console => true,
        Sender::Other { level, .. } => *level >= OPERATOR_LEVEL,
        Sender::Player { player, level } => match game.player(&player.uuid) {
            Some(Some(online)) => online.level.min(*level) >= OPERATOR_LEVEL,
            Some(None) => false,
            None => return Some(vec![NO_ANSWER.to_owned()]),
        },
    };
    (!allowed).then(|| {
        vec![format!(
            "Only the console and operators of level {OPERATOR_LEVEL} and more may run /golem {name}."
        )]
    })
}

/// Answers the player online of the name or UUID `who`, or the lines that say why there is
/// none.
fn online(game: &dyn Game, who: &str) -> Result<OnlinePlayer, Vec<String>> {
    match game.player(who) {
        Some(Some(player)) => Ok(player),
        Some(None) => Err(vec![format!("No player {who} is online.")]),
        None => Err(vec![NO_ANSWER.to_owned()]),
    }
}

/// Answers the lines of `/golem player <name>`: where the player is and how they play.
pub fn player(words: &[&str], context: &Context<'_>) -> Vec<String> {
    let [who] = words else {
        return vec!["Usage: /golem player <name>".to_owned()];
    };
    let game = context.game();
    let player = match online(game, who) {
        Ok(player) => player,
        Err(lines) => return lines,
    };
    let Some(location) = game.location(&player.uuid) else {
        return vec![NO_ANSWER.to_owned()];
    };
    let respawn = match game.respawn(&player.uuid) {
        Some(Some(block)) => block.to_string(),
        Some(None) => "none".to_owned(),
        None => "unknown".to_owned(),
    };
    let hand = match game.main_hand(&player.uuid) {
        Some(Some(item)) => format!("{} {}", item.count, item.id),
        Some(None) => "empty".to_owned(),
        None => "unknown".to_owned(),
    };
    // Milliseconds are whole numbers far below 2^52, which an f64 holds exactly.
    #[allow(clippy::cast_precision_loss)]
    let idle = player.idle_ms as f64 / 1000.0;
    vec![
        format!("{} ({}): {location}", player.name, player.uuid),
        format!(
            "Game mode {}, ping {} ms, operator level {}, idle {idle:.1} s",
            player.game_mode.name(),
            player.ping_ms,
            player.level
        ),
        format!("Display name: {}", player.display_name),
        format!(
            "List name: {}",
            player.list_name.as_deref().unwrap_or("none")
        ),
        format!("Respawn point: {respawn}"),
        format!("Main hand: {hand}"),
    ]
}

/// Answers the lines of `/golem world [<dimension>]`: the spawn and the heights of a dimension.
pub fn world(words: &[&str], context: &Context<'_>) -> Vec<String> {
    let dimension = match words {
        [] => OVERWORLD,
        [dimension] => dimension,
        _ => return vec!["Usage: /golem world [<dimension>]".to_owned()],
    };
    let game = context.game();
    let (Some(spawn), Some(heights)) = (game.spawn(dimension), game.heights(dimension)) else {
        return vec![format!(
            "The server has no dimension {dimension}, or the game gives no answer now."
        )];
    };
    vec![format!(
        "{dimension}: spawn {} {} {} at the angle {:.1}, blocks from y {} below y {}",
        spawn.block.x, spawn.block.y, spawn.block.z, spawn.angle, heights.min, heights.max
    )]
}

/// Answers the lines of `/golem teleport <name> <dimension> <x> <y> <z>` and queues the
/// teleport to the nearest safe place within [`SAFE_RADIUS`] blocks, with the rotation the
/// player has.
pub fn teleport(words: &[&str], context: &Context<'_>) -> Vec<String> {
    const USAGE: &str = "Usage: /golem teleport <name> <dimension> <x> <y> <z>";
    let [who, dimension, x, y, z] = words else {
        return vec![USAGE.to_owned()];
    };
    let (Some(x), Some(y), Some(z)) = (number(x), number(y), number(z)) else {
        return vec![format!("{USAGE}, with numbers for x, y and z")];
    };
    let game = context.game();
    let player = match online(game, who) {
        Ok(player) => player,
        Err(lines) => return lines,
    };
    if game.heights(dimension).is_none() {
        return vec![format!(
            "The server has no dimension {dimension}, or the game gives no answer now."
        )];
    }
    let Some(now) = game.location(&player.uuid) else {
        return vec![NO_ANSWER.to_owned()];
    };
    let target = Location {
        dimension: (*dimension).to_owned(),
        x,
        y,
        z,
        ..now
    };
    if !target.is_in_world() {
        return vec![format!(
            "{USAGE}, with x and z from -30000000 to below 30000000 and y from -20000000 to below 20000000"
        )];
    }
    match game.safe_location(&target, SAFE_RADIUS) {
        Some(SafePlace::Found(place)) => {
            if context.teleport(&player.uuid, &place) {
                vec![format!("Queued a teleport of {} to {place}.", player.name)]
            } else {
                vec![format!("{place} is no place to teleport to.")]
            }
        }
        Some(SafePlace::Nowhere) => vec![format!(
            "No safe place within {SAFE_RADIUS} blocks of {target}, so {} stays.",
            player.name
        )],
        Some(SafePlace::Unloaded) => vec![format!(
            "The chunks near {target} are not loaded, so {} stays, since the search for a safe place loads no chunk.",
            player.name
        )],
        None => vec![NO_ANSWER.to_owned()],
    }
}

/// Answers a word as a finite number.
fn number(word: &str) -> Option<f64> {
    word.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// Answers the lines of `/golem listname <name> [<text>]` and queues the name, or its clearing
/// without a text.
///
/// `arguments` is everything after the subcommand as it was typed. One whitespace character
/// after the name separates it from the text, which is taken as it is, runs of spaces included.
pub fn list_name(arguments: &str, context: &Context<'_>) -> Vec<String> {
    let arguments = arguments.trim_start();
    let (who, text) = arguments
        .split_once(char::is_whitespace)
        .unwrap_or((arguments, ""));
    if who.is_empty() {
        return vec!["Usage: /golem listname <name> [<text>]".to_owned()];
    }
    let player = match online(context.game(), who) {
        Ok(player) => player,
        Err(lines) => return lines,
    };
    if text.is_empty() {
        context.clear_list_name(&player.uuid);
        vec![format!(
            "The player list shows the name of {} again.",
            player.name
        )]
    } else {
        context.set_list_name(&player.uuid, text);
        vec![format!("The player list names {} {text}.", player.name)]
    }
}
