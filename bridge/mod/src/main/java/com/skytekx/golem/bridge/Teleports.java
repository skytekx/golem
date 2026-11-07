// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The teleports that Golem queues, which the bridge runs on the server thread at the end of a
 * tick and reports to Golem with their ids when they end: teleported, with where the player was
 * and where they are, or teleportFailed, with the reason.
 *
 * A teleport moves the player as the command tp does. One with the radius of a search for a safe
 * place first has the chunks of the search loaded by a ticket of its own, over as many ticks as
 * the game takes for them, and then searches near its target as the question safeLocation does,
 * so a far home or warp whose chunks no player keeps loaded still finds its safe place, while the
 * tick never waits for a chunk. The teleports of one player run in the order Golem queued them,
 * so a teleport waits behind an earlier one of the same player that waits for its chunks, and
 * the reports of one player come in that order too.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Host;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerChunkCache;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.level.TicketType;
import net.minecraft.util.Mth;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.Level;
import net.minecraftforge.event.ForgeEventFactory;
import net.minecraftforge.event.entity.EntityTeleportEvent;
import net.minecraftforge.event.entity.player.PlayerEvent;
import net.minecraftforge.eventbus.api.SubscribeEvent;

/** Runs the teleports of Golem and reports how each one ended. */
public final class Teleports {
	/** How many ticks a teleport waits for the chunks of its search, 30 seconds at 20 TPS. */
	static final int WAIT_TICKS = 600;

	/**
	 * The reason of a teleport of a player who is not online, who left while it waited, or whom a
	 * mod made leave inside its move, before the game moved them.
	 */
	static final String OFFLINE = "offline";

	/** The reason of a teleport into a dimension that the server does not have. */
	static final String DIMENSION = "dimension";

	/** The reason of a teleport whose fields do not read or whose place lies outside the world. */
	static final String PLACE = "place";

	/** The reason of a teleport that a mod cancelled. */
	static final String CANCELLED = "cancelled";

	/** The reason of a teleport whose search found no safe place. */
	static final String UNSAFE = "unsafe";

	/** The reason of a teleport whose chunks did not load while it waited. */
	static final String UNLOADED = "unloaded";

	/**
	 * The reason of a teleport that waited when the server stopped, or that never ran, since an
	 * error ended the server before its turn.
	 */
	static final String STOPPED = "stopped";

	/** The reason of a teleport that threw. */
	static final String ERROR = "error";

	/**
	 * The ticket that has the chunks of a search loaded while its teleport waits, one per
	 * teleport by its id, so two teleports near each other never take each other's ticket away.
	 * It runs out by itself a second after the longest wait, should a fault keep the bridge from
	 * removing it.
	 */
	private static final TicketType<Long> TICKET = TicketType.create("golem_teleport",
			Long::compareTo, WAIT_TICKS + 20);

	private final Guard guard;

	/** The faults already logged, each by a key that names the fault. */
	private final Set<String> logged = new HashSet<>();

	/** The teleports that wait, by player, each queue in the order Golem queued them. */
	private final Map<UUID, ArrayDeque<Request>> waiting = new LinkedHashMap<>();

	Teleports(Guard guard) {
		this.guard = guard;
	}

	/** A teleport that Golem queued, as its action reads. */
	private static final class Request {
		final long id;
		final String uuidText;
		final UUID uuid;
		final String dimension;
		/** The reason when the action does not read, or null. */
		String fault;
		double x;
		double y;
		double z;
		float yaw;
		float pitch;
		/** The radius of the search, or -1 for the place as it is. */
		int radius = -1;
		/** The level of the tickets, once the teleport added them. */
		ServerLevel ticketed;
		/** The chunks of the tickets. */
		final List<ChunkPos> chunks = new ArrayList<>();
		/** The ticks the teleport waited for its chunks. */
		int waited;

		Request(long id, String uuidText, UUID uuid, String dimension) {
			this.id = id;
			this.uuidText = uuidText;
			this.uuid = uuid;
			this.dimension = dimension;
		}
	}

