//! Loader lifecycle ABI and helpers.

use std::ffi::{c_char, CString};
use std::ptr::NonNull;

use crate::{UnrealDiscoveryApiV1, UnrealDiscoveryApiV2};

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

/// Version 2 Bouldy API: lifecycle V1 plus a separate discovery interface.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealApiV2 {
    /// Lifecycle/logging API. This must remain first for layout compatibility.
    pub lifecycle: UnrealApiV1,
    /// Optional game-agnostic discovery API.
    pub discovery: UnrealDiscoveryApiV1,
}

/// Version 3 Bouldy API: lifecycle V1 plus indexed discovery.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealApiV3 {
    /// Lifecycle/logging API. This must remain first for layout compatibility.
    pub lifecycle: UnrealApiV1,
    /// Optional indexed discovery API.
    pub discovery: UnrealDiscoveryApiV2,
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

/// Return the V1 lifecycle pointer embedded in a V2 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV2` for the duration
/// of the returned pointer's use.
pub unsafe fn lifecycle_from_v2(api: *mut UnrealApiV2) -> Option<NonNull<UnrealApiV1>> {
    let api = NonNull::new(api)?;
    // SAFETY: The caller guarantees that `api` points to a valid `UnrealApiV2`.
    let lifecycle = unsafe { &mut api.as_ptr().as_mut()?.lifecycle };
    Some(NonNull::from(lifecycle))
}

/// Return the base API pointer embedded in a V2 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV2` for the duration
/// of the returned pointer's use.
pub unsafe fn base_from_v2(api: *mut UnrealApiV2) -> Option<NonNull<UnrealApi>> {
    // SAFETY: This function has the same safety contract as `lifecycle_from_v2`.
    let lifecycle = unsafe { lifecycle_from_v2(api) }?;
    // SAFETY: `lifecycle` points into the valid V2 table.
    let base = unsafe { &mut lifecycle.as_ptr().as_mut()?.base };
    Some(NonNull::from(base))
}

/// Return the discovery API pointer embedded in a V2 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV2` for the duration
/// of the returned pointer's use.
pub unsafe fn discovery_from_v2(api: *mut UnrealApiV2) -> Option<NonNull<UnrealDiscoveryApiV1>> {
    let api = NonNull::new(api)?;
    // SAFETY: The caller guarantees that `api` points to a valid `UnrealApiV2`.
    let discovery = unsafe { &mut api.as_ptr().as_mut()?.discovery };
    Some(NonNull::from(discovery))
}

/// Return the V1 lifecycle pointer embedded in a V3 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV3` for the duration
/// of the returned pointer's use.
pub unsafe fn lifecycle_from_v3(api: *mut UnrealApiV3) -> Option<NonNull<UnrealApiV1>> {
    let api = NonNull::new(api)?;
    // SAFETY: The caller guarantees that `api` points to a valid `UnrealApiV3`.
    let lifecycle = unsafe { &mut api.as_ptr().as_mut()?.lifecycle };
    Some(NonNull::from(lifecycle))
}

/// Return the base API pointer embedded in a V3 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV3` for the duration
/// of the returned pointer's use.
pub unsafe fn base_from_v3(api: *mut UnrealApiV3) -> Option<NonNull<UnrealApi>> {
    // SAFETY: This function has the same safety contract as `lifecycle_from_v3`.
    let lifecycle = unsafe { lifecycle_from_v3(api) }?;
    // SAFETY: `lifecycle` points into the valid V3 table.
    let base = unsafe { &mut lifecycle.as_ptr().as_mut()?.base };
    Some(NonNull::from(base))
}

/// Return the discovery API pointer embedded in a V3 pointer.
///
/// # Safety
///
/// `api` must either be null or point to a valid `UnrealApiV3` for the duration
/// of the returned pointer's use.
pub unsafe fn discovery_from_v3(api: *mut UnrealApiV3) -> Option<NonNull<UnrealDiscoveryApiV2>> {
    let api = NonNull::new(api)?;
    // SAFETY: The caller guarantees that `api` points to a valid `UnrealApiV3`.
    let discovery = unsafe { &mut api.as_ptr().as_mut()?.discovery };
    Some(NonNull::from(discovery))
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
