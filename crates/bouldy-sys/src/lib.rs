//! Raw C ABI bridge definitions for Bouldy.
//!
//! This crate intentionally contains only minimal loader-facing ABI types. It
//! does not expose Unreal SDK bindings, generated game types, or asset tooling.

use std::ffi::{c_char, CString};
use std::ptr::NonNull;

/// Tick callback type registered by Rust with a loader shim.
pub type TickCallback = extern "C" fn(delta_seconds: f32);

/// Shutdown callback type registered by Rust with a loader shim.
pub type ShutdownCallback = extern "C" fn();

/// Required minimal Unreal/loader API passed to Rust mods.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealApi {
    /// Log a null-terminated UTF-8-ish message.
    pub log: extern "C" fn(*const c_char),
    /// Read the current frame delta in seconds from the loader/game.
    pub get_delta_seconds: extern "C" fn() -> f32,
}

/// Version 1 extension API for shim-owned lifecycle registration.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealApiV1 {
    /// Base API. This must remain the first field for layout compatibility.
    pub base: UnrealApi,
    /// Register the Rust tick callback with the host shim.
    pub register_tick: Option<extern "C" fn(TickCallback)>,
    /// Register the Rust shutdown callback with the host shim.
    pub register_shutdown: Option<extern "C" fn(ShutdownCallback)>,
}

/// Return the base API pointer embedded in a V1 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV1` for the duration
/// of the returned pointer's use.
pub unsafe fn base_from_v1(api: *mut UnrealApiV1) -> Option<NonNull<UnrealApi>> {
    let api = NonNull::new(api)?;
    // SAFETY: The caller guarantees that `api` points to a valid `UnrealApiV1`.
    let base = unsafe { &mut api.as_ptr().as_mut()?.base };
    Some(NonNull::from(base))
}

/// Log through the raw API pointer.
///
/// Interior NUL bytes are replaced with spaces before creating the C string.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealApi`. If non-null, its `log`
/// function pointer must be callable for the duration of this call.
pub unsafe fn log(api: *mut UnrealApi, msg: &str) {
    let Some(api) = NonNull::new(api) else {
        return;
    };

    let sanitized = msg.replace('\0', " ");
    let Ok(c_msg) = CString::new(sanitized) else {
        return;
    };

    // SAFETY: The caller guarantees the API pointer and function pointer are
    // valid. `c_msg` is NUL-terminated and lives for the duration of the call.
    unsafe {
        (api.as_ref().log)(c_msg.as_ptr());
    }
}

/// Read delta seconds through the raw API pointer.
///
/// Returns `0.0` for a null API pointer.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealApi`. If non-null, its
/// `get_delta_seconds` function pointer must be callable for this call.
pub unsafe fn get_delta_seconds(api: *mut UnrealApi) -> f32 {
    let Some(api) = NonNull::new(api) else {
        return 0.0;
    };

    // SAFETY: The caller guarantees the API pointer and function pointer are
    // valid for the duration of this call.
    unsafe { (api.as_ref().get_delta_seconds)() }
}

/// Register a tick callback through a V1 API if the host supplied one.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealApiV1`.
pub unsafe fn register_tick(api: *mut UnrealApiV1, callback: TickCallback) {
    let Some(api) = NonNull::new(api) else {
        return;
    };

    // SAFETY: The caller guarantees `api` points to a valid `UnrealApiV1`.
    if let Some(register) = unsafe { api.as_ref().register_tick } {
        register(callback);
    }
}

