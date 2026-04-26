//! Raw discovery ABI and helpers.

use std::ffi::{c_char, c_void, CString};
use std::ptr::NonNull;

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
