// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The permission handler golem:permissions for the PermissionAPI of Forge, so that a mod that
 * asks Forge whether a player holds a node gets the answer of the plugins of Golem.
 *
 * Forge collects the handlers on PermissionGatherEvent.Handler while the server starts and uses
 * the one that permissionHandler of forge-server.toml names. A node of the type Boolean asks the
 * native permission, whose answer true or false decides. Every other answer, and every node of
 * another type, leaves the node to its default resolver, which is what the default handler of
 * Forge answers, so a plugin decides only what it knows.
 *
 * Mods ask Forge on any thread, so the native may be called beside the server thread.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Host;
import java.util.Collection;
import java.util.Collections;
import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.level.ServerPlayer;
import net.minecraftforge.eventbus.api.SubscribeEvent;
import net.minecraftforge.server.permission.events.PermissionGatherEvent;
import net.minecraftforge.server.permission.handler.IPermissionHandler;
import net.minecraftforge.server.permission.nodes.PermissionDynamicContext;
import net.minecraftforge.server.permission.nodes.PermissionNode;
import net.minecraftforge.server.permission.nodes.PermissionTypes;

/** Offers Forge the permission handler of Golem. */
public final class Permissions {
	/**
	 * The id that permissionHandler of forge-server.toml names to choose the handler. Forge 47
	 * deprecates the constructor of two strings, and the factory never fails for this id.
	 */
	static final ResourceLocation ID = ResourceLocation.tryBuild(GolemBridge.ID, "permissions");

	private final Guard guard;

	Permissions(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Offers the handler, which Forge creates when the configuration names it.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void handlers(PermissionGatherEvent.Handler event) {
		event.addPermissionHandler(ID, nodes -> new Handler(guard, nodes));
	}

	/** The handler: the plugins first, then the default resolver of the node. */
	private static final class Handler implements IPermissionHandler {
		private final Guard guard;
		private final Set<PermissionNode<?>> nodes;

		Handler(Guard guard, Collection<PermissionNode<?>> nodes) {
			this.guard = guard;
			this.nodes = Collections.unmodifiableSet(new HashSet<>(nodes));
		}

		@Override
		public ResourceLocation getIdentifier() {
			return ID;
		}

		@Override
		public Set<PermissionNode<?>> getRegisteredNodes() {
			return nodes;
		}

		@Override
		public <T> T getPermission(ServerPlayer player, PermissionNode<T> node,
				PermissionDynamicContext<?>... context) {
			T answer = ask(player.getUUID(), node);
			if (answer != null) {
				return answer;
			}
			return node.getDefaultResolver().resolve(player, player.getUUID(), context);
		}

		@Override
		public <T> T getOfflinePermission(UUID player, PermissionNode<T> node,
				PermissionDynamicContext<?>... context) {
			T answer = ask(player, node);
			if (answer != null) {
				return answer;
			}
			return node.getDefaultResolver().resolve(null, player, context);
		}

		/** Answers what the plugins decide for a node of the type Boolean, or null. */
		private <T> T ask(UUID player, PermissionNode<T> node) {
			if (!PermissionTypes.BOOLEAN.equals(node.getType())) {
				return null;
			}
			int answer = guard.ask("Host.permission", () -> {
				return Host.permission(player.toString(), node.getNodeName());
			}, Host.PERMISSION_NONE);
			Class<T> type = node.getType().typeToken();
			if (answer == Host.PERMISSION_TRUE) {
				return type.cast(Boolean.TRUE);
			}
			if (answer == Host.PERMISSION_FALSE) {
				return type.cast(Boolean.FALSE);
			}
			return null;
		}
	}
}
