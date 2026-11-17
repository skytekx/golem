// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The actions that Golem queues for the game, run on the server thread at the end of a tick.
 *
 * Each action is one string, its fields joined by the unit separator U+001F, the kind first, as
 * okf/bridge.md specifies. An action of a kind the bridge does not know, or with the wrong number
 * of fields, comes from a Golem that is newer or broken. The bridge logs it once and skips it,
 * since the same fault would otherwise repeat on every tick. A teleport goes to Teleports, which
 * reports to Golem how it ended.
 */
package com.skytekx.golem.bridge;

import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import net.minecraft.commands.CommandSource;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.network.chat.Component;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerPlayer;

/** Runs the actions that the natives of Golem answer from Host.drain. */
final class Actions {
	/** The unit separator between the fields of an action. */
	private static final String SEPARATOR = "\u001F";

	private final Guard guard;
	private final ListNames names;
	private final Teleports teleports;

	/** The faults already logged, each by a key that names the fault. */
	private final Set<String> logged = new HashSet<>();

	Actions(Guard guard, ListNames names, Teleports teleports) {
		this.guard = guard;
		this.names = names;
		this.teleports = teleports;
	}

	/**
	 * Goes on with the teleports that wait for their chunks, and then runs the actions in their
	 * order, each on its own, so a failing one stops no other. What the guard lets pass, an error
	 * of the JVM or any other throwable that is no exception, ends the server and leaves the
	 * actions behind it without a tick to run them, and Golem holds them no longer. So the
	 * teleports among them end with the reason stopped before the throwable goes on.
	 *
	 * @param server the server whose tick ended
	 * @param queued the actions, oldest first, or null for none
	 */
	void run(MinecraftServer server, String[] queued) {
		int count = queued == null ? 0 : queued.length;
		int next = 0;
		try {
			guard.run("The teleports that wait", () -> teleports.tick(server));
			while (next < count) {
				String action = queued[next++];
				if (action != null) {
					String[] fields = action.split(SEPARATOR, -1);
					guard.run("The action " + fields[0], () -> perform(server, fields));
				}
			}
		} catch (Throwable fatal) {
			while (next < count) {
				String action = queued[next++];
				if (action != null) {
					teleports.neverRan(action.split(SEPARATOR, -1));
				}
			}
			throw fatal;
		}
	}

	private void perform(MinecraftServer server, String[] fields) {
		switch (fields[0]) {
		case "command" -> command(server, fields);
		case "message" -> message(server, fields);
		case "broadcast" -> broadcast(server, fields);
		case "styledMessage" -> styledMessage(server, fields);
		case "styledBroadcast" -> styledBroadcast(server, fields);
		case "teleport" -> teleport(server, fields);
		case "listName" -> listName(server, fields);
		default -> skip(fields, "of a kind the bridge does not know");
		}
	}

	/**
	 * Runs a command as the console, so the stop path is the one of an operator. What the command
	 * answers reaches the console and the log of the server but no operator in the game, so a
	 * plugin that acts through commands does not show what it does for every player to every
	 * operator, as a command typed at the console does under broadcast-console-to-ops.
	 */
	private void command(MinecraftServer server, String[] fields) {
		if (fits(fields, 2)) {
			CommandSourceStack console = server.createCommandSourceStack()
					.withSource(new QuietConsole(server));
			server.getCommands().performPrefixedCommand(console, fields[1]);
		}
	}

	/** The console as the source of a command, which tells no operator in the game. */
	private static final class QuietConsole implements CommandSource {
		private final MinecraftServer server;

		QuietConsole(MinecraftServer server) {
			this.server = server;
		}

		@Override
		public void sendSystemMessage(Component message) {
			server.sendSystemMessage(message);
		}

		@Override
		public boolean acceptsSuccess() {
			return true;
		}

		@Override
		public boolean acceptsFailure() {
			return true;
		}

		@Override
		public boolean shouldInformAdmins() {
			return false;
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
			player.sendSystemMessage(Links.text(fields[2]));
		}
	}

	/** Sends a system message to every player and to the console. */
	private void broadcast(MinecraftServer server, String[] fields) {
		if (fits(fields, 2)) {
			Component text = Links.text(fields[1]);
			server.getPlayerList().broadcastSystemMessage(text, false);
		}
	}

	/**
	 * Sends a styled text to one player as a system message, if they are online. The text is the
	 * JSON of a component that Golem wrote, which the game reads into a Component as it reads
	 * the text of tellraw, and no link is added to it: a styled text links where its plugin set
	 * a click.
	 */
	private void styledMessage(MinecraftServer server, String[] fields) {
		if (!fits(fields, 3)) {
			return;
		}
		ServerPlayer player = server.getPlayerList().getPlayer(UUID.fromString(fields[1]));
		Component text = styled(fields[0], fields[2]);
		if (player != null && text != null) {
			player.sendSystemMessage(text);
		}
	}

	/**
	 * Sends a styled text to the console and to every player online but the hidden ones, whose
	 * UUIDs the action joins by commas, as PlayerList.broadcastSystemMessage of the game sends a
	 * system message to all. A part that is no UUID names nobody.
	 */
	private void styledBroadcast(MinecraftServer server, String[] fields) {
		if (!fits(fields, 3)) {
			return;
		}
		Component text = styled(fields[0], fields[2]);
		if (text == null) {
			return;
		}
		Set<UUID> hidden = new HashSet<>();
		for (String part : fields[1].split(",")) {
			try {
				hidden.add(UUID.fromString(part));
			} catch (IllegalArgumentException notUuid) {
				// A part that is no UUID names nobody online.
			}
		}
		server.sendSystemMessage(text);
		for (ServerPlayer player : server.getPlayerList().getPlayers()) {
			if (!hidden.contains(player.getUUID())) {
				player.sendSystemMessage(text);
			}
		}
	}

	/** Answers the component of the JSON of a styled text, or null after one warning. */
	private Component styled(String kind, String json) {
		Component text;
		try {
			text = Component.Serializer.fromJson(json);
		} catch (RuntimeException notJson) {
			text = null;
		}
		if (text == null) {
			skip(kind + "/json", "Golem queued an action " + kind + " whose text does not read,"
					+ " which the bridge skips");
		}
		return text;
	}

	/**
	 * Hands a teleport to the teleports, which run it as the command tp does and report how it
	 * ended, see {@link Teleports}.
	 */
	private void teleport(MinecraftServer server, String[] fields) {
		if (fits(fields, 10)) {
			teleports.queue(server, fields);
		}
	}

	/**
	 * Ends the teleports that wait, as the server stops.
	 *
	 * @param server the server that stops
	 */
	void stopping(MinecraftServer server) {
		guard.run("The teleports that wait", () -> teleports.stopping(server));
	}

	/** Sets or clears the name of a player in the player list, if they are online. */
	private void listName(MinecraftServer server, String[] fields) {
		if (!fits(fields, 3)) {
			return;
		}
		UUID uuid = UUID.fromString(fields[1]);
		ServerPlayer player = server.getPlayerList().getPlayer(uuid);
		if (player != null) {
			names.set(uuid, fields[2]);
			player.refreshTabListName();
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
		skip(fields[0] + "/" + fields.length, "Golem queued an action " + fields[0] + " " + fault
				+ ", which the bridge skips");
	}

	private void skip(String key, String line) {
		if (logged.add(key)) {
			GolemBridge.LOG.warn(line);
		}
	}
}
