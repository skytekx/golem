// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The commands of the Golem plugins, registered with Brigadier.
 *
 * Every name that Golem answers becomes a literal that runs on its own and the same literal with
 * one greedy string argument, so the plugin parses its arguments itself. Brigadier asks for no
 * permission level: the plugin checks the sender through golem-permissions. A command block, a
 * function and the console all run as the console, with no player, as okf/bridge.md specifies.
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
	 * message, which the game rule sendCommandFeedback and a silent source do not suppress.
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

	/** Asks the plugin, with the player who runs the command or with no one for the console. */
	private static String[] ask(CommandSourceStack source, String name, String arguments) {
		String uuid = null;
		String player = null;
		if (source.getEntity() instanceof ServerPlayer sender) {
			uuid = sender.getStringUUID();
			player = sender.getGameProfile().getName();
		}
		String[] lines = Host.command(name, uuid, player, arguments);
		return lines == null ? NONE : lines;
	}
}
