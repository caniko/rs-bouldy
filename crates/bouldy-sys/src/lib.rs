//! Raw C ABI bridge definitions for Bouldy.
//!
//! This crate intentionally contains only minimal loader-facing ABI types. It
//! does not expose Unreal SDK bindings, generated game types, or asset tooling.

use std::ffi::{c_char, c_void, CString};
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

/// Discovery candidate kind for generic Unreal object reconnaissance.
///
/// These values intentionally model broad reflected concepts rather than
/// game-specific classes or symbols.
pub type DiscoveryKind = u32;

/// Candidate kind is unknown or supplied by a game-specific backend.
pub const DISCOVERY_KIND_UNKNOWN: DiscoveryKind = 0;
/// Candidate is a UObject instance or asset-like object.
pub const DISCOVERY_KIND_OBJECT: DiscoveryKind = 1 << 0;
/// Candidate is a UClass or reflected class.
pub const DISCOVERY_KIND_CLASS: DiscoveryKind = 1 << 1;
/// Candidate is a reflected function.
pub const DISCOVERY_KIND_FUNCTION: DiscoveryKind = 1 << 2;
/// Candidate is a reflected property or field.
pub const DISCOVERY_KIND_PROPERTY: DiscoveryKind = 1 << 3;
/// Candidate may be any known discovery kind.
pub const DISCOVERY_KIND_ANY: DiscoveryKind = DISCOVERY_KIND_OBJECT
    | DISCOVERY_KIND_CLASS
    | DISCOVERY_KIND_FUNCTION
    | DISCOVERY_KIND_PROPERTY;

/// Filter passed from Rust to the host discovery backend.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiscoveryFilter {
    /// Null-terminated UTF-8-ish search terms.
    pub terms: *const *const c_char,
    /// Number of pointers in `terms`.
    pub term_count: usize,
    /// Bitmask of `DISCOVERY_KIND_*` values. Zero means backend default.
    pub kind_mask: DiscoveryKind,
    /// Maximum number of candidates to visit. Zero means backend default.
    pub max_results: usize,
}

/// One discovery result supplied by the host backend.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiscoveryCandidate {
    /// One `DISCOVERY_KIND_*` value.
    pub kind: DiscoveryKind,
    /// Short display name, if known.
    pub name: *const c_char,
    /// Full object path, asset path, or qualified symbol path, if known.
    pub path: *const c_char,
    /// Owning class/package/module, if known.
    pub owner: *const c_char,
    /// Backend-specific flags. Rust mods must not assume semantics in v1.
    pub flags: u64,
}

/// Version 2 discovery result with stable schema and object indices.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiscoveryCandidateV2 {
    /// One `DISCOVERY_KIND_*` value.
    pub kind: DiscoveryKind,
    /// Candidate schema version. Current value is `2`.
    pub schema_version: u32,
    /// Short display name, if known.
    pub name: *const c_char,
    /// Full object path, asset path, or qualified symbol path, if known.
    pub path: *const c_char,
    /// Owning class/package/module, if known.
    pub owner: *const c_char,
    /// Backend-specific flags. Rust mods must not assume semantics in v1.
    pub flags: u64,
    /// Unreal object array chunk index, or `-1` when unknown.
    pub chunk_index: i32,
    /// Unreal object index within the chunk/global object array, or `-1` when unknown.
    pub object_index: i32,
}

/// Visitor called once per discovery candidate.
///
/// Return `true` to continue scanning, or `false` to stop early.
pub type DiscoveryVisitor =
    extern "C" fn(candidate: *const DiscoveryCandidate, user_data: *mut c_void) -> bool;

/// Visitor called once per V2 discovery candidate.
///
/// Return `true` to continue scanning, or `false` to stop early.
pub type DiscoveryVisitorV2 =
    extern "C" fn(candidate: *const DiscoveryCandidateV2, user_data: *mut c_void) -> bool;

/// Scan function supplied by the host discovery backend.
pub type DiscoveryScanFn = extern "C" fn(
    filter: *const DiscoveryFilter,
    visitor: DiscoveryVisitor,
    user_data: *mut c_void,
) -> usize;

/// Version 2 scan function supplied by the host discovery backend.
pub type DiscoveryScanV2Fn = extern "C" fn(
    filter: *const DiscoveryFilter,
    visitor: DiscoveryVisitorV2,
    user_data: *mut c_void,
) -> usize;

/// Structured export function supplied by the host discovery backend.
pub type DiscoveryExportFn = extern "C" fn(channel: *const c_char, payload: *const c_char);

/// Version 1 game-agnostic discovery API.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealDiscoveryApiV1 {
    /// Scan reflected or enumerated Unreal symbols using a generic filter.
    pub scan: Option<DiscoveryScanFn>,
    /// Export a structured record to the host backend.
    pub export_record: Option<DiscoveryExportFn>,
}

