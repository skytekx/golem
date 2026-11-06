// SPDX-License-Identifier: AGPL-3.0-only
// SPDX-FileCopyrightText: 2026 Florian Kleber <opensource@skytekx.com>
/*
 * The invocation of the JVM: loads libjvm of a Java home, creates the JVM with JNI_CreateJavaVM
 * on a thread of its own, calls the main class as JavaMain in java.c of JDK 17 calls it, and
 * waits in DestroyJavaVM until the last thread of the server ends.
 *
 * The main class loads through sun.launcher.LauncherHelper, the class the java launcher uses,
 * so the class resolves through the system class loader with the checks and the messages of the
 * launcher, and the arguments become strings in the encoding of the platform as they do there.
 * After main the thread detaches before DestroyJavaVM, so the main thread ends as it does under
 * the launcher and an exception of main reaches the uncaught exception handler of the JVM.
 *
 * This is the only module of the crate with unsafe code, since every call of the invocation API
 * crosses into C. Nothing here unwinds into the JVM: the hooks catch every panic, and the
 * function that registers the native methods runs under catch_unwind.
 */

use std::{
    ffi::{
        CString,
        c_void, //
    },
    io::Write,
    panic::AssertUnwindSafe,
    path::{
        Path,
        PathBuf, //
    },
    ptr,
};

use jni::{
    JNIEnv,
    objects::{
        JClass,
        JObject,
        JString,
        JValue, //
    },
    sys,
};

use crate::{
    JavaHome,
    JvmError,
    LaunchSpec,
    Outcome, //
};

/// The stack of the thread golem-jvm, which runs main and with it the boot of Forge. The launcher
/// gives its main thread the default of the JVM, 1 MiB on Linux on `x86_64`.
const STACK: usize = 8 << 20;

/// The launch mode of a main class, `LM_CLASS` of `LauncherHelper`.
const LM_CLASS: sys::jint = 1;

/// The value the launcher gives the property sun.java.launcher.
const LAUNCHER: &str = "-Dsun.java.launcher=SUN_STANDARD";

/// `JNI_CreateJavaVM`, as jni.h declares it.
type CreateJavaVm = unsafe extern "system" fn(
    vm: *mut *mut sys::JavaVM,
    env: *mut *mut c_void,
    arguments: *mut c_void,
) -> sys::jint;

/// What the thread golem-jvm needs to run the launch.
struct Job {
    libjvm: PathBuf,
    options: Vec<CString>,
    main_class: String,
    arguments: Vec<String>,
}

/// Logs a line of the host on standard error, and never panics, since the hooks call it from
/// the JVM.
fn log(line: std::fmt::Arguments<'_>) {
    let _ = writeln!(std::io::stderr(), "[golem] {line}");
}

/// The exit hook of the JVM, which `HotSpot` calls with the status of `System.exit` or `Runtime.halt`
/// before it ends the process itself.
extern "system" fn exit_hook(status: sys::jint) {
    let _ = std::panic::catch_unwind(|| log(format_args!("the JVM exits with status {status}")));
}

/// The abort hook of the JVM, which `HotSpot` calls on a fatal error before it aborts the process.
extern "system" fn abort_hook() {
    let _ = std::panic::catch_unwind(|| log(format_args!("the JVM aborts on a fatal error")));
}

/// The size an -Xss value names, `parse_size` of java.c: digits and at most one unit.
fn parse_size(text: &str) -> Option<usize> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    let number: u64 = text[..digits].parse().ok()?;
    let unit: u64 = match &text[digits..] {
        "" => 1,
        "k" | "K" => 1 << 10,
        "m" | "M" => 1 << 20,
        "g" | "G" => 1 << 30,
        "t" | "T" => 1 << 40,
        _ => return None,
    };
    usize::try_from(number.checked_mul(unit)?).ok()
}

/// The stack of the thread golem-jvm: the larger of [`STACK`] and the last -Xss of the
/// options, which the launcher takes for the stack of its main thread. An -Xss of 0 asks for the
/// default of the JVM, which is smaller than [`STACK`].
fn thread_stack(options: &[String]) -> usize {
    options
        .iter()
        .filter_map(|option| option.strip_prefix("-Xss"))
        .filter_map(parse_size)
        .next_back()
        .unwrap_or(0)
        .max(STACK)
}