/// Register a shutdown callback through a V1 API if the host supplied one.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealApiV1`.
pub unsafe fn register_shutdown(api: *mut UnrealApiV1, callback: ShutdownCallback) {
    let Some(api) = NonNull::new(api) else {
        return;
    };

    // SAFETY: The caller guarantees `api` points to a valid `UnrealApiV1`.
    if let Some(register) = unsafe { api.as_ref().register_shutdown } {
        register(callback);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    use std::mem::{offset_of, size_of};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static LAST_LOG: Mutex<Option<String>> = Mutex::new(None);
    static TICK_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWN_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn test_log(msg: *const c_char) {
        // SAFETY: Test callers pass a valid NUL-terminated string from `log`.
        let msg = unsafe { CStr::from_ptr(msg) }
            .to_string_lossy()
            .into_owned();
        *LAST_LOG.lock().unwrap() = Some(msg);
    }

    extern "C" fn test_delta() -> f32 {
        1.0 / 60.0
    }

    extern "C" fn test_tick(_delta: f32) {}

    extern "C" fn test_shutdown() {}

    extern "C" fn register_tick_callback(callback: TickCallback) {
        TICK_REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
        callback(0.5);
    }

    extern "C" fn register_shutdown_callback(callback: ShutdownCallback) {
        SHUTDOWN_REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
        callback();
    }

    #[test]
    fn abi_layout_keeps_base_api_at_offset_zero() {
        assert_eq!(offset_of!(UnrealApiV1, base), 0);
        assert_eq!(
            size_of::<Option<extern "C" fn(TickCallback)>>(),
            size_of::<usize>()
        );
        assert!(size_of::<UnrealApi>() >= size_of::<usize>() * 2);
        assert!(size_of::<UnrealApiV1>() >= size_of::<UnrealApi>() + size_of::<usize>() * 2);
    }

    #[test]
    fn log_ignores_null_api() {
        // SAFETY: This test verifies that the wrapper handles a null API.
        unsafe { log(std::ptr::null_mut(), "ignored") };
    }

    #[test]
    fn log_sanitizes_interior_nuls() {
        let mut api = UnrealApi {
            log: test_log,
            get_delta_seconds: test_delta,
        };

        // SAFETY: `api` points to a valid test API table for this call.
        unsafe { log(&mut api, "hello\0world") };

        assert_eq!(LAST_LOG.lock().unwrap().as_deref(), Some("hello world"));
    }

    #[test]
    fn base_from_v1_handles_null_and_returns_base_pointer() {
        // SAFETY: This test verifies null handling.
        assert!(unsafe { base_from_v1(std::ptr::null_mut()) }.is_none());

        let mut api = UnrealApiV1 {
            base: UnrealApi {
                log: test_log,
                get_delta_seconds: test_delta,
            },
            register_tick: None,
            register_shutdown: None,
        };

        // SAFETY: `api` points to a valid V1 API table for this call.
        let base = unsafe { base_from_v1(&mut api) }.expect("base pointer");
        assert_eq!(base.as_ptr(), std::ptr::addr_of_mut!(api.base));
    }

    #[test]
    fn lifecycle_registration_handles_null_and_missing_callbacks() {
        TICK_REGISTRATIONS.store(0, Ordering::SeqCst);
        SHUTDOWN_REGISTRATIONS.store(0, Ordering::SeqCst);

        // SAFETY: These calls verify null handling.
        unsafe {
            register_tick(std::ptr::null_mut(), test_tick);
            register_shutdown(std::ptr::null_mut(), test_shutdown);
        }

        let mut api = UnrealApiV1 {
            base: UnrealApi {
                log: test_log,
                get_delta_seconds: test_delta,
            },
            register_tick: None,
            register_shutdown: None,
        };

        // SAFETY: `api` points to a valid V1 API table with absent callbacks.
        unsafe {
            register_tick(&mut api, test_tick);
            register_shutdown(&mut api, test_shutdown);
        }

        assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 0);
        assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn lifecycle_registration_calls_host_callbacks() {
        TICK_REGISTRATIONS.store(0, Ordering::SeqCst);
        SHUTDOWN_REGISTRATIONS.store(0, Ordering::SeqCst);

        let mut api = UnrealApiV1 {
            base: UnrealApi {
                log: test_log,
                get_delta_seconds: test_delta,
            },
            register_tick: Some(register_tick_callback),
            register_shutdown: Some(register_shutdown_callback),
        };

        // SAFETY: `api` points to a valid V1 API table with test callbacks.
        unsafe {
            register_tick(&mut api, test_tick);
            register_shutdown(&mut api, test_shutdown);
        }

        assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 1);
    }
}
