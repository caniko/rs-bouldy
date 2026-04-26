//! Runtime callback context passed to mod implementations.

use std::ptr::NonNull;

use crate::discovery::DiscoveryContext;
use crate::state::current_discovery_api;
use crate::UnrealApi;

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

    /// Return the host discovery API, if this mod was initialized through V2 or V3.
    pub fn discovery(&self) -> Option<DiscoveryContext> {
        current_discovery_api().map(DiscoveryContext::from_api)
    }
}