/// The options of `JNI_CreateJavaVM`: those of the launch, then sun.java.command and
/// sun.java.launcher as the launcher sets them, last, so they win over the same properties in
/// the launch as they do under the launcher.
fn vm_options(spec: &LaunchSpec) -> Result<Vec<CString>, JvmError> {
    let command = std::iter::once(spec.main_class.as_str())
        .chain(spec.arguments.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    spec.options
        .iter()
        .cloned()
        .chain([format!("-Dsun.java.command={command}"), LAUNCHER.to_owned()])
        .map(|option| {
            CString::new(option).map_err(|error| {
                JvmError::new(format!(
                    "the option {:?} holds a NUL byte",
                    String::from_utf8_lossy(&error.into_vec())
                ))
            })
        })
        .collect()
}

/// Loads libjvm and finds `JNI_CreateJavaVM` in it.
///
/// The library stays loaded until the process ends, since threads of the JVM may outlive
/// `DestroyJavaVM`. It loads as the launcher loads it, with every symbol bound at once and made
/// global, so a native library that a mod loads finds the JNI functions.
fn load(libjvm: &Path) -> Result<CreateJavaVm, JvmError> {
    use libloading::os::unix::{
        Library,
        RTLD_GLOBAL,
        RTLD_NOW, //
    };

    // SAFETY: Loading libjvm runs its initializers, which are written to run when the java
    // launcher loads the library the same way, with RTLD_NOW and RTLD_GLOBAL.
    let library = unsafe { Library::open(Some(libjvm), RTLD_NOW | RTLD_GLOBAL) }
        .map_err(|error| JvmError::new(format!("could not load {}: {error}", libjvm.display())))?;
    // SAFETY: Every libjvm exports JNI_CreateJavaVM with the type of jni.h, which CreateJavaVm
    // spells.
    let create =
        unsafe { library.get::<CreateJavaVm>(b"JNI_CreateJavaVM\0") }.map_err(|error| {
            JvmError::new(format!(
                "{} holds no JNI_CreateJavaVM: {error}",
                libjvm.display()
            ))
        })?;
    let create = *create;
    std::mem::forget(library);
    Ok(create)
}

/// The meaning of an error code of `JNI_CreateJavaVM`.
fn create_error(code: sys::jint) -> String {
    let meaning = match code {
        sys::JNI_ERR => "JNI_ERR, the JVM could not start, see its messages above",
        sys::JNI_EDETACHED => "JNI_EDETACHED, the thread is not attached to the JVM",
        sys::JNI_EVERSION => "JNI_EVERSION, the JVM does not support JNI 1.8",
        sys::JNI_ENOMEM => "JNI_ENOMEM, the JVM ran out of memory while it started",
        sys::JNI_EEXIST => "JNI_EEXIST, a JVM already runs in this process",
        sys::JNI_EINVAL => "JNI_EINVAL, the JVM refused an option, see its message above",
        _ => return format!("JNI_CreateJavaVM answered the unknown code {code}"),
    };
    format!("JNI_CreateJavaVM failed with {meaning}")
}

/// Creates the JVM on the current thread with `options` and the hooks of the host.
fn create_vm(
    create: CreateJavaVm,
    options: &[CString],
) -> Result<(*mut sys::JavaVM, *mut sys::JNIEnv), JvmError> {
    let mut raw: Vec<sys::JavaVMOption> = options
        .iter()
        .map(|option| sys::JavaVMOption {
            optionString: option.as_ptr().cast_mut(),
            extraInfo: ptr::null_mut(),
        })
        .collect();
    let exit: extern "system" fn(sys::jint) = exit_hook;
    let abort: extern "system" fn() = abort_hook;
    raw.push(sys::JavaVMOption {
        optionString: c"exit".as_ptr().cast_mut(),
        extraInfo: exit as *mut c_void,
    });
    raw.push(sys::JavaVMOption {
        optionString: c"abort".as_ptr().cast_mut(),
        extraInfo: abort as *mut c_void,
    });
    let count = sys::jint::try_from(raw.len())
        .map_err(|_| JvmError::new("the launch has more options than the JVM takes"))?;
    let mut arguments = sys::JavaVMInitArgs {
        version: sys::JNI_VERSION_1_8,
        nOptions: count,
        options: raw.as_mut_ptr(),
        ignoreUnrecognized: sys::JNI_FALSE,
    };
    let mut vm = ptr::null_mut();
    let mut env = ptr::null_mut();
    // SAFETY: The pointers lead to live locals, the options point into `options` and to string
    // literals, which outlive the call, and HotSpot only reads them. The hooks have the types
    // that HotSpot casts extraInfo to for the options exit and abort.
    let code = unsafe { create(&raw mut vm, &raw mut env, (&raw mut arguments).cast()) };
    if code != sys::JNI_OK {
        return Err(JvmError::new(create_error(code)));
    }
    Ok((vm, env.cast()))
}

/// A Java string of `text` in the encoding of the platform, as `NewPlatformString` of java.c
/// makes it through `LauncherHelper.makePlatformString`.
fn platform_string<'local>(
    env: &mut JNIEnv<'local>,
    helper: &JClass<'_>,
    text: &str,
) -> jni::errors::Result<JObject<'local>> {
    let bytes = env.byte_array_from_slice(text.as_bytes())?;
    let string = env
        .call_static_method(
            helper,
            "makePlatformString",
            "(Z[B)Ljava/lang/String;",
            &[JValue::Bool(sys::JNI_TRUE), JValue::Object(&bytes)],
        )?
        .l()?;
    env.delete_local_ref(bytes)?;
    Ok(string)
}

