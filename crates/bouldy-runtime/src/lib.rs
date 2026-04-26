//! Safe runtime lifecycle and discovery layer for Bouldy mods.

use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock};

pub use bouldy_sys::{
    DiscoveryCandidate as RawDiscoveryCandidate, DiscoveryFilter as RawDiscoveryFilter,
    DiscoveryKind as RawDiscoveryKind, ShutdownCallback, TickCallback, UnrealApi, UnrealApiV1,
    UnrealApiV2, UnrealDiscoveryApiV1, DISCOVERY_KIND_ANY, DISCOVERY_KIND_CLASS,
    DISCOVERY_KIND_FUNCTION, DISCOVERY_KIND_OBJECT, DISCOVERY_KIND_PROPERTY,
    DISCOVERY_KIND_UNKNOWN,
};

#[derive(Default)]
struct RuntimeState {
    api_addr: Option<usize>,
    discovery_api_addr: Option<usize>,
}

static RUNTIME: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

fn runtime_state() -> &'static Mutex<RuntimeState> {
    RUNTIME.get_or_init(|| Mutex::new(RuntimeState::default()))
}

fn set_api(api: NonNull<UnrealApi>) {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.api_addr = Some(api.as_ptr() as usize);
}

fn set_discovery_api(api: Option<NonNull<UnrealDiscoveryApiV1>>) {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.discovery_api_addr = api.map(|api| api.as_ptr() as usize);
}

fn current_api() -> Option<NonNull<UnrealApi>> {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let addr = state.api_addr?;
    NonNull::new(addr as *mut UnrealApi)
}

fn current_discovery_api() -> Option<NonNull<UnrealDiscoveryApiV1>> {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let addr = state.discovery_api_addr?;
    NonNull::new(addr as *mut UnrealDiscoveryApiV1)
}

#[cfg(test)]
fn reset_runtime_for_tests() {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.api_addr = None;
    state.discovery_api_addr = None;
}

/// Context passed to mod lifecycle callbacks.
pub struct ModContext {
    api: NonNull<UnrealApi>,
}

impl ModContext {
    /// Create a context from a validated API pointer.
    pub fn new(api: NonNull<UnrealApi>) -> Self {
        Self { api }
    }

    /// Log a message through the host shim.
    pub fn log(&mut self, msg: &str) {
        // SAFETY: `ModContext` is only constructed from a non-null API pointer
        // that the host promises remains valid during callbacks.
        unsafe { bouldy_sys::log(self.api.as_ptr(), msg) };
    }

    /// Read delta seconds from the host shim.
    pub fn delta_seconds(&self) -> f32 {
        // SAFETY: Same lifetime contract as `log`.
        unsafe { bouldy_sys::get_delta_seconds(self.api.as_ptr()) }
    }

    /// Return the raw API pointer for lower-level runtime crates.
    ///
    /// Mod authors should not need this in the MVP.
    pub fn raw_api(&self) -> NonNull<UnrealApi> {
        self.api
    }

    /// Return the host discovery API, if this mod was initialized through V2.
    pub fn discovery(&self) -> Option<DiscoveryContext> {
        current_discovery_api().map(DiscoveryContext::new)
    }
}

/// Safe wrapper for the host discovery API.
#[derive(Clone, Copy)]
pub struct DiscoveryContext {
    api: NonNull<UnrealDiscoveryApiV1>,
}

impl DiscoveryContext {
    /// Create a discovery context from a validated host API pointer.
    pub fn new(api: NonNull<UnrealDiscoveryApiV1>) -> Self {
        Self { api }
    }

    /// Scan candidates matching `query`.
    pub fn scan(
        &self,
        query: &DiscoveryQuery,
        mut visitor: impl FnMut(DiscoveryCandidate) -> bool,
    ) -> usize {
        let term_cstrings = match query.term_cstrings() {
            Some(terms) => terms,
            None => return 0,
        };
        let term_ptrs = term_cstrings
            .iter()
            .map(|term| term.as_ptr())
            .collect::<Vec<_>>();
        let filter = RawDiscoveryFilter {
            terms: term_ptrs.as_ptr(),
            term_count: term_ptrs.len(),
            kind_mask: query.kind_mask,
            max_results: query.max_results,
        };

        struct VisitorState<'a> {
            visitor: &'a mut dyn FnMut(DiscoveryCandidate) -> bool,
        }

