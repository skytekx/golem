// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The game through the JNI: the object of the interface Game that the mod connected, asked with
 * the JNIEnv of the native that runs, so every question is read on the thread of that native.
 *
 * Each question runs in a local frame of its own, so a native that asks many questions leaves
 * no local reference behind, and converts the answer into Rust before the frame ends. A Java
 * exception or error that a question throws is taken and cleared here, so it never reaches the
 * caller of the native, and the question answers none. A question that fails, or an answer that
 * does not read, is logged the first time, the tenth, the hundredth and every further power of
 * ten, as the guard of the mod logs, with the class and the message of what it threw, and its
 * stack trace is printed for its first failure alone, so a plugin that asks on every tick
 * cannot fill the log. The safe wrapper of jni 0.21 covers every call here, so this module holds
 * no unsafe code.
 */

use std::cell::RefCell;

use jni::{
    JNIEnv,
    errors::Result,
    objects::{
        JObject,
        JObjectArray,
        JString,
        JThrowable,
        JValue, //
    },
};

use crate::{
    BlockLocation,
    Game,
    Heights,
    Item,
    Location,
    OnlinePlayer,
    SafePlace,
    Spawn,
    Throttle,
    game::wire,
    log, //
};

/// The radius that the interface Game of the host jar takes at most, in blocks.
const MAX_RADIUS: u32 = 8;

/// How often each question failed, by the name of its method.
static FAILURES: Throttle = Throttle::new();

/// The game behind the object that the mod connected, asked with the `JNIEnv` of a native.
pub(crate) struct JniGame<'a, 'local> {
    env: RefCell<&'a mut JNIEnv<'local>>,
    game: &'a JObject<'static>,
}

impl<'a, 'local> JniGame<'a, 'local> {
    /// The game of the object `game`, asked through `env`, the `JNIEnv` of the running native.
    pub(crate) fn new(env: &'a mut JNIEnv<'local>, game: &'a JObject<'static>) -> Self {
        Self {
            env: RefCell::new(env),
            game,
        }
    }

    /// Asks the method `method`, which takes one string and answers one, with `value`, and
    /// answers its string, or `None` for null or a failure.
    fn string(&self, method: &'static str, value: &str) -> Option<String> {
        self.ask(method, |env, game| {
            let value = env.new_string(value)?;
            let answer = env
                .call_method(
                    game,
                    method,
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(&value)],
                )?
                .l()?;
            text(env, &JString::from(answer))
        })
        .flatten()
    }

    /// Runs one question in a local frame of its own, and answers `None` when it fails, after
    /// the exception it left is taken and cleared and the failure is logged.
    fn ask<T>(
        &self,
        method: &'static str,
        question: impl FnOnce(&mut JNIEnv<'_>, &JObject<'static>) -> Result<T>,
    ) -> Option<T> {
        let Ok(mut env) = self.env.try_borrow_mut() else {
            return None;
        };
        let game = self.game;
        match env.with_local_frame(8, |env| question(env, game)) {
            Ok(answer) => Some(answer),
            Err(error) => {
                fault(&mut env, &FAILURES, method, &error, |count, fault| {
                    log(format_args!(
                        "the question {method} of the game {fault}, so it answers none, {count} times so far"
                    ));
                });
                None
            }
        }
    }
}

/// Takes and clears the exception that a failed call of the JNI left pending, so it never
/// reaches the JVM, and counts the failure of `kind` in `throttle`.
///
/// When a line on the log is due, `report` gets the count and the fault: what the call threw,
/// as `toString` of the throwable writes its class and message, or the error of the JNI when
/// it threw nothing. The stack trace of the throwable is printed for the first failure of
/// `kind` alone.
pub(crate) fn fault(
    env: &mut JNIEnv<'_>,
    throttle: &Throttle,
    kind: &'static str,
    error: &jni::errors::Error,
    report: impl FnOnce(u64, std::fmt::Arguments<'_>),
) {
    let thrown = take_exception(env);
    if let Some(count) = throttle.count(kind) {
        match &thrown {
            Some(throwable) => {
                let text = describe(env, throwable);
                report(count, format_args!("threw {text}"));
                if count == 1 {
                    let _ = env.call_method(throwable, "printStackTrace", "()V", &[]);
                    let _ = env.exception_clear();
                }
            }
            None => report(count, format_args!("failed with \"{error}\"")),
        }
    }
    if let Some(throwable) = thrown {
        let _ = env.delete_local_ref(throwable);
    }
}

/// Takes the pending exception and clears it, or answers `None` when none is pending.
fn take_exception<'local>(env: &mut JNIEnv<'local>) -> Option<JThrowable<'local>> {
    if !env.exception_check().unwrap_or(false) {
        return None;
    }
    let throwable = env.exception_occurred().ok();
    let _ = env.exception_clear();
    throwable.filter(|throwable| !throwable.is_null())
}

