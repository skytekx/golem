// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The actions that Golem queues for the game, run on the server thread at the end of a tick.
 *
 * Each action is one string, its fields joined by the unit separator U+001F, the kind first, as
 * okf/bridge.md specifies. An action of a kind the bridge does not know, or with the wrong number
 * of fields, comes from a Golem that is newer or broken. The bridge logs it once and skips it,
 * since the same fault would otherwise repeat on every tick.
 */
package com.skytekx.golem.bridge;

import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.network.chat.Component;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;

/** Runs the actions that the natives of Golem answer from Host.drain. */
final class Actions {
	/** The unit separator between the fields of an action. */
	private static final String SEPARATOR = "\u001F";

	private final Guard guard;

	/** The faults already logged, each as the kind and the number of fields. */
	private final Set<String> logged = new HashSet<>();

	Actions(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Runs the actions in their order, each on its own, so a failing one stops no other.
	 *
	 * @param server the server whose tick ended
	 * @param queued the actions, oldest first, or null for none
	 */
	void run(MinecraftServer server, String[] queued) {
		if (queued == null) {
			return;
		}
		for (String action : queued) {
			if (action != null) {
				String[] fields = action.split(SEPARATOR, -1);
				guard.run("The action " + fields[0], () -> perform(server, fields));
			}
		}
	}

	private void perform(MinecraftServer server, String[] fields) {
		switch (fields[0]) {
		case "command" -> command(server, fields);
		case "message" -> message(server, fields);
		case "broadcast" -> broadcast(server, fields);
		default -> skip(fields, "of a kind the bridge does not know");
		}
	}

	/** Runs a command as the console, so the stop path is the one of an operator. */
	private void command(MinecraftServer server, String[] fields) {
		if (fits(fields, 2)) {
			CommandSourceStack console = server.createCommandSourceStack();
			server.getCommands().performPrefixedCommand(console, fields[1]);
		}
	}

	/** Sends a system message to one player, if they are online. */
	private void message(MinecraftServer server, String[] fields) {
		if (!fits(fields, 3)) {
			return;
		}
		UUID uuid = UUID.fromString(fields[1]);
		ServerPlayer player = server.getPlayerList().getPlayer(uuid);
		if (player != null) {
			player.sendSystemMessage(Component.literal(fields[2]));
		}
	}

	/** Sends a system message to every player and to the console. */
	private void broadcast(MinecraftServer server, String[] fields) {
		if (fits(fields, 2)) {
			Component text = Component.literal(fields[1]);
			server.getPlayerList().broadcastSystemMessage(text, false);
		}
	}

	private boolean fits(String[] fields, int count) {
		if (fields.length == count) {
			return true;
		}
		skip(fields, "with " + (fields.length - 1) + " fields instead of " + (count - 1));
		return false;
	}

	private void skip(String[] fields, String fault) {
		String key = fields[0] + "/" + fields.length;
		if (logged.add(key)) {
			GolemBridge.LOG.warn("Golem queued an action {} {}, which the bridge skips",
					fields[0], fault);
		}
	}
}