/// Version 2 game-agnostic discovery API with indexed candidates.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnrealDiscoveryApiV2 {
    /// Scan reflected or enumerated Unreal symbols using a generic filter.
    pub scan: Option<DiscoveryScanV2Fn>,
    /// Export a structured record to the host backend.
    pub export_record: Option<DiscoveryExportFn>,
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

/// Scan discovery candidates through the host backend.
///
/// Returns the number of candidates reported by the host, or zero if discovery
/// is unavailable.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealDiscoveryApiV1`. `filter`,
/// `visitor`, and `user_data` must obey the host scan function's contract.
pub unsafe fn scan_discovery(
    api: *mut UnrealDiscoveryApiV1,
    filter: *const DiscoveryFilter,
    visitor: DiscoveryVisitor,
    user_data: *mut c_void,
) -> usize {
    let Some(api) = NonNull::new(api) else {
        return 0;
    };

    // SAFETY: The caller guarantees `api` points to a valid discovery table.
    let Some(scan) = (unsafe { api.as_ref().scan }) else {
        return 0;
    };

    scan(filter, visitor, user_data)
}

/// Scan V2 discovery candidates through the host backend.
///
/// Returns the number of candidates reported by the host, or zero if discovery
/// is unavailable.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealDiscoveryApiV2`. `filter`,
/// `visitor`, and `user_data` must obey the host scan function's contract.
pub unsafe fn scan_discovery_v2(
    api: *mut UnrealDiscoveryApiV2,
    filter: *const DiscoveryFilter,
    visitor: DiscoveryVisitorV2,
    user_data: *mut c_void,
) -> usize {
    let Some(api) = NonNull::new(api) else {
        return 0;
    };

    // SAFETY: The caller guarantees `api` points to a valid discovery table.
    let Some(scan) = (unsafe { api.as_ref().scan }) else {
        return 0;
    };

    scan(filter, visitor, user_data)
}

/// Export a structured discovery record through the host backend.
///
/// Interior NUL bytes are replaced with spaces before creating C strings.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealDiscoveryApiV1`.
pub unsafe fn export_discovery_record(
    api: *mut UnrealDiscoveryApiV1,
    channel: &str,
    payload: &str,
) {
    let Some(api) = NonNull::new(api) else {
        return;
    };

    // SAFETY: The caller guarantees `api` points to a valid discovery table.
    let Some(export_record) = (unsafe { api.as_ref().export_record }) else {
        return;
    };

    let Ok(channel) = CString::new(channel.replace('\0', " ")) else {
        return;
    };
    let Ok(payload) = CString::new(payload.replace('\0', " ")) else {
        return;
    };

    export_record(channel.as_ptr(), payload.as_ptr());
}