        extern "C" fn trampoline(
            candidate: *const RawDiscoveryCandidate,
            user_data: *mut c_void,
        ) -> bool {
            // SAFETY: The host discovery backend passes a candidate pointer
            // that is valid for the duration of this visitor call.
            let Some(candidate) = (unsafe { candidate.as_ref() }) else {
                return true;
            };
            // SAFETY: `user_data` is the `VisitorState` pointer supplied by
            // `DiscoveryContext::scan` for this synchronous scan call.
            let Some(state) = (unsafe { (user_data as *mut VisitorState<'_>).as_mut() }) else {
                return false;
            };
            let candidate = DiscoveryCandidate::from_raw(candidate);
            (state.visitor)(candidate)
        }

        let mut visitor_ref = |candidate| visitor(candidate);
        let mut state = VisitorState {
            visitor: &mut visitor_ref,
        };

        // SAFETY: `DiscoveryContext` is constructed from a non-null discovery
        // table. The filter and term C strings remain valid for this call.
        unsafe {
            bouldy_sys::scan_discovery(
                self.api.as_ptr(),
                &filter,
                trampoline,
                std::ptr::addr_of_mut!(state).cast::<c_void>(),
            )
        }
    }

    /// Export a structured discovery record through the host backend.
    pub fn export_record(&self, channel: &str, payload: &str) {
        // SAFETY: `DiscoveryContext` is constructed from a non-null discovery
        // table. Strings are sanitized by `bouldy-sys`.
        unsafe { bouldy_sys::export_discovery_record(self.api.as_ptr(), channel, payload) };
    }
}

/// Query for generic Unreal discovery scans.
#[derive(Debug, Clone)]
pub struct DiscoveryQuery {
    terms: Vec<String>,
    kind_mask: RawDiscoveryKind,
    max_results: usize,
}

impl DiscoveryQuery {
    /// Create a new query from search terms.
    pub fn new(terms: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            terms: terms.into_iter().map(Into::into).collect(),
            kind_mask: DISCOVERY_KIND_ANY,
            max_results: 256,
        }
    }

    /// Set the discovery kind bitmask.
    pub fn with_kind_mask(mut self, kind_mask: RawDiscoveryKind) -> Self {
        self.kind_mask = kind_mask;
        self
    }

    /// Set the maximum result count requested from the host.
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = max_results;
        self
    }

    /// Return query terms.
    pub fn terms(&self) -> &[String] {
        &self.terms
    }

    /// Return the raw discovery kind bitmask.
    pub fn kind_mask(&self) -> RawDiscoveryKind {
        self.kind_mask
    }

    /// Return the requested max result count.
    pub fn max_results(&self) -> usize {
        self.max_results
    }

    fn term_cstrings(&self) -> Option<Vec<CString>> {
        self.terms
            .iter()
            .map(|term| CString::new(term.replace('\0', " ")).ok())
            .collect()
    }
}

/// Owned discovery candidate supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryCandidate {
    /// Candidate kind.
    pub kind: RawDiscoveryKind,
    /// Short display name, if known.
    pub name: Option<String>,
    /// Full path or qualified symbol, if known.
    pub path: Option<String>,
    /// Owning class/package/module, if known.
    pub owner: Option<String>,
    /// Backend-specific flags.
    pub flags: u64,
}

impl DiscoveryCandidate {
    fn from_raw(raw: &RawDiscoveryCandidate) -> Self {
        Self {
            kind: raw.kind,
            name: optional_c_string(raw.name),
            path: optional_c_string(raw.path),
            owner: optional_c_string(raw.owner),
            flags: raw.flags,
        }
    }
}

fn optional_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }

    // SAFETY: Host discovery candidates use null-terminated C strings for the
    // duration of the visitor callback.
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

/// Trait implemented by Rust runtime mods.
pub trait Mod {
    /// Called once when the loader initializes the Rust mod.
    fn on_init(&mut self, _ctx: &mut ModContext) {}

    /// Called by the host shim each frame when V1/V2 tick registration is used.
    fn on_tick(&mut self, _delta: f32) {}