/// Loads the main class as the launcher does and calls its main method with `arguments`.
fn call_main(
    env: &mut JNIEnv<'_>,
    main_class: &str,
    arguments: &[String],
) -> jni::errors::Result<()> {
    let helper = env.find_class("sun/launcher/LauncherHelper")?;
    let name = platform_string(env, &helper, main_class)?;
    // A main class that does not load ends the JVM here through System.exit(1), after
    // LauncherHelper has printed the error of the launcher.
    let class = env
        .call_static_method(
            &helper,
            "checkAndLoadMain",
            "(ZILjava/lang/String;)Ljava/lang/Class;",
            &[
                JValue::Bool(sys::JNI_TRUE),
                JValue::Int(LM_CLASS),
                JValue::Object(&name),
            ],
        )?
        .l()?;
    let class = JClass::from(class);
    let count = sys::jsize::try_from(arguments.len())
        .map_err(|_| jni::errors::Error::JniCall(jni::errors::JniError::InvalidArguments))?;
    let array = env.new_object_array(count, "java/lang/String", JObject::null())?;
    for (index, argument) in (0..count).zip(arguments) {
        let string = platform_string(env, &helper, argument)?;
        env.set_object_array_element(&array, index, &string)?;
        env.delete_local_ref(string)?;
    }
    env.call_static_method(
        &class,
        "main",
        "([Ljava/lang/String;)V",
        &[JValue::Object(&array)],
    )?;
    Ok(())
}

/// The text of a throwable, from its toString.
fn describe(env: &mut JNIEnv<'_>, throwable: &JObject<'_>) -> Option<String> {
    let text = env
        .call_method(throwable, "toString", "()Ljava/lang/String;", &[])
        .and_then(jni::objects::JValueOwned::l);
    let Ok(text) = text else {
        let _ = env.exception_clear();
        return None;
    };
    let text = JString::from(text);
    env.get_string(&text).ok().map(Into::into)
}

/// The text of the pending exception, which stays pending, so that `DetachCurrentThread` hands
/// it to the uncaught exception handler as it does under the launcher.
fn pending_exception(env: &mut JNIEnv<'_>) -> Option<String> {
    if !env.exception_check().unwrap_or(false) {
        return None;
    }
    let throwable = env.exception_occurred().ok()?;
    let _ = env.exception_clear();
    let text = describe(env, &throwable)
        .unwrap_or_else(|| "an exception whose toString failed".to_owned());
    let _ = env.throw(throwable);
    Some(text)
}

