//! Safe runtime lifecycle layer for Bouldy mods.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock};

pub use bouldy_sys::{ShutdownCallback, TickCallback, UnrealApi, UnrealApiV1};

#[derive(Default)]
struct RuntimeState {
    api_addr: Option<usize>,
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

fn current_api() -> Option<NonNull<UnrealApi>> {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let addr = state.api_addr?;
    NonNull::new(addr as *mut UnrealApi)
}

#[cfg(test)]
fn reset_runtime_for_tests() {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.api_addr = None;
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
}

/// Trait implemented by Rust runtime mods.
pub trait Mod {
    /// Called once when the loader initializes the Rust mod.
    fn on_init(&mut self, _ctx: &mut ModContext) {}

    /// Called by the host shim each frame when V1 tick registration is used.
    fn on_tick(&mut self, _delta: f32) {}

    /// Called by the host shim during shutdown when V1 shutdown registration is used.
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

    pub use crate::{log, Mod, ModContext, UnrealApi, UnrealApiV1};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{c_char, CStr};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static LAST_LOG: Mutex<Option<String>> = Mutex::new(None);
    static TICK_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWN_REGISTRATIONS: AtomicUsize = AtomicUsize::new(0);
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

    fn reset_log() {
        *LAST_LOG.lock().unwrap() = None;
    }

    fn reset_test_state() {
        reset_runtime_for_tests();
        reset_log();
        TICK_REGISTRATIONS.store(0, Ordering::SeqCst);
        SHUTDOWN_REGISTRATIONS.store(0, Ordering::SeqCst);
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
