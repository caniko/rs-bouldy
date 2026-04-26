//! Process-global runtime pointers installed by the loader.

use std::ptr::NonNull;
use std::sync::{Mutex, OnceLock};

use crate::{UnrealApi, UnrealDiscoveryApiV1, UnrealDiscoveryApiV2};

#[derive(Default)]
struct RuntimeState {
    api_addr: Option<usize>,
    discovery_v1_api_addr: Option<usize>,
    discovery_v2_api_addr: Option<usize>,
}

static RUNTIME: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

fn runtime_state() -> &'static Mutex<RuntimeState> {
    RUNTIME.get_or_init(|| Mutex::new(RuntimeState::default()))
}

#[derive(Clone, Copy)]
pub(crate) enum DiscoveryApi {
    V1(NonNull<UnrealDiscoveryApiV1>),
    V2(NonNull<UnrealDiscoveryApiV2>),
}

pub(crate) fn set_api(api: NonNull<UnrealApi>) {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.api_addr = Some(api.as_ptr() as usize);
}

pub(crate) fn set_discovery_v1_api(api: Option<NonNull<UnrealDiscoveryApiV1>>) {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.discovery_v1_api_addr = api.map(|api| api.as_ptr() as usize);
    state.discovery_v2_api_addr = None;
}

pub(crate) fn set_discovery_v2_api(api: Option<NonNull<UnrealDiscoveryApiV2>>) {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.discovery_v1_api_addr = None;
    state.discovery_v2_api_addr = api.map(|api| api.as_ptr() as usize);
}

pub(crate) fn current_api() -> Option<NonNull<UnrealApi>> {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let addr = state.api_addr?;
    NonNull::new(addr as *mut UnrealApi)
}

pub(crate) fn current_discovery_api() -> Option<DiscoveryApi> {
    let state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    if let Some(addr) = state.discovery_v2_api_addr {
        return NonNull::new(addr as *mut UnrealDiscoveryApiV2).map(DiscoveryApi::V2);
    }
    let addr = state.discovery_v1_api_addr?;
    NonNull::new(addr as *mut UnrealDiscoveryApiV1).map(DiscoveryApi::V1)
}

#[cfg(test)]
pub(crate) fn reset_runtime_for_tests() {
    let mut state = runtime_state()
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    state.api_addr = None;
    state.discovery_v1_api_addr = None;
    state.discovery_v2_api_addr = None;
}
