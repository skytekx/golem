// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The text that Golem sends to players, with its links made clickable.
 *
 * The client of Minecraft 1.20.1 turns no address in a system message into a link and copies no
 * text of the chat, so a player would have to type the link to the source that /golem source
 * offers by hand. Forge makes the links of a chat message with ForgeHooks.newChatWithLinks, and
 * the bridge does the same for every line that Golem sends: an address with its scheme, such as
 * https://, opens on a click and shows underlined in blue, as a link in the chat of Forge does.
 * An address without a scheme stays plain text, so a word such as a file name never turns into a
 * link that a player did not mean.
 */
package com.skytekx.golem.bridge;

import net.minecraft.network.chat.Component;
import net.minecraftforge.common.ForgeHooks;

/** Makes the lines of Golem into components of the chat whose links a player clicks. */
final class Links {
	private Links() {
	}

	/**
	 * Answers the line as a component of the chat whose addresses with a scheme open on a click.
	 *
	 * @param line a line of Golem
	 * @return the component, which reads as the line
	 */
	static Component text(String line) {
		return ForgeHooks.newChatWithLinks(line, false);
	}
}
