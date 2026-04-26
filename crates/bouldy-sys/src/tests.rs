use super::*;
use std::ffi::{c_char, c_void, CStr, CString};
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

    extern "C" fn visitor(_candidate: *const DiscoveryCandidate, _user_data: *mut c_void) -> bool {
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

    extern "C" fn visitor(candidate: *const DiscoveryCandidateV2, _user_data: *mut c_void) -> bool {
        assert!(!candidate.is_null());
        // SAFETY: Test callback passes a valid candidate for this call.
        let candidate = unsafe { &*candidate };
        assert_eq!(candidate.schema_version, 2);
        assert_eq!(candidate.chunk_index, 3);
        assert_eq!(candidate.object_index, 144);
        true
    }

    // SAFETY: This test verifies null handling.
    let null_scan =
        unsafe { scan_discovery_v2(std::ptr::null_mut(), &filter, visitor, std::ptr::null_mut()) };
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
    let api_scan = unsafe { scan_discovery_v2(&mut api, &filter, visitor, std::ptr::null_mut()) };
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
