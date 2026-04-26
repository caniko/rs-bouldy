use super::*;
use crate::state::reset_runtime_for_tests;
use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());
static LAST_LOG: Mutex<Option<String>> = Mutex::new(None);
static TICK_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
static DISCOVERY_VISITS: AtomicUsize = AtomicUsize::new(0);
static EXPORTED_RECORD: Mutex<Option<(String, String)>> = Mutex::new(None);
static TEST_API: UnrealApi = UnrealApi {
    log: test_log,
    get_delta_seconds: test_delta,
};

extern "C" fn test_log(msg: *const c_char) {
    // SAFETY: Test callers pass a valid NUL-terminated string from runtime logging.
    let msg = unsafe { CStr::from_ptr(msg) }
        .to_string_lossy()
        .into_owned();
    *LAST_LOG.lock().unwrap() = Some(msg);
}

extern "C" fn test_delta() -> f32 {
    0.125
}

extern "C" fn test_tick(_delta: f32) {}

extern "C" fn test_shutdown() {}

extern "C" fn register_tick_callback(_callback: TickCallback) {
    TICK_REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn register_shutdown_callback(_callback: ShutdownCallback) {
    SHUTDOWN_REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
}

extern "C" fn scan_callback(
    filter: *const RawDiscoveryFilter,
    visitor: bouldy_sys::DiscoveryVisitor,
    user_data: *mut c_void,
) -> usize {
    assert!(!filter.is_null());
    // SAFETY: The runtime passes a valid filter pointer for the scan call.
    let filter = unsafe { &*filter };
    assert_eq!(filter.term_count, 2);
    assert_eq!(
        filter.kind_mask,
        DISCOVERY_KIND_FUNCTION | DISCOVERY_KIND_PROPERTY
    );
    assert_eq!(filter.max_results, 8);

    let name = CString::new("PerfectParryWindow").unwrap();
    let path = CString::new("/Script/SB.PerfectParryWindow").unwrap();
    let owner = CString::new("SBCombatComponent").unwrap();
    let candidate = RawDiscoveryCandidate {
        kind: DISCOVERY_KIND_FUNCTION,
        name: name.as_ptr(),
        path: path.as_ptr(),
        owner: owner.as_ptr(),
        flags: 42,
    };

    if visitor(&candidate, user_data) {
        DISCOVERY_VISITS.fetch_add(1, Ordering::SeqCst);
    }
    1
}

extern "C" fn scan_v2_callback(
    filter: *const RawDiscoveryFilter,
    visitor: bouldy_sys::DiscoveryVisitorV2,
    user_data: *mut c_void,
) -> usize {
    assert!(!filter.is_null());
    // SAFETY: The runtime passes a valid filter pointer for the scan call.
    let filter = unsafe { &*filter };
    assert_eq!(filter.term_count, 2);
    assert_eq!(
        filter.kind_mask,
        DISCOVERY_KIND_FUNCTION | DISCOVERY_KIND_PROPERTY
    );
    assert_eq!(filter.max_results, 8);

    let name = CString::new("EnemyDodgePunishWindow").unwrap();
    let path = CString::new("/Script/SB.EnemyDodgePunishWindow").unwrap();
    let owner = CString::new("SBEnemyCombatComponent").unwrap();
    let candidate = RawDiscoveryCandidateV2 {
        kind: DISCOVERY_KIND_PROPERTY,
        schema_version: 2,
        name: name.as_ptr(),
        path: path.as_ptr(),
        owner: owner.as_ptr(),
        flags: 77,
        chunk_index: 4,
        object_index: 255,
    };

    if visitor(&candidate, user_data) {
        DISCOVERY_VISITS.fetch_add(1, Ordering::SeqCst);
    }
    1
}

extern "C" fn export_callback(channel: *const c_char, payload: *const c_char) {
    // SAFETY: Test callers pass valid null-terminated strings.
    let channel = unsafe { CStr::from_ptr(channel) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: Test callers pass valid null-terminated strings.
    let payload = unsafe { CStr::from_ptr(payload) }
        .to_string_lossy()
        .into_owned();
    *EXPORTED_RECORD.lock().unwrap() = Some((channel, payload));
}

fn test_api() -> *mut UnrealApi {
    std::ptr::addr_of!(TEST_API).cast_mut()
}

fn test_api_v1() -> UnrealApiV1 {
    UnrealApiV1 {
        base: TEST_API,
        register_tick: Some(register_tick_callback),
        register_shutdown: Some(register_shutdown_callback),
    }
}

fn test_api_v2() -> UnrealApiV2 {
    UnrealApiV2 {
        lifecycle: test_api_v1(),
        discovery: UnrealDiscoveryApiV1 {
            scan: Some(scan_callback),
            export_record: Some(export_callback),
        },
    }
}

fn test_api_v3() -> UnrealApiV3 {
    UnrealApiV3 {
        lifecycle: test_api_v1(),
        discovery: UnrealDiscoveryApiV2 {
            scan: Some(scan_v2_callback),
            export_record: Some(export_callback),
        },
    }
}

fn reset_log() {
    *LAST_LOG.lock().unwrap() = None;
}

fn reset_test_state() {
    reset_runtime_for_tests();
    reset_log();
    TICK_REGISTRATIONS.store(0, Ordering::SeqCst);
    SHUTDOWN_REGISTRATIONS.store(0, Ordering::SeqCst);
    DISCOVERY_VISITS.store(0, Ordering::SeqCst);
    *EXPORTED_RECORD.lock().unwrap() = None;
}

#[test]
fn init_rejects_null_api() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    assert!(!init_with_base_api(std::ptr::null_mut(), |_| {}));
}

#[test]
fn init_calls_mod_body() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    let mut called = false;
    assert!(init_with_base_api(test_api(), |ctx| {
        called = true;
        assert_eq!(ctx.delta_seconds(), 0.125);
        assert!(ctx.discovery().is_none());
    }));
    assert!(called);
}

