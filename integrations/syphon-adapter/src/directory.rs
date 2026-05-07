// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Syphon server directory (singleton) per ADR-021 §4.
//!
//! Phase 3 PR 2 ships:
//!
//! - The chromaticity-side singleton wrapper (`SyphonServerDirectory`)
//!   used by adapter call sites.
//! - The [`Self::current_servers`] polling API surface that returns
//!   the snapshot of currently advertised servers.
//! - The [`parse_server_description`] helper that converts a
//!   framework-side `NSDictionary<NSString *, id>` into a
//!   [`SyphonServerId`] applying the canonical
//!   `SyphonClientBase.m:87-93` forward-compat refusal filter on
//!   the way through.
//!
//! Phase 3 PR 2 does **not** wire `current_servers()` to the live
//! `[SyphonServerDirectory sharedDirectory].servers` array. The
//! production wiring lives at the engine-side bridge in Phase 3
//! PR 3 (`engine/render/src/syphon_adapter.rs`) which:
//!
//! 1. Owns the Tauri main thread context required by
//!    `NSDistributedNotificationCenter` (per docs/54 Lesson 3.5).
//! 2. Calls
//!    `crate::bindings::SyphonServerDirectoryClass::shared_directory()`
//!    + `.servers()` to obtain the framework-side dictionary array.
//! 3. Iterates the array and feeds each dictionary through
//!    [`parse_server_description`] to produce filtered
//!    [`SyphonServerId`] values.
//! 4. Caches the result in a `Mutex<Vec<SyphonServerId>>` the
//!    chromaticity-side singleton reads from on `current_servers`.
//!
//! The PR 2 `current_servers()` returns an empty list because the
//! cache plumbing in step 4 lives engine-side; T2 corpus fixtures
//! exercise [`parse_server_description`] directly via injected
//! dictionary fixtures (no live framework call required).
//!
//! Singleton invariant: chromaticity exposes one
//! [`SyphonServerDirectory`] per process, mirroring the canonical
//! `SyphonServerDirectory`'s `+sharedDirectory` class-level
//! singleton. The framework's discovery model is process-global
//! (one `NSDistributedNotificationCenter` observer per process)
//! and chromaticity follows that contract.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use std::sync::OnceLock;

use crate::frames::{KNOWN_DICTIONARY_VERSION, SyphonServerId};

/// Process-global Syphon server directory.
///
/// Mirrors the canonical `SyphonServerDirectory`'s
/// `+sharedDirectory` class-level singleton per ADR-021 §4. Phase 3
/// PR 2 ships the type surface + the [`parse_server_description`]
/// helper; PR 3 wires the live call into
/// `crate::bindings::SyphonServerDirectoryClass::shared_directory()`
/// from the Tauri main thread per docs/54 Lesson 3.5.
///
/// **PR 2 scaffold posture**: [`SyphonServerDirectory::current_servers`]
/// returns an empty list because the cache is populated by the
/// engine-side bridge in PR 3.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug)]
pub struct SyphonServerDirectory {
    // The private field below blocks external construction so the
    // singleton invariant holds without requiring an opaque newtype
    // wrapper. PR 3 augments this with a
    // `Mutex<Vec<SyphonServerId>>` cache populated by the engine-
    // side bridge's NSDistributedNotificationCenter observer block.
    _private: (),
}

impl SyphonServerDirectory {
    /// Return the process-global singleton handle.
    #[must_use]
    pub fn shared() -> &'static SyphonServerDirectory {
        static INSTANCE: OnceLock<SyphonServerDirectory> = OnceLock::new();
        INSTANCE.get_or_init(|| SyphonServerDirectory { _private: () })
    }

    /// Current set of advertised servers on the local-machine
    /// Syphon bus.
    ///
    /// Phase 3 PR 2 returns an empty list. PR 3 wires the cache
    /// populated from the engine-side bridge's
    /// `NSDistributedNotificationCenter` observer + the
    /// `[SyphonServerDirectory sharedDirectory].servers` polling
    /// fallback. Forward-incompat servers
    /// (`dictionary_version > KNOWN_DICTIONARY_VERSION`) are filtered
    /// out by [`parse_server_description`] before entering the cache.
    #[must_use]
    pub fn current_servers(&self) -> Vec<SyphonServerId> {
        Vec::new()
    }
}

