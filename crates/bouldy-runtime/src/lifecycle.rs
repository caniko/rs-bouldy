//! Safe lifecycle entrypoint boundaries and panic containment.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr::NonNull;

use crate::context::ModContext;
use crate::state::{current_api, set_api, set_discovery_v1_api, set_discovery_v2_api};
use crate::{ShutdownCallback, TickCallback, UnrealApi, UnrealApiV1, UnrealApiV2, UnrealApiV3};

/// Trait implemented by Rust runtime mods.
pub trait Mod {
    /// Called once when the loader initializes the Rust mod.
    fn on_init(&mut self, _ctx: &mut ModContext) {}

    /// Called by the host shim each frame when V1/V2/V3 tick registration is used.
    fn on_tick(&mut self, _delta: f32) {}

    /// Called by the host shim during shutdown when V1/V2/V3 shutdown registration is used.
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
        set_discovery_v1_api(None);
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
        set_discovery_v1_api(None);
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
        set_discovery_v1_api(discovery_api);
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

/// Run an init boundary with a V3 API pointer and register lifecycle callbacks.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn init_with_v3_api(
    api: *mut UnrealApiV3,
    tick_callback: TickCallback,
    shutdown_callback: ShutdownCallback,
    init: impl FnOnce(&mut ModContext),
) -> bool {
    catch_ffi_bool("panic during Rust mod V3 init", || {
        // SAFETY: The caller is an FFI entrypoint. We validate null here, and
        // the host owns the lifetime contract for the pointed-to API table.
        let Some(base_api) = (unsafe { bouldy_sys::base_from_v3(api) }) else {
            return false;
        };
        // SAFETY: Same validated V3 table as above.
        let discovery_api = unsafe { bouldy_sys::discovery_from_v3(api) };
        // SAFETY: Same validated V3 table as above.
        let Some(lifecycle_api) = (unsafe { bouldy_sys::lifecycle_from_v3(api) }) else {
            return false;
        };

        set_api(base_api);
        set_discovery_v2_api(discovery_api);
        let mut ctx = ModContext::new(base_api);
        init(&mut ctx);

        // SAFETY: The host supplied a valid V3 table for this init call.
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
