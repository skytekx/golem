// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The names that Golem gives players in the player list, such as a mark for a player who is away.
 *
 * Forge asks for the name of a player in the list through PlayerEvent.TabListNameFormat, and
 * ServerPlayer.refreshTabListName asks again and sends the answer to every client. The bridge
 * keeps the names that the action listName sets and answers the event with them, last of every
 * listener, so the name of Golem wins over the one of another mod. A name holds until Golem
 * clears it or the player leaves.
 */
package com.skytekx.golem.bridge;

import java.util.Map;
import java.util.UUID;
import java.util.concurrent.ConcurrentHashMap;
import net.minecraft.network.chat.Component;
import net.minecraftforge.event.entity.player.PlayerEvent;
import net.minecraftforge.eventbus.api.EventPriority;
import net.minecraftforge.eventbus.api.SubscribeEvent;

/** The names of the player list that Golem set, by the UUID of the player. */
public final class ListNames {
	private final Map<UUID, Component> names = new ConcurrentHashMap<>();

	/**
	 * Sets the name of a player in the list, or clears it.
	 *
	 * @param uuid the UUID of the player
	 * @param name the name, or the empty string to clear it
	 */
	void set(UUID uuid, String name) {
		if (name.isEmpty()) {
			names.remove(uuid);
		} else {
			names.put(uuid, Component.literal(name));
		}
	}

	/**
	 * Answers the name that Golem set for a player, after every other listener.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void format(PlayerEvent.TabListNameFormat event) {
		Component name = names.get(event.getEntity().getUUID());
		if (name != null) {
			event.setDisplayName(name);
		}
	}

	/**
	 * Forgets the name of a player who left.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void left(PlayerEvent.PlayerLoggedOutEvent event) {
		names.remove(event.getEntity().getUUID());
	}
}