	/**
	 * Takes the action teleport with its fields after the kind: the id, the UUID, the dimension,
	 * x, y, z, the yaw, the pitch and the radius of the search, empty for none. It runs at once
	 * unless the player has a teleport that waits, or it waits for the chunks of its search.
	 *
	 * @param server the server whose tick ended
	 * @param fields the ten fields of the action, the kind first
	 */
	void queue(MinecraftServer server, String[] fields) {
		long id;
		try {
			id = Long.parseLong(fields[1]);
		} catch (NumberFormatException notNumber) {
			skip("teleport/id", "Golem queued a teleport whose id does not read, so the bridge"
					+ " skips it and reports nothing");
			return;
		}
		Request request = new Request(id, fields[2], uuid(fields[2]), fields[3]);
		read(request, fields);
		if (request.uuid == null) {
			report(request, OFFLINE);
			return;
		}
		ArrayDeque<Request> queue = waiting.get(request.uuid);
		if (queue != null) {
			queue.add(request);
		} else if (!attempt(server, request)) {
			queue = new ArrayDeque<>();
			queue.add(request);
			waiting.put(request.uuid, queue);
		}
	}

	/** Reads the place and the radius of a teleport, or notes why they do not read. */
	private void read(Request request, String[] fields) {
		try {
			request.x = Double.parseDouble(fields[4]);
			request.y = Double.parseDouble(fields[5]);
			request.z = Double.parseDouble(fields[6]);
			request.yaw = Float.parseFloat(fields[7]);
			request.pitch = Float.parseFloat(fields[8]);
			if (!fields[9].isEmpty()) {
				request.radius = Integer.parseInt(fields[9]);
			}
		} catch (NumberFormatException notNumber) {
			skip("teleport/number", "Golem queued a teleport whose numbers do not read, so the"
					+ " bridge skips it");
			request.fault = PLACE;
			return;
		}
		if (!finite(request.x, request.y, request.z, request.yaw, request.pitch)) {
			skip("teleport/finite", "Golem queued a teleport to a place that is no finite"
					+ " number, so the bridge skips it");
			request.fault = PLACE;
		} else if (!fields[9].isEmpty()
				&& (request.radius < 0 || request.radius > SafeLocation.MAX_RADIUS)) {
			skip("teleport/radius", "Golem queued a teleport whose radius lies outside 0 to "
					+ SafeLocation.MAX_RADIUS + ", so the bridge skips it");
			request.fault = PLACE;
		}
	}

	/**
	 * Goes on with the teleports that wait, at the end of every tick before the new actions, in
	 * the order of their players, each player's in the order Golem queued them.
	 *
	 * @param server the server whose tick ended
	 */
	void tick(MinecraftServer server) {
		for (UUID player : new ArrayList<>(waiting.keySet())) {
			ArrayDeque<Request> queue = waiting.get(player);
			while (queue != null && !queue.isEmpty()) {
				// The head leaves its queue for its attempt: a move may make a mod disconnect the
				// player, which ends the teleports that wait on the spot, and the one that runs
				// ends by itself.
				Request head = queue.poll();
				if (attempt(server, head)) {
					continue;
				}
				if (waiting.get(player) == queue) {
					queue.addFirst(head);
				} else {
					// The player left while the attempt ran, and the queue with them.
					ended(head, OFFLINE);
				}
				break;
			}
			if (queue != null && queue.isEmpty()) {
				waiting.remove(player, queue);
			}
		}
	}

	/**
	 * Ends every teleport that waits with the reason stopped, as the server stops.
	 *
	 * @param server the server that stops
	 */
	void stopping(MinecraftServer server) {
		List<Request> rest = new ArrayList<>();
		waiting.values().forEach(rest::addAll);
		waiting.clear();
		for (Request request : rest) {
			ended(request, STOPPED);
		}
	}

	/**
	 * Ends a teleport that the bridge never ran with the reason stopped: an action of a drain in
	 * which an error that ends the server flew out of an action before its turn. No tick follows
	 * to run it and Golem holds it no longer, so nothing else would end it. An action of another
	 * kind, and one that does not hold the fields of a teleport or an id that reads, is none.
	 *
	 * @param fields the fields of an action, the kind first
	 */
	void neverRan(String[] fields) {
		if (fields.length != 10 || !fields[0].equals("teleport")) {
			return;
		}
		long id;
		try {
			id = Long.parseLong(fields[1]);
		} catch (NumberFormatException notNumber) {
			return;
		}
		report(new Request(id, fields[2], null, fields[3]), STOPPED);
	}

