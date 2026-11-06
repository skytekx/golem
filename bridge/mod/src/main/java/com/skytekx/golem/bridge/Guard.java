// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The guard around every call of the bridge into the natives of Golem and into the game on
 * behalf of Golem.
 *
 * A fault of the bridge must never stop the server. The guard catches what a call throws, logs
 * it with the logger golem and answers a fallback, so the game goes on. A call that fails on
 * every tick would write twenty lines a second, so each call logs its first failure with the
 * stack trace and after that only its tenth, its hundredth and every further power of ten.
 *
 * Forge fires the chat event on a worker thread, so the guard is called from more than one
 * thread and keeps its counts in a concurrent map.
 */
package com.skytekx.golem.bridge;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Supplier;

/** Runs a call of the bridge and logs what it throws instead of passing it on. */
final class Guard {
	/** How often each call failed, by its name. */
	private final Map<String, Long> failures = new ConcurrentHashMap<>();

	/**
	 * Runs a call that answers nothing.
	 *
	 * @param name the name of the call in the log, such as Host.tick
	 * @param call the call
	 */
	void run(String name, Runnable call) {
		try {
			call.run();
		} catch (Exception | LinkageError fault) {
			failed(name, fault);
		}
	}

	/**
	 * Runs a call and answers its answer, or the fallback when it throws.
	 *
	 * @param name the name of the call in the log, such as Host.drain
	 * @param call the call
	 * @param fallback the answer when the call throws
	 * @return what the call answered, or the fallback
	 */
	<T> T ask(String name, Supplier<T> call, T fallback) {
		try {
			return call.get();
		} catch (Exception | LinkageError fault) {
			failed(name, fault);
			return fallback;
		}
	}

	private void failed(String name, Throwable fault) {
		long count = failures.merge(name, 1L, Long::sum);
		if (count == 1) {
			GolemBridge.LOG.error("{} failed, and the game goes on", name, fault);
		} else if (powerOfTen(count)) {
			GolemBridge.LOG.error("{} failed {} times, and the game goes on: {}",
					name, count, fault.toString());
		}
	}

	private static boolean powerOfTen(long count) {
		long rest = count;
		while (rest % 10 == 0) {
			rest /= 10;
		}
		return rest == 1;
	}
}