/// Answers `toString` of a throwable, its class and its message, or its class alone when that
/// throws as well.
fn describe(env: &mut JNIEnv<'_>, throwable: &JThrowable<'_>) -> String {
    let text = env
        .call_method(throwable, "toString", "()Ljava/lang/String;", &[])
        .and_then(jni::objects::JValueGen::l)
        .and_then(|value| {
            let value = JString::from(value);
            let text = text(env, &value);
            let _ = env.delete_local_ref(value);
            text
        });
    if let Ok(Some(text)) = text {
        text
    } else {
        let _ = env.exception_clear();
        "a throwable whose toString fails".to_owned()
    }
}

/// Answers a Java string as a Rust string, or `None` for null.
fn text(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(env.get_string(value)?.into()))
}

/// Logs an answer of the question `method` that does not read, at powers of ten, and answers
/// `None`.
fn unread<T>(method: &'static str, answer: &str) -> Option<T> {
    if let Some(count) = FAILURES.count(method) {
        log(format_args!(
            "the question {method} of the game answered {answer:?}, which does not read, so it answers none, {count} times so far"
        ));
    }
    None
}

impl Game for JniGame<'_, '_> {
    fn players(&self) -> Option<Vec<OnlinePlayer>> {
        let lines = self.ask("players", |env, game| {
            let array = env
                .call_method(game, "players", "()[Ljava/lang/String;", &[])?
                .l()?;
            if array.is_null() {
                return Ok(None);
            }
            let array = JObjectArray::from(array);
            let length = env.get_array_length(&array)?;
            let mut lines = Vec::new();
            for index in 0..length {
                let element = JString::from(env.get_object_array_element(&array, index)?);
                lines.extend(text(env, &element)?);
                env.delete_local_ref(element)?;
            }
            Ok(Some(lines))
        })??;
        Some(
            lines
                .iter()
                .filter_map(|line| wire::player(line).or_else(|| unread("players", line)))
                .collect(),
        )
    }

    fn location(&self, uuid: &str) -> Option<Location> {
        let answer = self.string("location", uuid)?;
        wire::location(&answer).or_else(|| unread("location", &answer))
    }

    fn respawn(&self, uuid: &str) -> Option<Option<BlockLocation>> {
        let answer = self.string("respawn", uuid)?;
        wire::respawn(&answer).or_else(|| unread("respawn", &answer))
    }

    fn spawn(&self, dimension: &str) -> Option<Spawn> {
        let answer = self.string("spawn", dimension)?;
        wire::spawn(dimension, &answer).or_else(|| unread("spawn", &answer))
    }

    fn main_hand(&self, uuid: &str) -> Option<Option<Item>> {
        let answer = self.string("mainHand", uuid)?;
        wire::main_hand(&answer).or_else(|| unread("mainHand", &answer))
    }

    fn heights(&self, dimension: &str) -> Option<Heights> {
        let answer = self.string("heights", dimension)?;
        wire::heights(&answer).or_else(|| unread("heights", &answer))
    }

    fn safe_location(&self, target: &Location, radius: u32) -> Option<SafePlace> {
        if !target.is_finite() {
            return None;
        }
        let radius = i32::try_from(radius.min(MAX_RADIUS)).unwrap_or(0);
        let answer = self
            .ask("safeLocation", |env, game| {
                let dimension = env.new_string(&target.dimension)?;
                let answer = env
                    .call_method(
                        game,
                        "safeLocation",
                        "(Ljava/lang/String;DDDI)Ljava/lang/String;",
                        &[
                            JValue::Object(&dimension),
                            JValue::Double(target.x),
                            JValue::Double(target.y),
                            JValue::Double(target.z),
                            JValue::Int(radius),
                        ],
                    )?
                    .l()?;
                text(env, &JString::from(answer))
            })
            .flatten()?;
        wire::safe_location(target, &answer).or_else(|| unread("safeLocation", &answer))
    }
}