	/**
	 * Ends the teleports of a player who left with the reason offline, before Golem hears that
	 * the player left.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void left(PlayerEvent.PlayerLoggedOutEvent event) {
		guard.run("The teleports of a player who left", () -> {
			ArrayDeque<Request> queue = waiting.remove(event.getEntity().getUUID());
			if (queue != null) {
				List<Request> theirs = new ArrayList<>(queue);
				queue.clear();
				for (Request request : theirs) {
					ended(request, OFFLINE);
				}
			}
		});
	}

	/**
	 * Runs a teleport as far as it gets now, and answers whether it ended, which it reported, or
	 * waits for the chunks of its search. A teleport that throws ends with the reason error. That
	 * holds for what the guard catches, behind which the game goes on, and for what it lets pass,
	 * an error of the JVM that ends the server: the teleport that runs stands in no queue, so
	 * nothing would end it behind such an error, and it ends here before the error goes on.
	 */
	private boolean attempt(MinecraftServer server, Request request) {
		Boolean ended;
		try {
			ended = guard.ask("The action teleport", () -> step(server, request), null);
		} catch (Error fatal) {
			release(request);
			report(request, ERROR);
			throw fatal;
		}
		if (ended == null) {
			release(request);
			report(request, ERROR);
			return true;
		}
		return ended;
	}

	private boolean step(MinecraftServer server, Request request) {
		if (request.fault != null) {
			return ended(request, request.fault);
		}
		ServerLevel level = level(server, request.dimension);
		if (level == null) {
			skip("teleport/" + request.dimension, "Golem queued a teleport into the dimension "
					+ request.dimension + ", which the server does not have, so the bridge"
					+ " skips it");
			return ended(request, DIMENSION);
		}
		ServerPlayer player = server.getPlayerList().getPlayer(request.uuid);
		if (player == null) {
			return ended(request, OFFLINE);
		}
		if (request.radius < 0) {
			return move(player, level, request, request.x, request.y, request.z);
		}
		if (!loaded(level, request)) {
			if (request.ticketed == null) {
				hold(level, request);
			}
			if (++request.waited > WAIT_TICKS) {
				return ended(request, UNLOADED);
			}
			return false;
		}
		SafeLocation.Answer answer = SafeLocation.find(level, request.x, request.y, request.z,
				request.radius);
		release(request);
		if (answer.unloaded()) {
			return ended(request, UNLOADED);
		}
		BlockPos feet = answer.feet();
		if (feet == null) {
			return ended(request, UNSAFE);
		}
		return move(player, level, request, feet.getX() + 0.5D, feet.getY(), feet.getZ() + 0.5D);
	}

	/**
	 * Moves the player as TeleportCommand.performTeleport of Minecraft 1.20.1 does: the event of
	 * Forge first, which a mod may cancel or whose target it may change, then the bounds of the
	 * world, then the teleportTo that keeps the chunk loaded with the ticket post_teleport, lets
	 * a rider dismount, wakes a sleeper and keeps the camera of a spectator within one level. A
	 * mod may cancel the travel into another dimension, and then the player stays. A player whom
	 * a listener of the event of Forge made leave is not moved.
	 */
	private boolean move(ServerPlayer player, ServerLevel level, Request request, double x,
			double y, double z) {
		String from = GameQueries.place(player);
		EntityTeleportEvent.TeleportCommand event =
				ForgeEventFactory.onEntityTeleportCommand(player, x, y, z);
		if (level.getServer().getPlayerList().getPlayer(request.uuid) != player) {
			// A listener of the event made the player leave, so the game holds nobody to move.
			return ended(request, OFFLINE);
		}
		if (event.isCanceled()) {
			skip("teleport/cancelled", "A mod cancelled a teleport that Golem queued");
			return ended(request, CANCELLED);
		}
		double targetX = event.getTargetX();
		double targetY = event.getTargetY();
		double targetZ = event.getTargetZ();
		if (!finite(targetX, targetY, targetZ, request.yaw, request.pitch)
				|| !Level.isInSpawnableBounds(BlockPos.containing(targetX, targetY, targetZ))) {
			skip("teleport/bounds", "Golem queued a teleport to a place outside the bounds of the"
					+ " world, so the bridge skips it");
			return ended(request, PLACE);
		}
		float facing = Mth.wrapDegrees(request.yaw);
		float looking = Mth.clamp(Mth.wrapDegrees(request.pitch), -90.0F, 90.0F);
		player.teleportTo(level, targetX, targetY, targetZ, Set.of(), facing, looking);
		if (player.serverLevel() != level) {
			return ended(request, CANCELLED);
		}
		if (!player.isFallFlying()) {
			player.setDeltaMovement(player.getDeltaMovement().multiply(1.0D, 0.0D, 1.0D));
			player.setOnGround(true);
		}
		String to = GameQueries.place(player);
		guard.run("Host.teleported", () -> Host.teleported(request.id, request.uuidText, from, to));
		return true;
	}