#[test]
fn init_contains_panics() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    assert!(!init_with_base_api(test_api(), |_| panic!("boom")));
    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod init")
    );
}

#[test]
fn tick_contains_panics() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    assert!(init_with_base_api(test_api(), |_| {}));

    tick_registered_mod(1.0, |_| panic!("boom"));
    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod tick")
    );
}

#[test]
fn global_log_before_init_is_noop() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    log("not initialized");

    assert_eq!(LAST_LOG.lock().unwrap().as_deref(), None);
}

#[test]
fn v1_init_rejects_null_api() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    assert!(!init_with_v1_api(
        std::ptr::null_mut(),
        test_tick,
        test_shutdown,
        |_| {}
    ));
}

#[test]
fn v1_init_calls_mod_body_and_registers_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    let mut api = test_api_v1();

    let mut called = false;
    assert!(init_with_v1_api(
        &mut api,
        test_tick,
        test_shutdown,
        |ctx| {
            called = true;
            assert!(ctx.discovery().is_none());
            ctx.log("v1 initialized");
        },
    ));

    assert!(called);
    assert_eq!(LAST_LOG.lock().unwrap().as_deref(), Some("v1 initialized"));
    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 1);
}

#[test]
fn v1_init_panic_returns_false_and_does_not_register_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    let mut api = test_api_v1();

    assert!(!init_with_v1_api(
        &mut api,
        test_tick,
        test_shutdown,
        |_| panic!("boom")
    ));

    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod V1 init")
    );
    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 0);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn v2_init_rejects_null_api() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    assert!(!init_with_v2_api(
        std::ptr::null_mut(),
        test_tick,
        test_shutdown,
        |_| {}
    ));
}

