// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game as Golem may ask it, the implementation of the interface Game of the host jar that
 * the mod hands Golem with Host.connect.
 *
 * Golem asks only while a native of the server thread runs. Every answer still checks the thread
 * itself and answers null on another one, so a fault of Golem can never touch the game from the
 * outside. Each answer is its fields joined by the unit separator U+001F, as okf/bridge.md
 * specifies, and a field that holds the separator, such as a display name a mod made, has it
 * replaced by a space. A query that throws is logged through the guard and answers null.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Game;
import com.skytekx.golem.host.Host;
import java.util.List;
import java.util.UUID;
import java.util.function.Function;
import net.minecraft.Util;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceKey;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.item.ItemStack;
import net.minecraftforge.registries.ForgeRegistries;
import net.minecraftforge.server.ServerLifecycleHooks;

/** Answers what Golem asks the game, on the server thread. */
final class GameQueries implements Game {
	/** The unit separator between the fields of an answer. */
	private static final String SEPARATOR = "\u001F";

	/** The answer of safeLocation when the search met a chunk that is not loaded. */
	private static final String UNLOADED = "unloaded";

	private final Guard guard;

	private GameQueries(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Hands Golem the game, once the natives answer.
	 *
	 * <p>The call lives here and not in the class of the mod, since the verifier of the JVM loads
	 * the interface Game of the host jar for every class that passes an object as a Game, and on
	 * a stock Forge without golem-host.jar the class of the mod must load without it.
	 *
	 * @param guard the guard of every query
	 */
	static void connect(Guard guard) {
		Host.connect(new GameQueries(guard));
	}

	@Override
	public String[] players() {
		return guard.ask("Game.players", () -> {
			MinecraftServer server = server();
			if (server == null) {
				return null;
			}
			List<ServerPlayer> players = server.getPlayerList().getPlayers();
			String[] answer = new String[players.size()];
			for (int index = 0; index < answer.length; index++) {
				answer[index] = describe(server, players.get(index));
			}
			return answer;
		}, null);
	}

	@Override
	public String location(String uuid) {
		return ofPlayer("Game.location", uuid, player -> join(dimension(player.serverLevel()),
				Double.toString(player.getX()), Double.toString(player.getY()),
				Double.toString(player.getZ()), Float.toString(player.getYRot()),
				Float.toString(player.getXRot())));
	}

	@Override
	public String respawn(String uuid) {
		return ofPlayer("Game.respawn", uuid, player -> {
			BlockPos block = player.getRespawnPosition();
			if (block == null) {
				return "";
			}
			return join(player.getRespawnDimension().location().toString(),
					Integer.toString(block.getX()), Integer.toString(block.getY()),
					Integer.toString(block.getZ()));
		});
	}

	@Override
	public String spawn(String dimension) {
		return ofLevel("Game.spawn", dimension, level -> {
			BlockPos block = level.getSharedSpawnPos();
			return join(Integer.toString(block.getX()), Integer.toString(block.getY()),
					Integer.toString(block.getZ()), Float.toString(level.getSharedSpawnAngle()));
		});
	}

	@Override
	public String mainHand(String uuid) {
		return ofPlayer("Game.mainHand", uuid, player -> {
			ItemStack stack = player.getMainHandItem();
			if (stack.isEmpty()) {
				return "";
			}
			// Forge 47 deprecates the vanilla registry of items for its own.
			ResourceLocation id = ForgeRegistries.ITEMS.getKey(stack.getItem());
			return id == null ? null : join(id.toString(), Integer.toString(stack.getCount()));
		});
	}

	@Override
	public String heights(String dimension) {
		return ofLevel("Game.heights", dimension, level -> join(
				Integer.toString(level.getMinBuildHeight()),
				Integer.toString(level.getMaxBuildHeight())));
	}

	@Override
	public String safeLocation(String dimension, double x, double y, double z, int radius) {
		if (!Double.isFinite(x) || !Double.isFinite(y) || !Double.isFinite(z)) {
			return null;
		}
		return ofLevel("Game.safeLocation", dimension, level -> {
			SafeLocation.Answer answer = SafeLocation.find(level, x, y, z, radius);
			if (answer.unloaded()) {
				return UNLOADED;
			}
			BlockPos feet = answer.feet();
			if (feet == null) {
				return "";
			}
			return join(Double.toString(feet.getX() + 0.5D), Double.toString(feet.getY()),
					Double.toString(feet.getZ() + 0.5D));
		});
	}

	/**
	 * Answers the server when the caller runs on its thread, and null on any other thread, since
	 * the game may be read on its own thread alone.
	 */
	private static MinecraftServer server() {
		MinecraftServer server = ServerLifecycleHooks.getCurrentServer();
		return server != null && server.isSameThread() ? server : null;
	}

	/** Answers a query about one online player, or null when they are not online. */
	private String ofPlayer(String name, String uuid, Function<ServerPlayer, String> query) {
		return guard.ask(name, () -> {
			MinecraftServer server = server();
			UUID id = uuid(uuid);
			if (server == null || id == null) {
				return null;
			}
			ServerPlayer player = server.getPlayerList().getPlayer(id);
			return player == null ? null : query.apply(player);
		}, null);
	}

	/** Answers a query about one dimension, or null when the server has no such dimension. */
	private String ofLevel(String name, String dimension, Function<ServerLevel, String> query) {
		return guard.ask(name, () -> {
			MinecraftServer server = server();
			ResourceLocation id = dimension == null ? null : ResourceLocation.tryParse(dimension);
			if (server == null || id == null) {
				return null;
			}
			ServerLevel level = server.getLevel(ResourceKey.create(Registries.DIMENSION, id));
			return level == null ? null : query.apply(level);
		}, null);
	}

	/** Answers a player as the fields of Game.players. */
	private static String describe(MinecraftServer server, ServerPlayer player) {
		Component listName = player.getTabListDisplayName();
		long idle = Math.max(0L, Util.getMillis() - player.getLastActionTime());
		return join(player.getStringUUID(), player.getGameProfile().getName(),
				player.getDisplayName().getString(), Integer.toString(player.latency),
				player.gameMode.getGameModeForPlayer().getName(),
				Integer.toString(server.getProfilePermissions(player.getGameProfile())),
				Long.toString(idle), listName == null ? "" : listName.getString());
	}

	private static String dimension(ServerLevel level) {
		return level.dimension().location().toString();
	}

	/** Answers the UUID of a text in its usual form, or null for text that is no UUID. */
	private static UUID uuid(String text) {
		if (text == null) {
			return null;
		}
		try {
			return UUID.fromString(text);
		} catch (IllegalArgumentException notUuid) {
			return null;
		}
	}

	private static String join(String... fields) {
		StringBuilder answer = new StringBuilder();
		for (int index = 0; index < fields.length; index++) {
			if (index > 0) {
				answer.append(SEPARATOR);
			}
			answer.append(fields[index].replace(SEPARATOR, " "));
		}
		return answer.toString();
	}
}