	/** Answers whether every chunk that the search of a teleport may read is loaded. */
	private static boolean loaded(ServerLevel level, Request request) {
		ServerChunkCache chunks = level.getChunkSource();
		for (ChunkPos chunk : area(request)) {
			if (chunks.getChunkNow(chunk.x, chunk.z) == null) {
				return false;
			}
		}
		return true;
	}

	/** Answers the chunks of the columns within the radius of the block of the target. */
	private static List<ChunkPos> area(Request request) {
		BlockPos target = BlockPos.containing(request.x, request.y, request.z);
		List<ChunkPos> area = new ArrayList<>();
		for (int x = (target.getX() - request.radius) >> 4;
				x <= (target.getX() + request.radius) >> 4; x++) {
			for (int z = (target.getZ() - request.radius) >> 4;
					z <= (target.getZ() + request.radius) >> 4; z++) {
				area.add(new ChunkPos(x, z));
			}
		}
		return area;
	}

	/** Adds the tickets that load the chunks of the search of a teleport. */
	private static void hold(ServerLevel level, Request request) {
		request.ticketed = level;
		for (ChunkPos chunk : area(request)) {
			level.getChunkSource().addRegionTicket(TICKET, chunk, 0, request.id);
			request.chunks.add(chunk);
		}
	}

	/**
	 * Removes the tickets of a teleport, if it added any, behind the guard, so a fault of a ticket
	 * keeps no teleport from its report.
	 */
	private void release(Request request) {
		ServerLevel level = request.ticketed;
		if (level != null) {
			List<ChunkPos> chunks = List.copyOf(request.chunks);
			request.ticketed = null;
			request.chunks.clear();
			guard.run("The tickets of a teleport", () -> {
				for (ChunkPos chunk : chunks) {
					level.getChunkSource().removeRegionTicket(TICKET, chunk, 0, request.id);
				}
			});
		}
	}

	/** Ends a teleport without a move, and answers that it ended. */
	private boolean ended(Request request, String reason) {
		release(request);
		report(request, reason);
		return true;
	}

	private void report(Request request, String reason) {
		guard.run("Host.teleportFailed", () -> {
			Host.teleportFailed(request.id, request.uuidText, reason);
		});
	}

	/** Answers the level of a dimension id, or null when the server has no such dimension. */
	private static ServerLevel level(MinecraftServer server, String dimension) {
		ResourceLocation id = ResourceLocation.tryParse(dimension);
		if (id == null) {
			return null;
		}
		return server.getLevel(ResourceKey.create(Registries.DIMENSION, id));
	}

	/** Answers the UUID of a text in its usual form, or null for text that is no UUID. */
	private static UUID uuid(String text) {
		try {
			return UUID.fromString(text);
		} catch (IllegalArgumentException notUuid) {
			return null;
		}
	}

	private static boolean finite(double x, double y, double z, float yaw, float pitch) {
		return Double.isFinite(x) && Double.isFinite(y) && Double.isFinite(z)
				&& Float.isFinite(yaw) && Float.isFinite(pitch);
	}

	private void skip(String key, String line) {
		if (logged.add(key)) {
			GolemBridge.LOG.warn(line);
		}
	}
}
