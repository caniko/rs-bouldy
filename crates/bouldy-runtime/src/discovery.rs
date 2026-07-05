//! Safe wrappers for host discovery scans and candidates.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr::NonNull;

use crate::state::DiscoveryApi;
use crate::{
    RawDiscoveryCandidate, RawDiscoveryCandidateV2, RawDiscoveryFilter, RawDiscoveryKind,
    UnrealDiscoveryApiV1, UnrealDiscoveryApiV2, DISCOVERY_KIND_ANY,
};

/// Safe wrapper for the host discovery API.
#[derive(Clone, Copy)]
pub struct DiscoveryContext {
    api: DiscoveryApi,
}

impl DiscoveryContext {
    pub(crate) fn from_api(api: DiscoveryApi) -> Self {
        Self { api }
    }

    /// Create a discovery context from a validated V1 host API pointer.
    pub fn new(api: NonNull<UnrealDiscoveryApiV1>) -> Self {
        Self {
            api: DiscoveryApi::V1(api),
        }
    }

    /// Create a discovery context from a validated V2 host API pointer.
    pub fn new_v2(api: NonNull<UnrealDiscoveryApiV2>) -> Self {
        Self {
            api: DiscoveryApi::V2(api),
        }
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

        let mut visitor_ref = |candidate| visitor(candidate);
        let mut state = VisitorState {
            visitor: &mut visitor_ref,
        };

        match self.api {
            DiscoveryApi::V1(api) => scan_v1(api, &filter, &mut state),
            DiscoveryApi::V2(api) => scan_v2(api, &filter, &mut state),
        }
    }

    /// Export a structured discovery record through the host backend.
    pub fn export_record(&self, channel: &str, payload: &str) {
        match self.api {
            DiscoveryApi::V1(api) => {
                // SAFETY: `DiscoveryContext` is constructed from a non-null
                // discovery table. Strings are sanitized by `bouldy-sys`.
                unsafe { bouldy_sys::export_discovery_record(api.as_ptr(), channel, payload) };
            }
            DiscoveryApi::V2(api) => {
                // SAFETY: `DiscoveryContext` is constructed from a non-null
                // discovery table. Strings are sanitized by `bouldy-sys`.
                unsafe { bouldy_sys::export_discovery_record_v2(api.as_ptr(), channel, payload) };
            }
        }
    }

    /// Scan candidates, export matching candidates as stable JSON records, and
    /// return the number of exported records.
    pub fn scan_and_export_json_records(
        &self,
        query: &DiscoveryQuery,
        channel: &str,
        mut predicate: impl FnMut(&DiscoveryCandidate) -> bool,
    ) -> usize {
        let mut exported = 0usize;
        self.scan(query, |candidate| {
            if predicate(&candidate) {
                self.export_record(channel, &candidate.record_json());
                exported = exported.saturating_add(1);
            }
            true
        });
        exported
    }
}

struct VisitorState<'a> {
    visitor: &'a mut dyn FnMut(DiscoveryCandidate) -> bool,
}

fn scan_v1(
    api: NonNull<UnrealDiscoveryApiV1>,
    filter: &RawDiscoveryFilter,
    state: &mut VisitorState<'_>,
) -> usize {
    // SAFETY: `DiscoveryContext` is constructed from a non-null discovery
    // table. The filter and term C strings remain valid for this call.
    unsafe {
        bouldy_sys::scan_discovery(
            api.as_ptr(),
            filter,
            trampoline_v1,
            std::ptr::from_mut(state).cast::<c_void>(),
        )
    }
}

fn scan_v2(
    api: NonNull<UnrealDiscoveryApiV2>,
    filter: &RawDiscoveryFilter,
    state: &mut VisitorState<'_>,
) -> usize {
    // SAFETY: `DiscoveryContext` is constructed from a non-null discovery
    // table. The filter and term C strings remain valid for this call.
    unsafe {
        bouldy_sys::scan_discovery_v2(
            api.as_ptr(),
            filter,
            trampoline_v2,
            std::ptr::from_mut(state).cast::<c_void>(),
        )
    }
}

