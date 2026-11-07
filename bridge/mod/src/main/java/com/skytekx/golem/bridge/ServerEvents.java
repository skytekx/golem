// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The listeners of the bridge for the life of the server and for its ticks.
 *
 * The end of every server tick is the one point where Golem acts on the game: the bridge reports
 * how long the tick took, drains the actions that Golem queued since the last tick and runs them
 * there, on the server thread, so Rust never touches the game from another thread.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Host;
import net.minecraft.server.MinecraftServer;
import net.minecraftforge.event.TickEvent;
import net.minecraftforge.event.server.ServerStartedEvent;
import net.minecraftforge.event.server.ServerStartingEvent;
import net.minecraftforge.event.server.ServerStoppedEvent;
import net.minecraftforge.event.server.ServerStoppingEvent;
import net.minecraftforge.eventbus.api.EventPriority;
import net.minecraftforge.eventbus.api.SubscribeEvent;
import net.minecraftforge.versions.forge.ForgeVersion;

/** Reports the life of the server and its ticks, and runs the actions after each tick. */
public final class ServerEvents {
	private static final String[] NONE = {};

	private final Guard guard;
	private final Actions actions;

	/** When the running tick started, in the nanoseconds of System.nanoTime. */
	private long tickStart;

	ServerEvents(Guard guard, Actions actions) {
		this.guard = guard;
		this.actions = actions;
	}

	/**
	 * Reports that the server is starting, with the versions it runs.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void starting(ServerStartingEvent event) {
		MinecraftServer server = event.getServer();
		GameQueries.stopped(false);
		guard.run("Host.serverStarting", () -> {
			Host.serverStarting(ForgeVersion.getVersion(), server.getServerVersion());
		});
	}

	/**
	 * Reports that the server has started.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void started(ServerStartedEvent event) {
		guard.run("Host.serverStarted", Host::serverStarted);
	}

	/**
	 * Reports that the server is stopping.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void stopping(ServerStoppingEvent event) {
		guard.run("Host.serverStopping", Host::serverStopping);
	}

	/**
	 * Reports that the server has stopped. Minecraft closed every level before this event, so the
	 * game answers no question of Golem from here on.
	 *
	 * @param event the event of Forge
	 */
	@SubscribeEvent
	public void stopped(ServerStoppedEvent event) {
		GameQueries.stopped(true);
		guard.run("Host.serverStopped", Host::serverStopped);
	}

	/**
	 * Notes the start of a tick, before every other listener, so the time covers theirs.
	 *
	 * @param event the event of Forge, of either phase
	 */
	@SubscribeEvent(priority = EventPriority.HIGHEST)
	public void tickStart(TickEvent.ServerTickEvent event) {
		if (event.phase == TickEvent.Phase.START) {
			tickStart = System.nanoTime();
		}
	}

	/**
	 * Reports the end of a tick, after every other listener, and runs the queued actions.
	 *
	 * @param event the event of Forge, of either phase
	 */
	@SubscribeEvent(priority = EventPriority.LOWEST)
	public void tickEnd(TickEvent.ServerTickEvent event) {
		if (event.phase != TickEvent.Phase.END) {
			return;
		}
		long nanos = System.nanoTime() - tickStart;
		guard.run("Host.tick", () -> Host.tick(nanos));
		String[] queued = guard.ask("Host.drain", Host::drain, NONE);
		actions.run(event.getServer(), queued);
	}
}