/// Export a structured discovery record through the V2 host backend.
///
/// Interior NUL bytes are replaced with spaces before creating C strings.
///
/// # Safety
///
/// `api` must be null or point to a valid `UnrealDiscoveryApiV2`.
pub unsafe fn export_discovery_record_v2(
    api: *mut UnrealDiscoveryApiV2,
    channel: &str,
    payload: &str,
) {
    let Some(api) = NonNull::new(api) else {
        return;
    };

    // SAFETY: The caller guarantees `api` points to a valid discovery table.
    let Some(export_record) = (unsafe { api.as_ref().export_record }) else {
        return;
    };

    let Ok(channel) = CString::new(channel.replace('\0', " ")) else {
        return;
    };
    let Ok(payload) = CString::new(payload.replace('\0', " ")) else {
        return;
    };

    export_record(channel.as_ptr(), payload.as_ptr());
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
    static DISCOVERY_VISITS: AtomicUsize = AtomicUsize::new(0);

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

    extern "C" fn scan_callback(
        filter: *const DiscoveryFilter,
        visitor: DiscoveryVisitor,
        user_data: *mut c_void,
    ) -> usize {
        assert!(!filter.is_null());
        let name = CString::new("PlayerParryWindow").unwrap();
        let path = CString::new("/Script/SB.PlayerParryWindow").unwrap();
        let owner = CString::new("SBCombatComponent").unwrap();
        let candidate = DiscoveryCandidate {
            kind: DISCOVERY_KIND_FUNCTION,
            name: name.as_ptr(),
            path: path.as_ptr(),
            owner: owner.as_ptr(),
            flags: 7,
        };
        if visitor(&candidate, user_data) {
            DISCOVERY_VISITS.fetch_add(1, Ordering::SeqCst);
        }
        1
    }

    extern "C" fn scan_v2_callback(
        filter: *const DiscoveryFilter,
        visitor: DiscoveryVisitorV2,
        user_data: *mut c_void,
    ) -> usize {
        assert!(!filter.is_null());
        let name = CString::new("EnemyDodgePunishWindow").unwrap();
        let path = CString::new("/Script/SB.EnemyDodgePunishWindow").unwrap();
        let owner = CString::new("SBEnemyCombatComponent").unwrap();
        let candidate = DiscoveryCandidateV2 {
            kind: DISCOVERY_KIND_PROPERTY,
            schema_version: 2,
            name: name.as_ptr(),
            path: path.as_ptr(),
            owner: owner.as_ptr(),
            flags: 9,
            chunk_index: 3,
            object_index: 144,
        };
        if visitor(&candidate, user_data) {
            DISCOVERY_VISITS.fetch_add(1, Ordering::SeqCst);
        }
        1
    }

    extern "C" fn export_callback(channel: *const c_char, payload: *const c_char) {
        // SAFETY: Test callers pass valid null-terminated strings.
        let channel = unsafe { CStr::from_ptr(channel) }.to_string_lossy();
        // SAFETY: Test callers pass valid null-terminated strings.
        let payload = unsafe { CStr::from_ptr(payload) }.to_string_lossy();
        assert_eq!(channel, "combat");
        assert_eq!(payload, "hello world");
    }

    #[test]
    fn abi_layout_keeps_base_api_at_offset_zero() {
        assert_eq!(offset_of!(UnrealApiV1, base), 0);
        assert_eq!(offset_of!(UnrealApiV2, lifecycle), 0);
        assert_eq!(offset_of!(UnrealApiV3, lifecycle), 0);
        assert_eq!(
            size_of::<Option<extern "C" fn(TickCallback)>>(),
            size_of::<usize>()
        );
        assert_eq!(size_of::<Option<DiscoveryScanFn>>(), size_of::<usize>());
        assert_eq!(size_of::<Option<DiscoveryScanV2Fn>>(), size_of::<usize>());
        assert!(size_of::<UnrealApi>() >= size_of::<usize>() * 2);
        assert!(size_of::<UnrealApiV1>() >= size_of::<UnrealApi>() + size_of::<usize>() * 2);
        assert!(size_of::<UnrealApiV2>() >= size_of::<UnrealApiV1>());
        assert!(size_of::<UnrealApiV3>() >= size_of::<UnrealApiV1>());
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
    fn v2_pointer_helpers_handle_null_and_return_embedded_pointers() {
        // SAFETY: This test verifies null handling.
        assert!(unsafe { base_from_v2(std::ptr::null_mut()) }.is_none());
        // SAFETY: This test verifies null handling.
        assert!(unsafe { lifecycle_from_v2(std::ptr::null_mut()) }.is_none());
        // SAFETY: This test verifies null handling.
        assert!(unsafe { discovery_from_v2(std::ptr::null_mut()) }.is_none());

        let mut api = UnrealApiV2 {
            lifecycle: UnrealApiV1 {
                base: UnrealApi {
                    log: test_log,
                    get_delta_seconds: test_delta,
                },
                register_tick: None,
                register_shutdown: None,
            },
            discovery: UnrealDiscoveryApiV1 {
                scan: None,
                export_record: None,
            },
        };

        // SAFETY: `api` points to a valid V2 API table.
        let lifecycle = unsafe { lifecycle_from_v2(&mut api) }.unwrap();
        // SAFETY: `api` points to a valid V2 API table.
        let base = unsafe { base_from_v2(&mut api) }.unwrap();
        // SAFETY: `api` points to a valid V2 API table.
        let discovery = unsafe { discovery_from_v2(&mut api) }.unwrap();
        assert_eq!(lifecycle.as_ptr(), std::ptr::addr_of_mut!(api.lifecycle));
        assert_eq!(base.as_ptr(), std::ptr::addr_of_mut!(api.lifecycle.base));
        assert_eq!(discovery.as_ptr(), std::ptr::addr_of_mut!(api.discovery));
    }

    #[test]
    fn v3_pointer_helpers_handle_null_and_return_embedded_pointers() {
        // SAFETY: This test verifies null handling.
        assert!(unsafe { base_from_v3(std::ptr::null_mut()) }.is_none());
        // SAFETY: This test verifies null handling.
        assert!(unsafe { lifecycle_from_v3(std::ptr::null_mut()) }.is_none());
        // SAFETY: This test verifies null handling.
        assert!(unsafe { discovery_from_v3(std::ptr::null_mut()) }.is_none());

        let mut api = UnrealApiV3 {
            lifecycle: UnrealApiV1 {
                base: UnrealApi {
                    log: test_log,
                    get_delta_seconds: test_delta,
                },
                register_tick: None,
                register_shutdown: None,
            },
            discovery: UnrealDiscoveryApiV2 {
                scan: None,
                export_record: None,
            },
        };

        // SAFETY: `api` points to a valid V3 API table.
        let lifecycle = unsafe { lifecycle_from_v3(&mut api) }.unwrap();
        // SAFETY: `api` points to a valid V3 API table.
        let base = unsafe { base_from_v3(&mut api) }.unwrap();
        // SAFETY: `api` points to a valid V3 API table.
        let discovery = unsafe { discovery_from_v3(&mut api) }.unwrap();
        assert_eq!(lifecycle.as_ptr(), std::ptr::addr_of_mut!(api.lifecycle));
        assert_eq!(base.as_ptr(), std::ptr::addr_of_mut!(api.lifecycle.base));
        assert_eq!(discovery.as_ptr(), std::ptr::addr_of_mut!(api.discovery));
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

    #[test]
    fn discovery_scan_handles_null_missing_and_callback() {
        DISCOVERY_VISITS.store(0, Ordering::SeqCst);
        let filter = DiscoveryFilter {
            terms: std::ptr::null(),
            term_count: 0,
            kind_mask: DISCOVERY_KIND_ANY,
            max_results: 10,
        };

        extern "C" fn visitor(
            _candidate: *const DiscoveryCandidate,
            _user_data: *mut c_void,
        ) -> bool {
            true
        }

        // SAFETY: This test verifies null handling.
        let null_scan =
            unsafe { scan_discovery(std::ptr::null_mut(), &filter, visitor, std::ptr::null_mut()) };
        assert_eq!(null_scan, 0);

        let mut missing = UnrealDiscoveryApiV1 {
            scan: None,
            export_record: None,
        };

        // SAFETY: `missing` points to a valid table with absent callbacks.
        let missing_scan =
            unsafe { scan_discovery(&mut missing, &filter, visitor, std::ptr::null_mut()) };
        assert_eq!(missing_scan, 0);

        let mut api = UnrealDiscoveryApiV1 {
            scan: Some(scan_callback),
            export_record: None,
        };

        // SAFETY: `api` points to a valid table with test callbacks.
        let api_scan = unsafe { scan_discovery(&mut api, &filter, visitor, std::ptr::null_mut()) };
        assert_eq!(api_scan, 1);
        assert_eq!(DISCOVERY_VISITS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn discovery_v2_scan_handles_null_missing_and_callback() {
        DISCOVERY_VISITS.store(0, Ordering::SeqCst);
        let filter = DiscoveryFilter {
            terms: std::ptr::null(),
            term_count: 0,
            kind_mask: DISCOVERY_KIND_ANY,
            max_results: 10,
        };

        extern "C" fn visitor(
            candidate: *const DiscoveryCandidateV2,
            _user_data: *mut c_void,
        ) -> bool {
            assert!(!candidate.is_null());
            // SAFETY: Test callback passes a valid candidate for this call.
            let candidate = unsafe { &*candidate };
            assert_eq!(candidate.schema_version, 2);
            assert_eq!(candidate.chunk_index, 3);
            assert_eq!(candidate.object_index, 144);
            true
        }

        // SAFETY: This test verifies null handling.
        let null_scan = unsafe {
            scan_discovery_v2(std::ptr::null_mut(), &filter, visitor, std::ptr::null_mut())
        };
        assert_eq!(null_scan, 0);

        let mut missing = UnrealDiscoveryApiV2 {
            scan: None,
            export_record: None,
        };

        // SAFETY: `missing` points to a valid table with absent callbacks.
        let missing_scan =
            unsafe { scan_discovery_v2(&mut missing, &filter, visitor, std::ptr::null_mut()) };
        assert_eq!(missing_scan, 0);

        let mut api = UnrealDiscoveryApiV2 {
            scan: Some(scan_v2_callback),
            export_record: None,
        };

        // SAFETY: `api` points to a valid table with test callbacks.
        let api_scan =
            unsafe { scan_discovery_v2(&mut api, &filter, visitor, std::ptr::null_mut()) };
        assert_eq!(api_scan, 1);
        assert_eq!(DISCOVERY_VISITS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn discovery_export_sanitizes_and_handles_missing() {
        let mut missing = UnrealDiscoveryApiV1 {
            scan: None,
            export_record: None,
        };

        // SAFETY: `missing` points to a valid table with absent callbacks.
        unsafe { export_discovery_record(&mut missing, "combat", "ignored") };

        let mut api = UnrealDiscoveryApiV1 {
            scan: None,
            export_record: Some(export_callback),
        };

        // SAFETY: `api` points to a valid table with a test export callback.
        unsafe { export_discovery_record(&mut api, "combat", "hello\0world") };

        let mut api_v2 = UnrealDiscoveryApiV2 {
            scan: None,
            export_record: Some(export_callback),
        };

        // SAFETY: `api_v2` points to a valid table with a test export callback.
        unsafe { export_discovery_record_v2(&mut api_v2, "combat", "hello\0world") };
    }
}
