//! Safe runtime lifecycle and discovery layer for Bouldy mods.

mod context;
mod discovery;
mod lifecycle;
mod state;

pub use bouldy_sys::{
    DiscoveryCandidate as RawDiscoveryCandidate, DiscoveryCandidateV2 as RawDiscoveryCandidateV2,
    DiscoveryFilter as RawDiscoveryFilter, DiscoveryKind as RawDiscoveryKind, ShutdownCallback,
    TickCallback, UnrealApi, UnrealApiV1, UnrealApiV2, UnrealApiV3, UnrealDiscoveryApiV1,
    UnrealDiscoveryApiV2, DISCOVERY_KIND_ANY, DISCOVERY_KIND_CLASS, DISCOVERY_KIND_FUNCTION,
    DISCOVERY_KIND_OBJECT, DISCOVERY_KIND_PROPERTY, DISCOVERY_KIND_UNKNOWN,
};
pub use context::ModContext;
pub use discovery::{DiscoveryCandidate, DiscoveryContext, DiscoveryQuery};
pub use lifecycle::{
    init_with_base_api, init_with_v1_api, init_with_v2_api, init_with_v3_api, log,
    shutdown_registered_mod, tick_registered_mod, Mod,
};

/// Common imports for mod authors.
pub mod prelude {
    pub use bouldy_macros::unreal_mod;

    pub use crate::{
        log, DiscoveryCandidate, DiscoveryContext, DiscoveryQuery, Mod, ModContext, UnrealApi,
        UnrealApiV1, UnrealApiV2, UnrealApiV3,
    };
}

#[cfg(test)]
mod tests;
