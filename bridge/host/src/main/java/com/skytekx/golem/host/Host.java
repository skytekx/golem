// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The host class of Golem: the native methods that the Rust of Golem registers on this class
 * before it starts Forge, and the probe that tells the bridge mod whether it did.
 *
 * The class lives alone in golem-host.jar on the module path of the JVM, outside the layers that
 * Forge builds, so the bridge mod in the game layer and the JVM host see the same class. It loads
 * no library and looks up no symbol: the JVM host registers every native method with
 * RegisterNatives. okf/bridge.md is the contract for every method here.
 */
package com.skytekx.golem.host;

/**
 * The native methods of Golem, which the bridge mod calls on the server thread.
 *
 * <p>A native method never throws: the Rust side catches every panic and answers what a pass
 * answers, nothing, false or an empty array. On a stock Forge no native is registered, so every
 * one of them throws {@link UnsatisfiedLinkError}, and {@link #available()} tells the mod so
 * before it calls any of them.
 */
public final class Host {
	/** The answer of {@link #available()}, or null before its first call. */
	private static Boolean available;

	private Host() {
	}

	/**
	 * Answers true when a Golem host registered the native methods, false on a stock Forge.
	 *
	 * <p>The first call asks {@link #version()} once and keeps the answer for every later call.
	 *
	 * @return whether the native methods of this class answer
	 */
	public static synchronized boolean available() {
		if (available == null) {
			available = probe();
		}
		return available;
	}

	private static boolean probe() {
		try {
			version();
			return true;
		} catch (UnsatisfiedLinkError missing) {
			return false;
		}
	}

	/**
	 * Answers the version of Golem, such as 0.1.0.
	 *
	 * @return the version of the Golem that registered the natives
	 */
	public static native String version();

	/**
	 * Reports that the server is starting, before the worlds load.
	 *
	 * @param forgeVersion the version of Forge, such as 47.4.18
	 * @param minecraftVersion the version of Minecraft, such as 1.20.1
	 */
	public static native void serverStarting(String forgeVersion, String minecraftVersion);

	/** Reports that the server has started and reached Done. */
	public static native void serverStarted();

	/** Reports that the server is stopping. */
	public static native void serverStopping();

	/** Reports that the server has stopped, after the worlds were saved. */
	public static native void serverStopped();

	/**
	 * Reports the end of a server tick.
	 *
	 * @param nanos the nanoseconds from the start of the tick to its end
	 */
	public static native void tick(long nanos);

	/**
	 * Reports that a player joined.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @param name the name of the profile of the player
	 */
	public static native void playerJoined(String uuid, String name);

	/**
	 * Reports that a player left.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @param name the name of the profile of the player
	 */
	public static native void playerLeft(String uuid, String name);

	/**
	 * Reports a chat message of a player, which a plugin may cancel.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @param name the name of the profile of the player
	 * @param message the raw text of the message
	 * @return true to cancel the message
	 */
	public static native boolean chat(String uuid, String name, String message);

	/**
	 * Reports the death of a player.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @param name the name of the profile of the player
	 * @param cause the id of the damage type, such as minecraft:fall
	 */
	public static native void death(String uuid, String name, String cause);

	/**
	 * Reports the respawn of a player.
	 *
	 * @param uuid the UUID of the player in its usual form with dashes
	 * @param name the name of the profile of the player
	 * @param dimension the id of the dimension of the respawn, such as minecraft:overworld
	 */
	public static native void respawn(String uuid, String name, String dimension);

	/**
	 * Answers the names of the commands of the plugins, which the mod registers with Brigadier.
	 *
	 * @return the names, each without its slash
	 */
	public static native String[] commandNames();

	/**
	 * Runs a command of a plugin.
	 *
	 * @param name the name of the command, one of {@link #commandNames()}
	 * @param senderUuid the UUID of the player who runs it, or null for every other source
	 * @param senderName the name of the player who runs it, null for the console and RCON, or
	 *     the name of any other source, such as @ for a command block
	 * @param level the permission level of the source, from 0 to 4
	 * @param arguments everything after the name and one space, or the empty string
	 * @return the lines the sender sees
	 */
	public static native String[] command(String name, String senderUuid, String senderName,
			int level, String arguments);

	/**
	 * Answers the actions that Golem queued since the last call, oldest first.
	 *
	 * @return each action as its fields joined by the unit separator U+001F, kind first
	 */
	public static native String[] drain();
}
