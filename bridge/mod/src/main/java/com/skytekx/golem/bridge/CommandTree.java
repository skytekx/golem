// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The tree of commands of Brigadier, as far as a plugin command that takes the name of a command
 * of the game or a mod needs it.
 *
 * Brigadier merges a literal that is registered under a name the root already holds into the node
 * of that name: the node keeps its requirement and its arguments, which win the parse, and takes
 * the command of the new literal. A plugin command named like a command of the game would run only
 * without arguments and only for whom the game allows. So the bridge moves the node of the game
 * aside to minecraft:<name>, as CraftBukkit keeps a command of the game that a plugin replaced,
 * and registers the plugin command on the free name. Brigadier 1.1.8 offers no way to remove a
 * child, so the bridge removes it from the two private maps of the root, children and literals,
 * whose names stay as they are on a server, since only Minecraft is remapped. An alias of the root
 * that redirects to the moved node, such as tell to msg, is rebuilt to redirect to its new place,
 * since the tree that a client receives keeps a redirect only to a node that the root still holds.
 */
package com.skytekx.golem.bridge;

import com.mojang.brigadier.tree.CommandNode;
import com.mojang.brigadier.tree.LiteralCommandNode;
import com.mojang.brigadier.tree.RootCommandNode;
import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import net.minecraft.commands.CommandSourceStack;

/** Moves a command of the game or a mod aside, so a plugin command can take its name. */
final class CommandTree {
	/** The prefix of the name under which a command that a plugin replaced stays. */
	static final String PREFIX = "minecraft:";

	/** The map of the children of a node by their names, or null when Brigadier hides it. */
	private static final Field CHILDREN = field("children");

	/** The map of the literal children of a node by their names, or null when hidden. */
	private static final Field LITERALS = field("literals");

	private CommandTree() {
	}

	private static Field field(String name) {
		try {
			Field field = CommandNode.class.getDeclaredField(name);
			field.setAccessible(true);
			return field;
		} catch (ReflectiveOperationException | RuntimeException hidden) {
			return null;
		}
	}

	/**
	 * Moves the command of the root with the name to minecraft:<name>, with its requirement, its
	 * command, its arguments and its redirect, and points the aliases of the root that redirect to
	 * it at the moved command.
	 *
	 * @param root the root of the dispatcher of the event
	 * @param name the name that a plugin command takes
	 * @return true when the name is free, false when it stays as it was, since the root holds
	 *         minecraft:<name> already or Brigadier hides its maps
	 */
	static boolean moveAside(RootCommandNode<CommandSourceStack> root, String name) {
		CommandNode<CommandSourceStack> taken = root.getChild(name);
		if (taken == null) {
			return true;
		}
		if (CHILDREN == null || LITERALS == null || root.getChild(PREFIX + name) != null) {
			return false;
		}
		LiteralCommandNode<CommandSourceStack> moved = copy(PREFIX + name, taken,
				taken.getRedirect());
		if (!remove(root, name)) {
			return false;
		}
		root.addChild(moved);
		List<CommandNode<CommandSourceStack>> aliases = new ArrayList<>();
		for (CommandNode<CommandSourceStack> child : root.getChildren()) {
			if (child.getRedirect() == taken && child.getChildren().isEmpty()) {
				aliases.add(child);
			}
		}
		for (CommandNode<CommandSourceStack> alias : aliases) {
			if (remove(root, alias.getName())) {
				root.addChild(copy(alias.getName(), alias, moved));
			}
		}
		return true;
	}

	/** Answers a literal with the name that does what the node does, redirected to redirect. */
	private static LiteralCommandNode<CommandSourceStack> copy(String name,
			CommandNode<CommandSourceStack> node, CommandNode<CommandSourceStack> redirect) {
		LiteralCommandNode<CommandSourceStack> copy = new LiteralCommandNode<>(name,
				node.getCommand(), node.getRequirement(), redirect, node.getRedirectModifier(),
				node.isFork());
		for (CommandNode<CommandSourceStack> child : node.getChildren()) {
			copy.addChild(child);
		}
		return copy;
	}

	/** Removes the child with the name from the root, and answers whether it could. */
	private static boolean remove(RootCommandNode<CommandSourceStack> root, String name) {
		try {
			((Map<?, ?>) CHILDREN.get(root)).remove(name);
			((Map<?, ?>) LITERALS.get(root)).remove(name);
			return true;
		} catch (ReflectiveOperationException | RuntimeException hidden) {
			return false;
		}
	}
}
