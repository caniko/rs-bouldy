//! Raw C ABI bridge definitions for Bouldy.
//!
//! This crate intentionally contains only minimal loader-facing ABI types. It
//! does not expose Unreal SDK bindings, generated game types, or asset tooling.

mod discovery;
mod lifecycle;

pub use discovery::{
    export_discovery_record, export_discovery_record_v2, scan_discovery, scan_discovery_v2,
    DiscoveryCandidate, DiscoveryCandidateV2, DiscoveryExportFn, DiscoveryFilter, DiscoveryKind,
    DiscoveryScanFn, DiscoveryScanV2Fn, DiscoveryVisitor, DiscoveryVisitorV2, UnrealDiscoveryApiV1,
    UnrealDiscoveryApiV2, DISCOVERY_KIND_ANY, DISCOVERY_KIND_CLASS, DISCOVERY_KIND_FUNCTION,
    DISCOVERY_KIND_OBJECT, DISCOVERY_KIND_PROPERTY, DISCOVERY_KIND_UNKNOWN,
};
pub use lifecycle::{
    base_from_v1, base_from_v2, base_from_v3, discovery_from_v2, discovery_from_v3,
    get_delta_seconds, lifecycle_from_v2, lifecycle_from_v3, log, register_shutdown, register_tick,
    ShutdownCallback, TickCallback, UnrealApi, UnrealApiV1, UnrealApiV2, UnrealApiV3,
};

#[cfg(test)]
mod tests;