extern "C" fn trampoline_v1(
    candidate: *const RawDiscoveryCandidate,
    user_data: *mut c_void,
) -> bool {
    // SAFETY: The host discovery backend passes a candidate pointer that is
    // valid for the duration of this visitor call.
    let Some(candidate) = (unsafe { candidate.as_ref() }) else {
        return true;
    };
    // SAFETY: `user_data` is the `VisitorState` pointer supplied by
    // `DiscoveryContext::scan` for this synchronous scan call.
    let Some(state) = (unsafe { (user_data as *mut VisitorState<'_>).as_mut() }) else {
        return false;
    };
    (state.visitor)(DiscoveryCandidate::from_raw(candidate))
}

extern "C" fn trampoline_v2(
    candidate: *const RawDiscoveryCandidateV2,
    user_data: *mut c_void,
) -> bool {
    // SAFETY: The host discovery backend passes a candidate pointer that is
    // valid for the duration of this visitor call.
    let Some(candidate) = (unsafe { candidate.as_ref() }) else {
        return true;
    };
    // SAFETY: `user_data` is the `VisitorState` pointer supplied by
    // `DiscoveryContext::scan` for this synchronous scan call.
    let Some(state) = (unsafe { (user_data as *mut VisitorState<'_>).as_mut() }) else {
        return false;
    };
    (state.visitor)(DiscoveryCandidate::from_raw_v2(candidate))
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

    pub(crate) fn term_cstrings(&self) -> Option<Vec<CString>> {
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
    /// Candidate schema version from the host discovery backend.
    pub schema_version: u32,
    /// Unreal object array chunk index, when known.
    pub chunk_index: Option<i32>,
    /// Unreal object index within the chunk/global object array, when known.
    pub object_index: Option<i32>,
}

impl DiscoveryCandidate {
    fn from_raw(raw: &RawDiscoveryCandidate) -> Self {
        Self {
            kind: raw.kind,
            name: optional_c_string(raw.name),
            path: optional_c_string(raw.path),
            owner: optional_c_string(raw.owner),
            flags: raw.flags,
            schema_version: 1,
            chunk_index: None,
            object_index: None,
        }
    }

    fn from_raw_v2(raw: &RawDiscoveryCandidateV2) -> Self {
        Self {
            kind: raw.kind,
            name: optional_c_string(raw.name),
            path: optional_c_string(raw.path),
            owner: optional_c_string(raw.owner),
            flags: raw.flags,
            schema_version: raw.schema_version,
            chunk_index: non_negative_index(raw.chunk_index),
            object_index: non_negative_index(raw.object_index),
        }
    }

    /// Return whether any term appears in the candidate name, path, or owner.
    ///
    /// Matching is ASCII case-insensitive and intended for broad discovery
    /// reconnaissance terms, not locale-aware text search.
    pub fn matches_any_term<'a>(&self, terms: impl IntoIterator<Item = &'a str>) -> bool {
        let haystack = [
            self.name.as_deref().unwrap_or_default(),
            self.path.as_deref().unwrap_or_default(),
            self.owner.as_deref().unwrap_or_default(),
        ]
        .join(" ")
        .to_ascii_lowercase();

        terms
            .into_iter()
            .any(|term| haystack.contains(&term.to_ascii_lowercase()))
    }

    /// Format a stable JSON discovery record without adding a JSON dependency.
    pub fn record_json(&self) -> String {
        format!(
            "{{\"schema_version\":{},\"kind\":{},\"name\":{},\"path\":{},\"owner\":{},\"flags\":{},\"chunk_index\":{},\"object_index\":{}}}",
            self.schema_version,
            self.kind,
            json_string(self.name.as_deref()),
            json_string(self.path.as_deref()),
            json_string(self.owner.as_deref()),
            self.flags,
            json_i32(self.chunk_index),
            json_i32(self.object_index)
        )
    }
}

fn non_negative_index(index: i32) -> Option<i32> {
    (index >= 0).then_some(index)
}

fn json_i32(value: Option<i32>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn json_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", json_escape(value)),
        None => "null".to_owned(),
    }
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            ch if ch.is_control() => " ".chars().collect::<Vec<_>>(),
            ch => vec![ch],
        })
        .collect::<String>()
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