/// Server-description-dictionary parsing primitive per ADR-021 §4.
///
/// Accepts the four canonical `SyphonServerDescription*Key` values
/// (`UUIDKey`, `NameKey`, `AppNameKey`, `DictionaryVersionKey`)
/// already extracted from the framework-side `NSDictionary` and
/// returns:
///
/// - `Some(SyphonServerId)` if `dictionary_version <= KNOWN_DICTIONARY_VERSION`.
/// - `None` if the dictionary is forward-incompat per the canonical
///   `SyphonClientBase.m:87-93` defensive refusal pattern.
///
/// The extraction step (turning the framework-side `NSDictionary`
/// into the four `String` / `u32` arguments below) lives engine-
/// side at the Phase 3 PR 3 bridge — chromaticity does not link
/// against `objc2-foundation::NSDictionary`-extraction routines from
/// the integrations crate because those routines transitively depend
/// on the framework binary being present at link time, which CI
/// runners cannot guarantee per ADR-021 §10 Negative consequence
/// bullet 2. Splitting the parse step into a String-oriented helper
/// keeps the T2 corpus testable without any objc2 / framework
/// dependency.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
///
/// # Examples
///
/// ```
/// # #[cfg(target_os = "macos")] {
/// use chromaticity_syphon_adapter::directory::parse_server_description;
///
/// // Compatible server: dictionary_version 0 == KNOWN_DICTIONARY_VERSION.
/// let result = parse_server_description(
///     "uuid-1234".to_string(),
///     "Channel A".to_string(),
///     "TouchDesigner".to_string(),
///     0,
/// );
/// assert!(result.is_some());
///
/// // Forward-incompat server: dictionary_version 1 > KNOWN_DICTIONARY_VERSION.
/// let refused = parse_server_description(
///     "uuid-5678".to_string(),
///     "Channel B".to_string(),
///     "FutureApp".to_string(),
///     1,
/// );
/// assert!(refused.is_none());
/// # }
/// ```
#[must_use]
// Clippy flags `<=` against `u32` minimum (0) as
// `absurd_extreme_comparisons` because today
// `KNOWN_DICTIONARY_VERSION == 0`. The comparison is intentional:
// ADR-021 §Rollout review-cadence bullet (d) anticipates a framework
// upgrade that bumps `KNOWN_DICTIONARY_VERSION`, after which the
// inequality stops being trivially true. Suppressing the lint at the
// call site keeps the predicate honest about its forward-compat
// intent; using `==` would silently break the day the constant rises.
#[allow(clippy::absurd_extreme_comparisons)]
pub fn parse_server_description(
    uuid: String,
    name: String,
    app_name: String,
    dictionary_version: u32,
) -> Option<SyphonServerId> {
    if dictionary_version > KNOWN_DICTIONARY_VERSION {
        return None;
    }
    Some(SyphonServerId {
        uuid,
        name,
        app_name,
        dictionary_version,
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod directory_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — singleton invariant
    //! and PR 2 parse-helper-shape pins.

    use super::*;

    /// Evidence: EV-INTEGRATION-MATRIX — singleton invariant: two
    /// successive `shared()` calls return the same address per
    /// ADR-021 §4 (mirrors canonical `+sharedDirectory`).
    #[test]
    fn shared_directory_returns_stable_address() {
        let first = SyphonServerDirectory::shared() as *const _;
        let second = SyphonServerDirectory::shared() as *const _;
        assert_eq!(
            first, second,
            "shared() must return the same singleton address across calls"
        );
    }

    /// Evidence: EV-INTEGRATION-MATRIX — PR 2 scaffold posture: the
    /// directory returns an empty server list until PR 3 wires the
    /// engine-side cache from the live framework polling.
    #[test]
    fn pr2_scaffold_returns_empty_server_list() {
        let directory = SyphonServerDirectory::shared();
        assert!(
            directory.current_servers().is_empty(),
            "PR 2 scaffold must return an empty server list; PR 3 wires the engine-side cache"
        );
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `parse_server_description`
    /// accepts dictionary_version 0 (the canonical
    /// `kSyphonDictionaryVersion`).
    #[test]
    fn parse_accepts_known_dictionary_version() {
        let result = parse_server_description(
            "uuid".to_string(),
            "name".to_string(),
            "app".to_string(),
            KNOWN_DICTIONARY_VERSION,
        );
        assert!(result.is_some());
        let id = result.unwrap();
        assert_eq!(id.dictionary_version, KNOWN_DICTIONARY_VERSION);
    }

    /// Evidence: EV-INTEGRATION-MATRIX — `parse_server_description`
    /// refuses dictionary_version > KNOWN_DICTIONARY_VERSION per the
    /// canonical `SyphonClientBase.m:87-93` defensive forward-compat
    /// refusal pattern.
    #[test]
    fn parse_refuses_forward_incompat_dictionary_version() {
        let result = parse_server_description(
            "uuid".to_string(),
            "name".to_string(),
            "app".to_string(),
            KNOWN_DICTIONARY_VERSION + 1,
        );
        assert!(result.is_none());
    }
}
