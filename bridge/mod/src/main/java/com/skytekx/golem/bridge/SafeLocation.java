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
 * The search reads the blocks through the level, which loads the chunks it looks at, as a
 * teleport there would. The radius is at most 8 blocks, so a search reads at most 2109 places.
 */
package com.skytekx.golem.bridge;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
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
	 * Answers the block of the feet of the nearest safe place near a target.
	 *
	 * @param level the level of the target
	 * @param x the x coordinate of the target
	 * @param y the y coordinate of the target
	 * @param z the z coordinate of the target
	 * @param radius how far to look, in blocks, held to 0 to {@link #MAX_RADIUS}
	 * @return the block of the feet, or null when no place within the radius is safe
	 */
	static BlockPos find(ServerLevel level, double x, double y, double z, int radius) {
		int limit = Math.max(0, Math.min(radius, MAX_RADIUS));
		BlockPos target = BlockPos.containing(x, y, z);
		BlockPos.MutableBlockPos feet = new BlockPos.MutableBlockPos();
		for (int[] offset : OFFSETS) {
			if (offset[3] > limit * limit) {
				break;
			}
			feet.set(target.getX() + offset[0], target.getY() + offset[1],
					target.getZ() + offset[2]);
			if (safe(level, feet)) {
				return feet.immutable();
			}
		}
		return null;
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

	private static boolean safe(ServerLevel level, BlockPos feet) {
		if (feet.getY() - 1 < level.getMinBuildHeight()
				|| feet.getY() + 1 >= level.getMaxBuildHeight()
				|| !level.getWorldBorder().isWithinBounds(feet)) {
			return false;
		}
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
