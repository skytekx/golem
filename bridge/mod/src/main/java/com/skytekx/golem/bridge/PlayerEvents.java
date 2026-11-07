// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The listeners of the bridge for the players: joins, leaves, chat, deaths and respawns.
 *
 * Chat and deaths are events that another mod may cancel, so the bridge listens on them last,
 * with the lowest priority, and Golem sees only what really happens. A plugin that cancels a
 * chat message does so after every other mod had its say. Forge 47 fires the chat event on a
 * worker thread of the server and not on the server thread, so the call into Golem does too.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Host;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.player.Player;
import net.minecraftforge.event.ServerChatEvent;
import net.minecraftforge.event.entity.living.LivingDeathEvent;
import net.minecraftforge.event.entity.player.PlayerEvent;
import net.minecraftforge.eventbus.api.EventPriority;
import net.minecraftforge.eventbus.api.SubscribeEvent;

/** Reports what the players do to the natives of Golem. */
public final class PlayerEvents {
	private final Guard guard;

	PlayerEvents(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Reports that a player joined.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void joined(PlayerEvent.PlayerLoggedInEvent event) {
		Player player = event.getEntity();
		guard.run("Host.playerJoined", () -> {
			Host.playerJoined(player.getStringUUID(), name(player));
		});
	}

	/**
	 * Reports that a player left.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void left(PlayerEvent.PlayerLoggedOutEvent event) {
		Player player = event.getEntity();
		guard.run("Host.playerLeft", () -> {
			Host.playerLeft(player.getStringUUID(), name(player));
		});
	}

	/**
	 * Reports a chat message with its raw text, and cancels it when a plugin asks to.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void chat(ServerChatEvent event) {
		ServerPlayer player = event.getPlayer();
		Boolean cancel = guard.ask("Host.chat", () -> {
			return Host.chat(player.getStringUUID(), name(player), event.getRawText());
		}, Boolean.FALSE);
		if (Boolean.TRUE.equals(cancel)) {
			event.setCanceled(true);
		}
	}

	/**
	 * Reports the death of a player with the id of the damage type and where they died, so a
	 * plugin can send them back there.
	 *
	 * @param event the event of Forge, for any living entity
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void death(LivingDeathEvent event) {
		if (!(event.getEntity() instanceof ServerPlayer player)) {
			return;
		}
		guard.run("Host.death", () -> {
			String cause = damageType(event.getSource());
			String dimension = player.level().dimension().location().toString();
			Host.death(player.getStringUUID(), name(player), cause, dimension, player.getX(),
					player.getY(), player.getZ(), player.getYRot(), player.getXRot());
		});
	}

	/**
	 * Reports the respawn of a player with the id of the dimension they respawn in. Forge fires
	 * the event after a death and when a player leaves the End through its exit portal, which
	 * the game handles as a respawn too, so the bridge says which of the two it is.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void respawn(PlayerEvent.PlayerRespawnEvent event) {
		Player player = event.getEntity();
		boolean endConquered = event.isEndConquered();
		guard.run("Host.respawn", () -> {
			String dimension = player.level().dimension().location().toString();
			Host.respawn(player.getStringUUID(), name(player), dimension, endConquered);
		});
	}

	private static String name(Player player) {
		return player.getGameProfile().getName();
	}

	/**
	 * Answers the registry id of the damage type, such as minecraft:fall.
	 *
	 * <p>A damage type that a mod builds outside the registry has no id, so its message id
	 * stands in for it.
	 */
	private static String damageType(DamageSource source) {
		return source.typeHolder().unwrapKey()
				.map(key -> key.location().toString())
				.orElse(source.getMsgId());
	}
}
