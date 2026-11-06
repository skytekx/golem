// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The natives of golem-bridge: the functions behind the native methods of the host class, which
 * the JVM calls on the server thread, and their registration with RegisterNatives.
 *
 * Each native converts what the JVM hands it, calls its function of host.rs and converts the
 * answer back, so the logic lives in plain Rust that the notebooks reach without a JVM. The
 * safe wrapper of jni 0.21 covers every call here, so this module holds no unsafe code.
 */

use std::{
    ffi::c_void,
    panic::{
        self,
        AssertUnwindSafe, //
    },
    ptr, //
};

use jni::{
    JNIEnv, NativeMethod,
    errors::Result,
    objects::{
        JClass,
        JObject,
        JString, //
    },
    sys::{
        JNI_FALSE,
        JNI_TRUE,
        jboolean,
        jdouble,
        jfloat,
        jint,
        jlong,
        jobjectArray,
        jsize,
        jstring, //
    },
};

use crate::{
    HOST_CLASS,
    Location,
    host,
    log, //
};

/// The class of the elements of every array the natives answer.
const STRING_CLASS: &str = "java/lang/String";

/// Every native method of the host class: its name, its descriptor and its function.
///
/// The names and the descriptors are those of the host class in okf/bridge.md.
fn table() -> [(&'static str, &'static str, *mut c_void); 14] {
    [
        ("version", "()Ljava/lang/String;", version as *mut c_void),
        (
            "serverStarting",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            server_starting as *mut c_void,
        ),
        ("serverStarted", "()V", server_started as *mut c_void),
        ("serverStopping", "()V", server_stopping as *mut c_void),
        ("serverStopped", "()V", server_stopped as *mut c_void),
        ("tick", "(J)V", tick as *mut c_void),
        (
            "playerJoined",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            player_joined as *mut c_void,
        ),
        (
            "playerLeft",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            player_left as *mut c_void,
        ),
        (
            "chat",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z",
            chat as *mut c_void,
        ),
        (
            "death",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;DDDFF)V",
            death as *mut c_void,
        ),
        (
            "respawn",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
            respawn as *mut c_void,
        ),
        (
            "commandNames",
            "()[Ljava/lang/String;",
            command_names as *mut c_void,
        ),
        (
            "command",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;ILjava/lang/String;)[Ljava/lang/String;",
            command as *mut c_void,
        ),
        ("drain", "()[Ljava/lang/String;", drain as *mut c_void),
    ]
}

/// Answers the name and the descriptor of every native method that [`register`] registers.
pub fn signatures() -> Vec<(&'static str, &'static str)> {
    table()
        .iter()
        .map(|&(name, signature, _)| (name, signature))
        .collect()
}

/// Registers every native method on the host class, which `FindClass` looks up.
///
/// On an error the pending exception of the JVM is printed and cleared, so the caller may go
/// on with the JVM.
pub fn register(env: &mut JNIEnv<'_>) -> Result<()> {
    let methods: Vec<NativeMethod> = table()
        .into_iter()
        .map(|(name, signature, function)| NativeMethod {
            name: name.into(),
            sig: signature.into(),
            fn_ptr: function,
        })
        .collect();
    let registered = env.register_native_methods(HOST_CLASS, &methods);
    if registered.is_err() {
        clear_exception(env);
    }
    registered
}

/// Runs the body of a native and answers its value, or the pass value on a fault.
///
/// A panic and an error of the JNI are logged with the name of the native, the exception they
/// may have left pending is cleared, and `pass` makes the answer the contract names for a pass.
fn answer<'local, T>(
    env: &mut JNIEnv<'local>,
    native: &str,
    pass: fn(&mut JNIEnv<'local>) -> T,
    body: impl FnOnce(&mut JNIEnv<'local>) -> Result<T>,
) -> T {
    match panic::catch_unwind(AssertUnwindSafe(|| body(env))) {
        Ok(Ok(value)) => return value,
        Ok(Err(error)) => log(format_args!(
            "the native {native} failed in the JNI with \"{error}\", so it answers the pass value"
        )),
        Err(payload) => log(format_args!(
            "the native {native} panicked with \"{}\", so it answers the pass value",
            host::panic_message(payload.as_ref())
        )),
    }
    clear_exception(env);
    pass(env)
}

/// Prints and clears the exception that a failed call of the JNI left pending, if any.
fn clear_exception(env: &mut JNIEnv<'_>) {
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
}

/// Answers a Java string as a Rust string, or `None` for null.
///
/// A string that is no valid modified UTF-8, such as one with a lone surrogate, is decoded
/// with the replacement character, so the conversion never fails on its text.
fn string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(env.get_string(value)?.into()))
}

/// Answers `lines` as a new `String[]`.
fn string_array(env: &mut JNIEnv<'_>, lines: &[String]) -> Result<jobjectArray> {
    let length = jsize::try_from(lines.len()).unwrap_or(jsize::MAX);
    let array = env.new_object_array(length, STRING_CLASS, JObject::null())?;
    for (index, line) in (0..length).zip(lines) {
        let element = env.new_string(line)?;
        env.set_object_array_element(&array, index, &element)?;
        // A long answer would otherwise fill the table of local references of the call.
        env.delete_local_ref(element)?;
    }
    Ok(array.into_raw())
}

