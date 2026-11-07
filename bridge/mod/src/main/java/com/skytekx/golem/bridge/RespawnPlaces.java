// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * Where a player would respawn now, as PlayerList.respawn of Minecraft 1.20.1 resolves the
 * respawn point that the player holds, for the question respawnPlace of the game.
 *
 * The game checks the point only when the player respawns: a bed must stand there with room
 * beside it, an anchor must hold a charge, and a point that the command spawnpoint forced needs
 * room for the player alone. The question asks the same method of the game, for a player who
 * lives, so no charge of an anchor is used, turns the player towards a bed or an anchor as a
 * respawn does, and lifts them out of whatever they would stand in, as the game does after it.
 * It reads loaded chunks alone, so a point whose chunks around it are not loaded answers that,
 * and the question never loads a chunk.
 */
package com.skytekx.golem.bridge;

import java.util.Optional;
import net.minecraft.core.BlockPos;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerChunkCache;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/** Resolves the respawn point of a player as a respawn would, without a respawn. */
final class RespawnPlaces {
	/**
	 * How far from the block of the point the game reads blocks to resolve it: the other half of
	 * a bed, the places beside both halves where the player stands up, and the space of the
	 * player there.
	 */
	private static final int REACH = 3;

	private RespawnPlaces() {
	}

	/**
	 * Answers where the player would respawn now, in the form of the answer of respawnPlace.
	 *
	 * @param server the server of the player
	 * @param player the player
	 * @return the dimension, x, y, z and the yaw, the empty string for none, or unloaded
	 */
	static String find(MinecraftServer server, ServerPlayer player) {
		BlockPos point = player.getRespawnPosition();
		if (point == null) {
			return "";
		}
		ServerLevel level = server.getLevel(player.getRespawnDimension());
		if (level == null) {
			return "";
		}
		if (!loaded(level, point)) {
			return GameQueries.UNLOADED;
		}
		float angle = player.getRespawnAngle();
		Optional<Vec3> found = Player.findRespawnPositionAndUseSpawnBlock(level, point, angle,
				player.isRespawnForced(), true);
		if (found.isEmpty()) {
			return "";
		}
		Vec3 feet = found.get();
		BlockState state = level.getBlockState(point);
		float yaw = angle;
		if (state.is(BlockTags.BEDS) || state.is(Blocks.RESPAWN_ANCHOR)) {
			Vec3 facing = Vec3.atBottomCenterOf(point).subtract(feet).normalize();
			yaw = (float) Mth.wrapDegrees(Mth.atan2(facing.z, facing.x)
					* (double) (180.0F / (float) Math.PI) - 90.0D);
		}
		double y = feet.y;
		AABB box = EntityType.PLAYER.getDimensions().makeBoundingBox(feet.x, y, feet.z);
		while (!level.noCollision(box) && y < level.getMaxBuildHeight()) {
			y += 1.0D;
			box = box.move(0.0D, 1.0D, 0.0D);
		}
		return GameQueries.join(level.dimension().location().toString(),
				Double.toString(feet.x), Double.toString(y), Double.toString(feet.z),
				Float.toString(yaw));
	}

	/** Answers whether every chunk within the reach of the point is loaded. */
	private static boolean loaded(ServerLevel level, BlockPos point) {
		ServerChunkCache chunks = level.getChunkSource();
		for (int x = (point.getX() - REACH) >> 4; x <= (point.getX() + REACH) >> 4; x++) {
			for (int z = (point.getZ() - REACH) >> 4; z <= (point.getZ() + REACH) >> 4; z++) {
				if (chunks.getChunkNow(x, z) == null) {
					return false;
				}
			}
		}
		return true;
	}
}
