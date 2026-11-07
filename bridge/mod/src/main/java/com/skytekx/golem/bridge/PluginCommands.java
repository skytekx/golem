// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The commands of the Golem plugins, registered with Brigadier.
 *
 * Every name that Golem answers becomes a literal that runs on its own and the same literal with
 * one greedy string argument, so the plugin parses its arguments itself. Brigadier asks for no
 * permission level: the plugin checks the sender through golem-permissions. The natives learn who
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
import com.skytekx.golem.host.Host;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraftforge.event.RegisterCommandsEvent;
import net.minecraftforge.eventbus.api.SubscribeEvent;

/** Registers the commands of the plugins and hands every run of one to the natives of Golem. */
public final class PluginCommands {
	private static final String[] NONE = {};

	/** The name of the one argument, which holds everything after the name and one space. */
	private static final String ARGUMENTS = "arguments";

	private final Guard guard;

	PluginCommands(Guard guard) {
		this.guard = guard;
	}

	/**
	 * Registers every command of the plugins, at the start and on every reload of the data.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void register(RegisterCommandsEvent event) {
		String[] names = guard.ask("Host.commandNames", Host::commandNames, NONE);
		if (names == null) {
			return;
		}
		for (String name : names) {
			if (parses(name)) {
				event.getDispatcher().register(literal(name));
			} else {
				GolemBridge.LOG.warn("Golem names the command '{}', which Brigadier"
						+ " cannot parse, so the bridge skips it", name);
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
