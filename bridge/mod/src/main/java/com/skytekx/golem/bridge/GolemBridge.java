// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The Golem bridge mod: the class that Forge constructs, which decides whether the bridge runs.
 *
 * The mod meets three kinds of server. Under Golem the host class is on the module path and its
 * natives answer, so the mod registers its listeners. On a stock Forge without golem-host.jar
 * the host class is missing, and on a stock Forge with the jar but without Golem the natives are
 * not registered. In both of those the mod logs one warning and registers nothing, so the server
 * runs as if the mod were not installed.
 */
package com.skytekx.golem.bridge;

import com.skytekx.golem.host.Host;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.javafmlmod.FMLJavaModLoadingContext;
import org.apache.logging.log4j.LogManager;
import org.apache.logging.log4j.Logger;

/** The mod golem, which Forge constructs once, on a thread of the mod loader. */
@Mod(GolemBridge.ID)
public final class GolemBridge {
	/** The id of the mod in its mods.toml. */
	public static final String ID = "golem";

	/** The logger of every line of the bridge. */
	static final Logger LOG = LogManager.getLogger("golem");

	private static final String OFF = "so the Golem bridge is off"
			+ " and the server runs as a stock Forge server";

	/**
	 * Registers the listeners of the bridge when the natives of Golem answer.
	 *
	 * @param context the context of the mod, which names its version
	 */
	public GolemBridge(FMLJavaModLoadingContext context) {
		if (!natives()) {
			return;
		}
		Guard guard = new Guard();
		MinecraftForge.EVENT_BUS.register(new ServerEvents(guard, new Actions(guard)));
		Object version = context.getContainer().getModInfo().getVersion();
		LOG.info("The Golem bridge {} is on", version);
	}

	/**
	 * Answers whether the natives of Golem answer, and logs the one warning when they do not.
	 *
	 * <p>The reference to the host class resolves on the first call, so a missing
	 * golem-host.jar shows as NoClassDefFoundError here and nowhere else.
	 */
	private static boolean natives() {
		try {
			if (Host.available()) {
				return true;
			}
			LOG.warn("The natives of Golem are not registered, {}", OFF);
		} catch (NoClassDefFoundError missing) {
			LOG.warn("The host class of Golem is missing, {}", OFF);
		} catch (RuntimeException | LinkageError fault) {
			LOG.error("The probe of the natives of Golem failed, {}", OFF, fault);
		}
		return false;
	}
}