/// Runs `register` and the main class on the thread that created the JVM.
fn enter<F>(
    raw: *mut sys::JNIEnv,
    main_class: &str,
    arguments: &[String],
    register: F,
) -> Result<Result<(), String>, JvmError>
where
    F: FnOnce(&mut JNIEnv<'_>) -> jni::errors::Result<()>,
{
    // SAFETY: JNI_CreateJavaVM answered `raw` for this thread, which stays attached as the
    // main thread of the JVM until leave detaches it, after every use of the wrapper.
    let mut env = unsafe { JNIEnv::from_raw(raw) }
        .map_err(|error| JvmError::new(format!("the JVM answered no JNIEnv: {error}")))?;
    let registered = std::panic::catch_unwind(AssertUnwindSafe(|| register(&mut env)));
    let failure = match registered {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("the function panicked".to_owned()),
    };
    if let Some(failure) = failure {
        let exception = pending_exception(&mut env)
            .map(|text| format!(", {text}"))
            .unwrap_or_default();
        return Err(JvmError::new(format!(
            "registering the native methods failed: {failure}{exception}"
        )));
    }
    Ok(call_main(&mut env, main_class, arguments)
        .map_err(|error| pending_exception(&mut env).unwrap_or_else(|| error.to_string())))
}

/// Detaches the current thread and destroys the JVM, which waits for every thread of the
/// server that is no daemon. Answers whether the thread detached and the code of `DestroyJavaVM`.
fn leave(vm: *mut sys::JavaVM) -> Result<(bool, sys::jint), JvmError> {
    // SAFETY: `vm` is the JVM that JNI_CreateJavaVM made, and its table of functions lives as
    // long as the process.
    let functions = unsafe { &**vm };
    let (Some(detach), Some(destroy)) = (functions.DetachCurrentThread, functions.DestroyJavaVM)
    else {
        return Err(JvmError::new(
            "the JVM has no DetachCurrentThread or DestroyJavaVM",
        ));
    };
    // SAFETY: The current thread created the JVM, holds no Java frames, and no wrapper of its
    // JNIEnv is used after this point.
    let detached = unsafe { detach(vm) } == sys::JNI_OK;
    // SAFETY: The JVM exists, and DestroyJavaVM may be called from a detached thread, which
    // it attaches itself.
    let code = unsafe { destroy(vm) };
    Ok((detached, code))
}

/// Creates the JVM of `job`, runs it and waits until it ends, on the current thread.
fn launch<F>(job: &Job, register: F) -> Result<Outcome, JvmError>
where
    F: FnOnce(&mut JNIEnv<'_>) -> jni::errors::Result<()>,
{
    let create = load(&job.libjvm)?;
    let (vm, env) = create_vm(create, &job.options)?;
    let main = enter(env, &job.main_class, &job.arguments, register);
    let (detached, destroy_code) = leave(vm)?;
    let main = main?;
    if !detached {
        return Err(JvmError::new(
            "the main thread could not detach from the JVM",
        ));
    }
    Ok(Outcome { main, destroy_code })
}

/// Runs `spec` in a new JVM of `java_home` on the thread golem-jvm and waits until it ends.
pub(crate) fn run<F>(
    java_home: &JavaHome,
    spec: &LaunchSpec,
    register: F,
) -> Result<Outcome, JvmError>
where
    F: FnOnce(&mut JNIEnv<'_>) -> jni::errors::Result<()> + Send + 'static,
{
    let job = Job {
        libjvm: java_home.libjvm(),
        options: vm_options(spec)?,
        main_class: spec.main_class.clone(),
        arguments: spec.arguments.clone(),
    };
    let thread = std::thread::Builder::new()
        .name("golem-jvm".to_owned())
        .stack_size(thread_stack(&spec.options))
        .spawn(move || launch(&job, register))
        .map_err(|error| JvmError::new(format!("could not start the thread golem-jvm: {error}")))?;
    thread
        .join()
        .map_err(|_| JvmError::new("the thread golem-jvm panicked"))?
}