/// The pass value of a native that answers a `String[]`, an empty array, or null when even
/// that cannot be made.
fn empty_array(env: &mut JNIEnv<'_>) -> jobjectArray {
    if let Ok(array) = env.new_object_array(0, STRING_CLASS, JObject::null()) {
        array.into_raw()
    } else {
        clear_exception(env);
        ptr::null_mut()
    }
}

/// The pass value of a native that answers nothing.
fn nothing(_: &mut JNIEnv<'_>) {}

extern "system" fn version<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>) -> jstring {
    answer(
        &mut env,
        "version",
        |_| ptr::null_mut(),
        |env| Ok(env.new_string(host::version())?.into_raw()),
    )
}

extern "system" fn server_starting<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    forge_version: JString<'local>,
    minecraft_version: JString<'local>,
) {
    answer(&mut env, "serverStarting", nothing, |env| {
        let forge_version = string(env, &forge_version)?;
        let minecraft_version = string(env, &minecraft_version)?;
        host::server_starting(forge_version.as_deref(), minecraft_version.as_deref());
        Ok(())
    });
}

extern "system" fn server_started<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>) {
    answer(&mut env, "serverStarted", nothing, |_| {
        host::server_started();
        Ok(())
    });
}

extern "system" fn server_stopping<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>) {
    answer(&mut env, "serverStopping", nothing, |_| {
        host::server_stopping();
        Ok(())
    });
}

extern "system" fn server_stopped<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>) {
    answer(&mut env, "serverStopped", nothing, |_| {
        host::server_stopped();
        Ok(())
    });
}

extern "system" fn tick<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>, nanos: jlong) {
    answer(&mut env, "tick", nothing, |_| {
        host::tick(nanos);
        Ok(())
    });
}

extern "system" fn player_joined<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    uuid: JString<'local>,
    name: JString<'local>,
) {
    answer(&mut env, "playerJoined", nothing, |env| {
        let uuid = string(env, &uuid)?;
        let name = string(env, &name)?;
        host::player_joined(uuid.as_deref(), name.as_deref());
        Ok(())
    });
}

extern "system" fn player_left<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    uuid: JString<'local>,
    name: JString<'local>,
) {
    answer(&mut env, "playerLeft", nothing, |env| {
        let uuid = string(env, &uuid)?;
        let name = string(env, &name)?;
        host::player_left(uuid.as_deref(), name.as_deref());
        Ok(())
    });
}

extern "system" fn chat<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    uuid: JString<'local>,
    name: JString<'local>,
    message: JString<'local>,
) -> jboolean {
    answer(
        &mut env,
        "chat",
        |_| JNI_FALSE,
        |env| {
            let uuid = string(env, &uuid)?;
            let name = string(env, &name)?;
            let message = string(env, &message)?;
            let cancel = host::chat(uuid.as_deref(), name.as_deref(), message.as_deref());
            Ok(if cancel { JNI_TRUE } else { JNI_FALSE })
        },
    )
}

// The parameters are those of the native method, which the JNI hands over one by one.
#[allow(clippy::too_many_arguments)]
extern "system" fn death<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    uuid: JString<'local>,
    name: JString<'local>,
    cause: JString<'local>,
    dimension: JString<'local>,
    x: jdouble,
    y: jdouble,
    z: jdouble,
    yaw: jfloat,
    pitch: jfloat,
) {
    answer(&mut env, "death", nothing, |env| {
        let uuid = string(env, &uuid)?;
        let name = string(env, &name)?;
        let cause = string(env, &cause)?;
        let location = string(env, &dimension)?.map(|dimension| Location {
            dimension,
            x,
            y,
            z,
            yaw,
            pitch,
        });
        host::death(uuid.as_deref(), name.as_deref(), cause.as_deref(), location);
        Ok(())
    });
}

extern "system" fn respawn<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    uuid: JString<'local>,
    name: JString<'local>,
    dimension: JString<'local>,
) {
    answer(&mut env, "respawn", nothing, |env| {
        let uuid = string(env, &uuid)?;
        let name = string(env, &name)?;
        let dimension = string(env, &dimension)?;
        host::respawn(uuid.as_deref(), name.as_deref(), dimension.as_deref());
        Ok(())
    });
}

extern "system" fn command_names<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jobjectArray {
    answer(&mut env, "commandNames", empty_array, |env| {
        string_array(env, &host::command_names())
    })
}

extern "system" fn command<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    name: JString<'local>,
    sender_uuid: JString<'local>,
    sender_name: JString<'local>,
    level: jint,
    arguments: JString<'local>,
) -> jobjectArray {
    answer(&mut env, "command", empty_array, |env| {
        let name = string(env, &name)?;
        let sender_uuid = string(env, &sender_uuid)?;
        let sender_name = string(env, &sender_name)?;
        let arguments = string(env, &arguments)?;
        let lines = host::command(
            name.as_deref(),
            sender_uuid.as_deref(),
            sender_name.as_deref(),
            level,
            arguments.as_deref(),
        );
        string_array(env, &lines)
    })
}

extern "system" fn drain<'local>(mut env: JNIEnv<'local>, _class: JClass<'local>) -> jobjectArray {
    answer(&mut env, "drain", empty_array, |env| {
        let actions = host::drain();
        string_array(env, &actions).inspect_err(|_| host::requeue(&actions))
    })
}
