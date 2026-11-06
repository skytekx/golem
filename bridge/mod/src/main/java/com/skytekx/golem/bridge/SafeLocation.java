// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The search for a place near a target where a player stands safely, for a teleport of Golem.
 *
 * A place is safe when the block below the feet carries a player on its top face and hurts no
 * one standing on it, the blocks of the feet and the head let a player through and hold no
 * fluid and nothing that hurts, the ground lies above the void and the head below the top of the
 * world, and the place lies inside the world border. The search looks at the blocks within a
 * sphere around the block of the target, the nearest first, and at the same distance the higher
 * first, then the lower x, then the lower z, so the same world always gives the same answer.
 *
 * The search reads the blocks of loaded chunks alone. Loading a chunk on the server thread waits
 * for the workers of the server, which may wait for the handler of Golem that asks, so a question
 * that loaded a chunk could stall the tick or hang the server. At the first place in its order
 * whose chunk is not loaded, before it found a safe one, the search stops and answers that the
 * chunk is not loaded, so an answer with a place is always the answer of the whole search. The
 * radius is at most 8 blocks, so a search reads at most 2109 places.
 */
package com.skytekx.golem.bridge;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerChunkCache;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.tags.BlockTags;
import net.minecraft.tags.FluidTags;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;

/** Finds a safe place to stand near a target. */
final class SafeLocation {
	/** The largest radius of a search, in blocks. */
	static final int MAX_RADIUS = 8;

	/** Every offset within the largest radius, as dx, dy, dz and the square of the distance. */
	private static final int[][] OFFSETS = offsets();

	private SafeLocation() {
	}

	/**
	 * What a search answers.
	 *
	 * @param feet the block of the feet of the nearest safe place, or null for none
	 * @param unloaded whether the search met a chunk that is not loaded before it found a place
	 */
	record Answer(BlockPos feet, boolean unloaded) {
		/** No place within the radius is safe. */
		static final Answer NONE = new Answer(null, false);

		/** The search met a chunk that is not loaded before it found a safe place. */
		static final Answer UNLOADED = new Answer(null, true);
	}

	/**
	 * Answers the block of the feet of the nearest safe place near a target.
	 *
	 * @param level the level of the target
	 * @param x the x coordinate of the target
	 * @param y the y coordinate of the target
	 * @param z the z coordinate of the target
	 * @param radius how far to look, in blocks, held to 0 to {@link #MAX_RADIUS}
	 * @return the block of the feet, none, or that a chunk the search must read is not loaded
	 */
	static Answer find(ServerLevel level, double x, double y, double z, int radius) {
		int limit = Math.max(0, Math.min(radius, MAX_RADIUS));
		BlockPos target = BlockPos.containing(x, y, z);
		ServerChunkCache chunks = level.getChunkSource();
		BlockPos.MutableBlockPos feet = new BlockPos.MutableBlockPos();
		for (int[] offset : OFFSETS) {
			if (offset[3] > limit * limit) {
				break;
			}
			feet.set(target.getX() + offset[0], target.getY() + offset[1],
					target.getZ() + offset[2]);
			if (!inside(level, feet)) {
				continue;
			}
			// The ground, the feet and the head share one column and so one chunk.
			if (chunks.getChunkNow(feet.getX() >> 4, feet.getZ() >> 4) == null) {
				return Answer.UNLOADED;
			}
			if (safe(level, feet)) {
				return new Answer(feet.immutable(), false);
			}
		}
		return Answer.NONE;
	}

	/** Answers the offsets of the sphere of the largest radius in the order of the search. */
	private static int[][] offsets() {
		List<int[]> offsets = new ArrayList<>();
		for (int dx = -MAX_RADIUS; dx <= MAX_RADIUS; dx++) {
			for (int dy = -MAX_RADIUS; dy <= MAX_RADIUS; dy++) {
				for (int dz = -MAX_RADIUS; dz <= MAX_RADIUS; dz++) {
					int distance = dx * dx + dy * dy + dz * dz;
					if (distance <= MAX_RADIUS * MAX_RADIUS) {
						offsets.add(new int[] {dx, dy, dz, distance});
					}
				}
			}
		}
		offsets.sort(Comparator.<int[]>comparingInt(offset -> offset[3])
				.thenComparingInt(offset -> -offset[1])
				.thenComparingInt(offset -> offset[0])
				.thenComparingInt(offset -> offset[2]));
		return offsets.toArray(new int[0][]);
	}

	/** Answers whether the ground and the head lie within the heights and the world border. */
	private static boolean inside(ServerLevel level, BlockPos feet) {
		return feet.getY() - 1 >= level.getMinBuildHeight()
				&& feet.getY() + 1 < level.getMaxBuildHeight()
				&& level.getWorldBorder().isWithinBounds(feet);
	}

	/** Answers whether a player stands safely with the feet in a block of a loaded chunk. */
	private static boolean safe(ServerLevel level, BlockPos feet) {
		BlockPos below = feet.below();
		BlockState ground = level.getBlockState(below);
		if (!ground.isFaceSturdy(level, below, Direction.UP) || hurts(ground)) {
			return false;
		}
		return passable(level, feet) && passable(level, feet.above());
	}

	/** Answers whether a player passes through the block unhurt and breathes in it. */
	private static boolean passable(ServerLevel level, BlockPos pos) {
		BlockState state = level.getBlockState(pos);
		return state.getCollisionShape(level, pos).isEmpty() && state.getFluidState().isEmpty()
				&& !hurts(state);
	}

	/** Answers whether the block hurts a player who stands on or in it. */
	private static boolean hurts(BlockState state) {
		return state.is(BlockTags.FIRE) || state.is(BlockTags.CAMPFIRES)
				|| state.is(Blocks.CACTUS) || state.is(Blocks.MAGMA_BLOCK)
				|| state.is(Blocks.SWEET_BERRY_BUSH) || state.is(Blocks.WITHER_ROSE)
				|| state.is(Blocks.POWDER_SNOW) || state.is(Blocks.POINTED_DRIPSTONE)
				|| state.getFluidState().is(FluidTags.LAVA);
	}
}
