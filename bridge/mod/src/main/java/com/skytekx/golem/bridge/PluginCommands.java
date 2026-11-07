// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The commands of the Golem plugins, registered with Brigadier.
 *
 * Every name that Golem answers becomes a literal that runs on its own and the same literal with
 * one greedy string argument, so the plugin parses its arguments itself. Brigadier asks for no
 * permission level: the plugin checks the sender through golem-permissions. A name that the game
 * or a mod holds already goes to the plugin, and CommandTree moves the command it replaces to
 * minecraft:<name>, but for the commands of the game that Spigot keeps. The natives learn who
 * runs a command and at which permission level, so a command block or a function never passes for
 * the console, and a source that runs a command as a player through execute as never passes for
 * that player, as okf/bridge.md specifies.
 */
package com.skytekx.golem.bridge;

import com.mojang.brigadier.Command;
import com.mojang.brigadier.arguments.StringArgumentType;
import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import com.mojang.brigadier.builder.RequiredArgumentBuilder;
import com.mojang.brigadier.context.CommandContext;
import com.mojang.brigadier.tree.RootCommandNode;
import com.skytekx.golem.host.Host;
import java.util.Set;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraftforge.event.RegisterCommandsEvent;
import net.minecraftforge.eventbus.api.EventPriority;
import net.minecraftforge.eventbus.api.SubscribeEvent;

/** Registers the commands of the plugins and hands every run of one to the natives of Golem. */
public final class PluginCommands {
	private static final String[] NONE = {};

	/** The name of the one argument, which holds everything after the name and one space. */
	private static final String ARGUMENTS = "arguments";

	/**
	 * The commands of the game that keep their names when a plugin names one of them, the list
	 * commands.replace-commands of spigot.yml as the live server holds it, the default of Spigot.
	 * Minecraft 1.20.1 has no testforblock, so that name is free for a plugin.
	 */
	private static final Set<String> KEPT = Set.of("setblock", "summon", "testforblock",
			"tellraw");

	private final Guard guard;

	PluginCommands(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Registers every command of the plugins, at the start and on every reload of the data.
	 *
	 * <p>The bridge listens last, after the mods registered their commands, so the rule below and
	 * not the order of the listeners decides a name that the game or a mod holds too. A plugin
	 * command takes the plain name and the command it replaces stays as minecraft:<name>, as
	 * CraftBukkit orders them, but for the names of {@link #KEPT}, which keep the command of the
	 * game.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void register(RegisterCommandsEvent event) {
		String[] names = guard.ask("Host.commandNames", Host::commandNames, NONE);
		if (names == null) {
			return;
		}
		RootCommandNode<CommandSourceStack> root = event.getDispatcher().getRoot();
		for (String name : names) {
			if (!parses(name)) {
				GolemBridge.LOG.warn("Golem names the command '{}', which Brigadier"
						+ " cannot parse, so the bridge skips it", name);
			} else if (root.getChild(name) == null) {
				event.getDispatcher().register(literal(name));
			} else if (KEPT.contains(name)) {
				GolemBridge.LOG.warn("Golem names the command '{}', which the game keeps, as"
						+ " replace-commands of Spigot keeps it, so the bridge skips it", name);
			} else if (CommandTree.moveAside(root, name)) {
				event.getDispatcher().register(literal(name));
				GolemBridge.LOG.info("The command '{}' of Golem takes the name of a command of"
						+ " the game or a mod, which stays as '{}{}'", name, CommandTree.PREFIX,
						name);
			} else {
				GolemBridge.LOG.warn("Golem names the command '{}', which the game or a mod"
						+ " holds and the bridge cannot move to '{}{}', so the bridge skips it",
						name, CommandTree.PREFIX, name);
			}
		}
	}

	/** Answers whether a player can type the name as the first word of a command. */
	private static boolean parses(String name) {
		if (name == null || name.isEmpty()) {
			return false;
		}
		return name.chars().noneMatch(Character::isWhitespace);
	}

	private LiteralArgumentBuilder<CommandSourceStack> literal(String name) {
		RequiredArgumentBuilder<CommandSourceStack, String> rest =
				Commands.argument(ARGUMENTS, StringArgumentType.greedyString());
		rest.executes(context -> {
			return run(context, name, StringArgumentType.getString(context, ARGUMENTS));
		});
		LiteralArgumentBuilder<CommandSourceStack> alone = Commands.literal(name);
		alone.executes(context -> run(context, name, ""));
		return alone.then(rest);
	}

	/**
	 * Runs the command in the plugin and sends every line of its answer to the sender, as a system
	 * message, which the game rule sendCommandFeedback does not suppress. A silent source, such
	 * as a function, still sees no line, since the game drops every message to it.
	 */
	private int run(CommandContext<CommandSourceStack> context, String name, String rest) {
		CommandSourceStack source = context.getSource();
		String[] lines = guard.ask("Host.command", () -> ask(source, name, rest), null);
		if (lines == null) {
			source.sendFailure(Component.literal("The command " + name
					+ " failed, and the log of the server says why"));
			return 0;
		}
		for (String line : lines) {
			if (line != null) {
				source.sendSystemMessage(Component.literal(line));
			}
		}
		return Command.SINGLE_SUCCESS;
	}

	/**
	 * Asks the plugin with the source of the command and its permission level: a player who runs
	 * it themself with their UUID and name, the console and RCON with neither, and every other
	 * source with its name alone. A command that another source runs as a player through execute
	 * as keeps that source in the field source of the stack, which Forge makes public, so it
	 * reaches the plugin as that source, named after the player, and never borrows the rights
	 * of the player.
	 */
	private static String[] ask(CommandSourceStack source, String name, String arguments) {
		int level = level(source);
		Entity entity = source.getEntity();
		String uuid = null;
		String sender = null;
		if (entity instanceof ServerPlayer player && source.source == player) {
			uuid = player.getStringUUID();
			sender = player.getGameProfile().getName();
		} else if (entity != null || level < Commands.LEVEL_OWNERS) {
			sender = source.getTextName();
		}
		String[] lines = Host.command(name, uuid, sender, level, arguments);
		return lines == null ? NONE : lines;
	}

	/** Answers the permission level of the source, which Minecraft keeps private, from 0 to 4. */
	private static int level(CommandSourceStack source) {
		int level = Commands.LEVEL_OWNERS;
		while (level > Commands.LEVEL_ALL && !source.hasPermission(level)) {
			level--;
		}
		return level;
	}
}
