// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game as the Rust of Golem may ask it: the players online, their places and the places of
 * the worlds, the item in a hand and a safe place to stand.
 *
 * The bridge mod implements this interface and hands it to Golem with Host.connect. Golem calls
 * it through the JNI only while a native of the server thread runs, so every answer is read on
 * the thread that owns the game. okf/bridge.md is the contract for every method here.
 */
package com.skytekx.golem.host;

/**
 * What Golem may ask the game, on the server thread.
 *
 * <p>Every answer is its fields joined by the unit separator U+001F, as the actions are. An
 * answer is null when the game cannot answer, such as for a player who is not online, a
 * dimension the server does not have or a call from another thread, and the empty string when
 * the game answers that there is none, such as a player without a respawn point.
 */
public interface Game {
	/**
	 * Answers every player online, each as their UUID, the name of their profile, their display
	 * name, their ping in milliseconds, their game mode, their permission level, the
	 * milliseconds since their last action and their name in the player list, which is empty
	 * when the client shows the name of the profile.
	 *
	 * @return one answer per player, or null
	 */
	String[] players();

	/**
	 * Answers where a player is: the dimension, x, y and z of the feet, the yaw and the pitch.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @return the location, or null when the player is not online
	 */
	String location(String uuid);

	/**
	 * Answers the respawn point of a player, a bed or a respawn anchor: the dimension and x, y
	 * and z of the block.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @return the point, the empty string when the player has none, or null when the player is
	 *     not online
	 */
	String respawn(String uuid);

	/**
	 * Answers the shared spawn of a dimension: x, y and z of the block and the angle.
	 *
	 * @param dimension the id of the dimension, such as minecraft:overworld
	 * @return the spawn, or null when the server has no such dimension
	 */
	String spawn(String dimension);

	/**
	 * Answers the item in the main hand of a player: its registry id and the count.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @return the item, the empty string for an empty hand, or null when the player is not online
	 */
	String mainHand(String uuid);

	/**
	 * Answers the heights of a dimension: the lowest y of a block and the y above the highest.
	 *
	 * @param dimension the id of the dimension, such as minecraft:overworld
	 * @return the heights, or null when the server has no such dimension
	 */
	String heights(String dimension);

	/**
	 * Answers the nearest place near a target where a player stands safely: x, y and z of the
	 * feet, in the middle of the block. The search reads the blocks of loaded chunks alone and
	 * loads none.
	 *
	 * @param dimension the id of the dimension, such as minecraft:overworld
	 * @param x the x coordinate of the target
	 * @param y the y coordinate of the target
	 * @param z the z coordinate of the target
	 * @param radius how far from the block of the target to look, in blocks, at most 8
	 * @return the place, the empty string when there is none within the radius, the word unloaded
	 *     when the search met a chunk that is not loaded before it found a place, or null when the
	 *     server has no such dimension or the target is no finite number
	 */
	String safeLocation(String dimension, double x, double y, double z, int radius);
}
