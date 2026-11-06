// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The actions that Golem queues for the game, run on the server thread at the end of a tick.
 *
 * Each action is one string, its fields joined by the unit separator U+001F, the kind first, as
 * okf/bridge.md specifies. An action of a kind the bridge does not know, or with the wrong number
 * of fields, comes from a Golem that is newer or broken. The bridge logs it once and skips it,
 * since the same fault would otherwise repeat on every tick. So does a teleport into a dimension
 * the server does not have or to a place that is no finite number.
 */
package com.skytekx.golem.bridge;

import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceKey;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.level.TicketType;
import net.minecraft.util.Mth;
import net.minecraft.world.level.ChunkPos;

/** Runs the actions that the natives of Golem answer from Host.drain. */
final class Actions {
	/** The unit separator between the fields of an action. */
	private static final String SEPARATOR = "\u001F";

	private final Guard guard;
	private final ListNames names;

	/** The faults already logged, each by a key that names the fault. */
	private final Set<String> logged = new HashSet<>();

	Actions(Guard guard, ListNames names) {
		this.guard = guard;
		this.names = names;
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
		case "teleport" -> teleport(server, fields);
		case "listName" -> listName(server, fields);
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

	/**
	 * Teleports a player, if they are online, as the command tp does: the chunk of the place
	 * stays loaded for a moment, a rider dismounts, a sleeper wakes, the fall stops and the place
	 * may lie in another dimension.
	 */
	private void teleport(MinecraftServer server, String[] fields) {
		if (!fits(fields, 8)) {
			return;
		}
		ServerPlayer player = server.getPlayerList().getPlayer(UUID.fromString(fields[1]));
		if (player == null) {
			return;
		}
		ServerLevel level = level(server, fields[2]);
		double x = Double.parseDouble(fields[3]);
		double y = Double.parseDouble(fields[4]);
		double z = Double.parseDouble(fields[5]);
		float yaw = Float.parseFloat(fields[6]);
		float pitch = Float.parseFloat(fields[7]);
		if (level == null) {
			skip("teleport/" + fields[2], "Golem queued a teleport into the dimension " + fields[2]
					+ ", which the server does not have, so the bridge skips it");
		} else if (!finite(x, y, z, yaw, pitch)) {
			skip("teleport/finite", "Golem queued a teleport to a place that is no finite"
					+ " number, so the bridge skips it");
		} else {
			move(player, level, x, y, z, yaw, pitch);
		}
	}

	private static void move(ServerPlayer player, ServerLevel level, double x, double y, double z,
			float yaw, float pitch) {
		ChunkPos chunk = new ChunkPos(BlockPos.containing(x, y, z));
		level.getChunkSource().addRegionTicket(TicketType.POST_TELEPORT, chunk, 1,
				player.getId());
		if (player.isSleeping()) {
			player.stopSleepInBed(true, true);
		}
		float facing = Mth.wrapDegrees(yaw);
		player.teleportTo(level, x, y, z, facing, Mth.clamp(Mth.wrapDegrees(pitch), -90.0F,
				90.0F));
		player.setYHeadRot(facing);
		if (!player.isFallFlying()) {
			player.setDeltaMovement(player.getDeltaMovement().multiply(1.0D, 0.0D, 1.0D));
			player.setOnGround(true);
		}
	}

	/** Answers the level of a dimension id, or null when the server has no such dimension. */
	private static ServerLevel level(MinecraftServer server, String dimension) {
		ResourceLocation id = ResourceLocation.tryParse(dimension);
		if (id == null) {
			return null;
		}
		return server.getLevel(ResourceKey.create(Registries.DIMENSION, id));
	}

	private static boolean finite(double x, double y, double z, float yaw, float pitch) {
		return Double.isFinite(x) && Double.isFinite(y) && Double.isFinite(z)
				&& Float.isFinite(yaw) && Float.isFinite(pitch);
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
