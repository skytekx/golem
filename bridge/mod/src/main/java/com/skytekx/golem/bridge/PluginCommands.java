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
 * made a command run and at which permission level: the player who typed it or clicked the sign
 * that runs it, whatever entity it runs as, the console, or another source. So a command block
 * never passes for the console, a function passes for it only at the level 4, where the game gives
 * it no entity and the rights of the console, and a source that is no player and runs a command as
 * a player through execute as never passes for that player, as okf/bridge.md specifies.
 */
package com.skytekx.golem.bridge;

import com.mojang.brigadier.Command;
import com.mojang.brigadier.ParseResults;
import com.mojang.brigadier.arguments.StringArgumentType;
import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import com.mojang.brigadier.builder.RequiredArgumentBuilder;
import com.mojang.brigadier.context.CommandContext;
import com.mojang.brigadier.tree.RootCommandNode;
import com.skytekx.golem.host.Host;
import java.util.Set;
import net.minecraft.commands.CommandSource;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.core.BlockPos;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.block.entity.SignBlockEntity;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.phys.Vec3;
import net.minecraftforge.event.CommandEvent;
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

	/** Reads the classes on the stack of the thread, for {@link #startedBySign()}. */
	private static final StackWalker STACK =
			StackWalker.getInstance(StackWalker.Option.RETAIN_CLASS_REFERENCE);

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

	/**
	 * Keeps who clicked a sign with the command that the click runs, before the game runs it.
	 *
	 * <p>The game gives the command of a sign a stack whose source takes no line and which runs
	 * as the player who clicked, and execute as changes that entity, so nothing in the stack
	 * says who clicked from then on. The bridge therefore puts a source of its own in the place of
	 * the one of the game, which takes no line either and holds that player, and execute as keeps
	 * it as it keeps every source. Only the stack that a command starts with is marked, and only
	 * when it has this shape and belongs to a sign: the block entity of a sign hands the command
	 * to the game, or the stack stands at the exact middle of a sign, as the sign of a mod does
	 * that runs its command from a class of its own. A command that the bridge takes for the
	 * click of a sign is judged as that player: by their nodes, and where no rule names a node
	 * by the lower of their operator level and the level of the stack. So it holds no more than
	 * that player holds, also when the bridge took it for a click in error. The listener runs
	 * behind the guard of the mod, so a fault in it is logged and the command starts as the game
	 * built it.
	 *
	 * @param event the event of Forge, which holds the stack that the command starts with
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void started(CommandEvent event) {
		guard.run("The start of a command", () -> mark(event));
	}

	/** Puts the source of the bridge into the stack of the event when a sign starts the command. */
	private static void mark(CommandEvent event) {
		ParseResults<CommandSourceStack> parse = event.getParseResults();
		CommandSourceStack source = parse.getContext().getSource();
		if (source.source == CommandSource.NULL && source.getEntity() instanceof ServerPlayer player
				&& (atSign(source) || startedBySign())) {
			Clicked clicked = new Clicked(player);
			event.setParseResults(Commands.mapSource(parse, stack -> stack.withSource(clicked)));
		}
	}

	/**
	 * Answers whether the block entity of a sign hands the command in progress to the game: the
	 * first class below the frames of Commands on the stack of the thread is that of a sign, as
	 * when a click runs the commands of its lines. A command that another command starts while
	 * the click runs has that other command below the frames of Commands, and no sign.
	 */
	private static boolean startedBySign() {
		return STACK.walk(frames -> frames.map(StackWalker.StackFrame::getDeclaringClass)
				.dropWhile(type -> type != Commands.class)
				.dropWhile(type -> type == Commands.class)
				.findFirst()
				.filter(SignBlockEntity.class::isAssignableFrom)
				.isPresent());
	}

	/**
	 * Answers whether the stack stands at a sign: at the exact middle of a block that is one by
	 * the tag of the game for signs, which the signs of mods join. The game gives the command of
	 * a sign the middle of the block of the sign as its place, and so does the sign of a mod, so
	 * a stack at any other place of the block is none of a sign. The answer reads a loaded chunk
	 * alone, and a stack that a mod built without a level stands at no sign.
	 */
	private static boolean atSign(CommandSourceStack source) {
		ServerLevel level = source.getLevel();
		if (level == null) {
			return false;
		}
		Vec3 where = source.getPosition();
		BlockPos place = BlockPos.containing(where);
		if (!where.equals(Vec3.atCenterOf(place))) {
			return false;
		}
		ChunkPos column = new ChunkPos(place);
		LevelChunk chunk = level.getChunkSource().getChunkNow(column.x, column.z);
		return chunk != null && chunk.getBlockState(place).is(BlockTags.ALL_SIGNS);
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
	 * Runs the command in the plugin and sends every line of its answer to what made the command
	 * run, as the game sends the feedback of its own commands with sendSuccess: to the player who
	 * typed it or clicked the sign that runs it, the console, the client of RCON or the output of
	 * a command block, whatever entity the command runs as. The lines go through a copy of the
	 * stack whose source accepts every line, so the game rule sendCommandFeedback does not
	 * suppress them. A silent source, such as a function, still sees no line, since the game
	 * drops the feedback of such a stack. A link in a line, such as the one to the source that
	 * /golem source offers, opens on a click. The line of a call that failed stays with the
	 * rule of the game, which shows no failure of the command of a sign. Only a player who is
	 * online is the sender of a command, so the lines of a command that any other player typed
	 * or started with a click go to no one.
	 */
	private int run(CommandContext<CommandSourceStack> context, String name, String rest) {
		CommandSourceStack source = context.getSource();
		ServerPlayer made = made(source);
		boolean online = made == null || online(made);
		String[] lines = guard.ask("Host.command", () -> ask(source, made, online, name, rest),
				null);
		if (!online) {
			return lines == null ? 0 : Command.SINGLE_SUCCESS;
		}
		if (lines == null) {
			source.sendFailure(Component.literal("The command " + name
					+ " failed, and the log of the server says why"));
			return 0;
		}
		CommandSourceStack answer =
				source.withSource(new Answer(made == null ? source.source : made));
		for (String line : lines) {
			if (line != null) {
				answer.sendSuccess(() -> Links.text(line), false);
			}
		}
		return Command.SINGLE_SUCCESS;
	}

	/**
	 * Takes the lines of an answer in place of what made the command run and hands each on to
	 * it. It accepts every line, where a player and a command block accept feedback only under
	 * the game rule sendCommandFeedback, and it tells no operator.
	 */
	private static final class Answer implements CommandSource {
		private final CommandSource made;

		Answer(CommandSource made) {
			this.made = made;
		}

		@Override
		public void sendSystemMessage(Component line) {
			made.sendSystemMessage(line);
		}

		@Override
		public boolean acceptsSuccess() {
			return true;
		}

		@Override
		public boolean acceptsFailure() {
			return true;
		}

		@Override
		public boolean shouldInformAdmins() {
			return false;
		}
	}

	/**
	 * Stands in the place of the source of a command that the click of a sign runs. It takes no
	 * line and no feedback, as the source that the game gives such a command, and holds the
	 * player who clicked.
	 */
	private static final class Clicked implements CommandSource {
		private final ServerPlayer player;

		Clicked(ServerPlayer player) {
			this.player = player;
		}

		@Override
		public void sendSystemMessage(Component line) {
		}

		@Override
		public boolean acceptsSuccess() {
			return false;
		}

		@Override
		public boolean acceptsFailure() {
			return false;
		}

		@Override
		public boolean shouldInformAdmins() {
			return false;
		}
	}

	/**
	 * Answers the player who made the command run, by typing it or by the click of a sign, or
	 * null for a command that no player made run.
	 */
	private static ServerPlayer made(CommandSourceStack source) {
		if (source.source instanceof Clicked clicked) {
			return clicked.player;
		}
		return source.source instanceof ServerPlayer typed ? typed : null;
	}

	/**
	 * Answers whether a player is online: the very player that the list of the server holds
	 * under their UUID, and no other player that carries the same profile.
	 */
	private static boolean online(ServerPlayer player) {
		return player.server.getPlayerList().getPlayer(player.getUUID()) == player;
	}

	/**
	 * Asks the plugin with the source of the command and its permission level: the player who
	 * made the command run with their UUID and name, the console and RCON with neither, and every
	 * other source with its name alone. The game keeps what made a command run in the field
	 * source of the stack, which Forge makes public, and execute as changes the entity of the
	 * stack and keeps that field. So a command that a player typed reaches the plugin as that
	 * player, whatever entity it runs as, and a command that the console or a command block runs
	 * as a player reaches it as that source, named after the player, and never borrows the rights
	 * of the player. The command of a sign reaches it as the player who clicked, whom
	 * {@link #started(CommandEvent)} keeps in that field. Only a player who is online is the
	 * sender of a command: any other player who made a command run reaches the plugin as another
	 * source, under the name of the stack and at the level 0, whatever the level of the stack.
	 */
	private static String[] ask(CommandSourceStack source, ServerPlayer made, boolean online,
			String name, String arguments) {
		int level = level(source);
		String uuid = null;
		String sender = null;
		if (made != null && online) {
			uuid = made.getStringUUID();
			sender = made.getGameProfile().getName();
		} else if (made != null) {
			sender = source.getTextName();
			level = Commands.LEVEL_ALL;
		} else if (source.getEntity() != null || level < Commands.LEVEL_OWNERS) {
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