#[test]
fn v2_init_registers_lifecycle_and_discovery_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    let mut api = test_api_v2();
    assert!(init_with_v2_api(
        &mut api,
        test_tick,
        test_shutdown,
        |ctx| {
            ctx.log("v2 initialized");
            let discovery = ctx.discovery().expect("discovery context");
            let query = DiscoveryQuery::new(["parry", "dodge"])
                .with_kind_mask(DISCOVERY_KIND_FUNCTION | DISCOVERY_KIND_PROPERTY)
                .with_max_results(8);
            let mut seen = Vec::new();
            let count = discovery.scan(&query, |candidate| {
                seen.push(candidate);
                true
            });
            assert_eq!(count, 1);
            assert_eq!(seen.len(), 1);
            assert_eq!(seen[0].name.as_deref(), Some("PerfectParryWindow"));
            discovery.export_record("combat", "{\"kind\":\"function\"}");
        }
    ));

    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(DISCOVERY_VISITS.load(Ordering::SeqCst), 1);
    assert_eq!(
        EXPORTED_RECORD.lock().unwrap().as_ref(),
        Some(&("combat".to_owned(), "{\"kind\":\"function\"}".to_owned()))
    );
}

#[test]
fn v2_init_panic_returns_false_and_does_not_register_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    let mut api = test_api_v2();

    assert!(!init_with_v2_api(
        &mut api,
        test_tick,
        test_shutdown,
        |_| panic!("boom")
    ));

    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod V2 init")
    );
    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 0);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn v3_init_registers_lifecycle_and_indexed_discovery_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();

    let mut api = test_api_v3();
    assert!(init_with_v3_api(
        &mut api,
        test_tick,
        test_shutdown,
        |ctx| {
            ctx.log("v3 initialized");
            let discovery = ctx.discovery().expect("discovery context");
            let query = DiscoveryQuery::new(["parry", "dodge"])
                .with_kind_mask(DISCOVERY_KIND_FUNCTION | DISCOVERY_KIND_PROPERTY)
                .with_max_results(8);
            let mut seen = Vec::new();
            let count = discovery.scan(&query, |candidate| {
                seen.push(candidate);
                true
            });
            assert_eq!(count, 1);
            assert_eq!(seen.len(), 1);
            assert_eq!(seen[0].schema_version, 2);
            assert_eq!(seen[0].name.as_deref(), Some("EnemyDodgePunishWindow"));
            assert_eq!(seen[0].chunk_index, Some(4));
            assert_eq!(seen[0].object_index, Some(255));
            discovery.export_record("combat", "{\"kind\":\"property\"}");
        }
    ));

    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 1);
    assert_eq!(DISCOVERY_VISITS.load(Ordering::SeqCst), 1);
    assert_eq!(
        EXPORTED_RECORD.lock().unwrap().as_ref(),
        Some(&("combat".to_owned(), "{\"kind\":\"property\"}".to_owned()))
    );
}

#[test]
fn v3_init_panic_returns_false_and_does_not_register_callbacks() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    let mut api = test_api_v3();

    assert!(!init_with_v3_api(
        &mut api,
        test_tick,
        test_shutdown,
        |_| panic!("boom")
    ));

    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod V3 init")
    );
    assert_eq!(TICK_REGISTRATIONS.load(Ordering::SeqCst), 0);
    assert_eq!(SHUTDOWN_REGISTRATIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn discovery_query_sanitizes_terms_with_nuls() {
    let query = DiscoveryQuery::new(["parry\0window"]);
    let terms = query.term_cstrings().expect("sanitized terms");
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].as_c_str().to_string_lossy(), "parry window");
}

#[test]
fn shutdown_contains_panics() {
    let _guard = TEST_LOCK.lock().unwrap();
    reset_test_state();
    assert!(init_with_base_api(test_api(), |_| {}));

    shutdown_registered_mod(|| panic!("boom"));

    assert_eq!(
        LAST_LOG.lock().unwrap().as_deref(),
        Some("panic during Rust mod shutdown")
    );
}