    /// Called by the host shim during shutdown when V1/V2 shutdown registration is used.
    fn on_shutdown(&mut self) {}
}

/// Log through the globally stored API pointer, if initialized.
pub fn log(msg: &str) {
    let Some(api) = current_api() else {
        return;
    };

    // SAFETY: `current_api` only returns a pointer previously accepted by init.
    unsafe { bouldy_sys::log(api.as_ptr(), msg) };
}

/// Run an init boundary with a base API pointer.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn init_with_base_api(api: *mut UnrealApi, init: impl FnOnce(&mut ModContext)) -> bool {
    catch_ffi_bool("panic during Rust mod init", || {
        let Some(api) = NonNull::new(api) else {
            return false;
        };

        set_api(api);
        set_discovery_api(None);
        let mut ctx = ModContext::new(api);
        init(&mut ctx);
        true
    })
}

/// Run an init boundary with a V1 API pointer and register lifecycle callbacks.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn init_with_v1_api(
    api: *mut UnrealApiV1,
    tick_callback: TickCallback,
    shutdown_callback: ShutdownCallback,
    init: impl FnOnce(&mut ModContext),
) -> bool {
    catch_ffi_bool("panic during Rust mod V1 init", || {
        // SAFETY: The caller is an FFI entrypoint. We validate null here, and
        // the host owns the lifetime contract for the pointed-to API table.
        let Some(base_api) = (unsafe { bouldy_sys::base_from_v1(api) }) else {
            return false;
        };

        set_api(base_api);
        set_discovery_api(None);
        let mut ctx = ModContext::new(base_api);
        init(&mut ctx);

        // SAFETY: The host supplied a valid V1 table for this init call.
        unsafe {
            bouldy_sys::register_tick(api, tick_callback);
            bouldy_sys::register_shutdown(api, shutdown_callback);
        }

        true
    })
}

/// Run an init boundary with a V2 API pointer and register lifecycle callbacks.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn init_with_v2_api(
    api: *mut UnrealApiV2,
    tick_callback: TickCallback,
    shutdown_callback: ShutdownCallback,
    init: impl FnOnce(&mut ModContext),
) -> bool {
    catch_ffi_bool("panic during Rust mod V2 init", || {
        // SAFETY: The caller is an FFI entrypoint. We validate null here, and
        // the host owns the lifetime contract for the pointed-to API table.
        let Some(base_api) = (unsafe { bouldy_sys::base_from_v2(api) }) else {
            return false;
        };
        // SAFETY: Same validated V2 table as above.
        let discovery_api = unsafe { bouldy_sys::discovery_from_v2(api) };
        // SAFETY: Same validated V2 table as above.
        let Some(lifecycle_api) = (unsafe { bouldy_sys::lifecycle_from_v2(api) }) else {
            return false;
        };

        set_api(base_api);
        set_discovery_api(discovery_api);
        let mut ctx = ModContext::new(base_api);
        init(&mut ctx);

        // SAFETY: The host supplied a valid V2 table for this init call.
        unsafe {
            bouldy_sys::register_tick(lifecycle_api.as_ptr(), tick_callback);
            bouldy_sys::register_shutdown(lifecycle_api.as_ptr(), shutdown_callback);
        }

        true
    })
}

/// Run a panic-safe tick callback body.
pub fn tick_registered_mod(delta: f32, tick: impl FnOnce(f32)) {
    catch_ffi_unit("panic during Rust mod tick", || tick(delta));
}

/// Run a panic-safe shutdown callback body.
pub fn shutdown_registered_mod(shutdown: impl FnOnce()) {
    catch_ffi_unit("panic during Rust mod shutdown", shutdown);
}

fn catch_ffi_bool(message: &str, f: impl FnOnce() -> bool) -> bool {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => {
            log(message);
            false
        }
    }
}

fn catch_ffi_unit(message: &str, f: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(f)).is_err() {
        log(message);
    }
}

/// Common imports for mod authors.
pub mod prelude {
    pub use bouldy_macros::unreal_mod;

    pub use crate::{
        log, DiscoveryCandidate, DiscoveryContext, DiscoveryQuery, Mod, ModContext, UnrealApi,
        UnrealApiV1, UnrealApiV2,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{c_char, CStr, CString};
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
}
