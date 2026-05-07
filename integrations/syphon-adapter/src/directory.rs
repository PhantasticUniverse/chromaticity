// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Syphon server directory (singleton) per ADR-021 §4.
//!
//! Phase 3 PR 1 ships the singleton scaffold + the `current_servers`
//! method shape; PR 2 wires the actual `+sharedDirectory`
//! Objective-C class method binding plus the
//! `NSDistributedNotificationCenter` observer that listens for
//! cross-process server-add / server-update / server-remove
//! notifications. The PR 1 scaffold returns an empty server list
//! and serves as the type surface PR 2 fills in.
//!
//! Singleton invariant: chromaticity exposes one
//! [`SyphonServerDirectory`] per process, mirroring the canonical
//! `SyphonServerDirectory`'s `+sharedDirectory` class-level
//! singleton. The framework's discovery model is process-global
//! (one notification observer per process) and chromaticity
//! follows that contract.
//!
//! T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).

use std::sync::OnceLock;

use crate::frames::SyphonServerId;

/// Process-global Syphon server directory.
///
/// Mirrors the canonical `SyphonServerDirectory`'s
/// `+sharedDirectory` class-level singleton per ADR-021 §4. Phase 3
/// PR 1 ships the type surface only; PR 2 wires the actual
/// `+sharedDirectory` Objective-C call plus the
/// `NSDistributedNotificationCenter` observer that maintains the
/// in-memory server list.
///
/// **PR 1 scaffold posture**: [`SyphonServerDirectory::current_servers`]
/// returns an empty list. PR 2 replaces the empty list with the
/// real framework-discovered set.
///
/// T1 evidence: `EV-INTEGRATION-MATRIX` (Syphon arm).
#[derive(Debug)]
pub struct SyphonServerDirectory {
    // Phase 3 PR 1: zero-sized scaffold. PR 2 adds:
    //   - `Retained<NSObject>` handle to the framework's
    //     `+sharedDirectory` instance.
    //   - `NSDistributedNotificationCenter` observer block / token
    //     for the three server lifecycle notifications.
    //   - `Mutex<Vec<SyphonServerId>>` cached server list updated
    //     from the observer block.
    // The private field below blocks external construction so the
    // singleton invariant holds without requiring an opaque newtype
    // wrapper.
    _private: (),
}

impl SyphonServerDirectory {
    /// Return the process-global singleton handle.
    ///
    /// Mirrors the canonical `SyphonServerDirectory`'s
    /// `+sharedDirectory` class method per ADR-021 §4. The handle
    /// is constructed lazily on first call and lives for the
    /// process lifetime; subsequent calls return the same address.
    ///
    /// Phase 3 PR 1: the singleton holds no real framework state;
    /// PR 2 wires the `+sharedDirectory` call and the
    /// `NSDistributedNotificationCenter` observer at first-call
    /// time.
    #[must_use]
    pub fn shared() -> &'static SyphonServerDirectory {
        static INSTANCE: OnceLock<SyphonServerDirectory> = OnceLock::new();
        INSTANCE.get_or_init(|| SyphonServerDirectory { _private: () })
    }

    /// Current set of advertised servers on the local-machine
    /// Syphon bus.
    ///
    /// Phase 3 PR 1 returns an empty list. PR 2 wires the real
    /// implementation: the cached server list is maintained by the
    /// `NSDistributedNotificationCenter` observer block, with
    /// forward-incompat servers
    /// (`dictionary_version > [`crate::frames::KNOWN_DICTIONARY_VERSION`]`)
    /// filtered out per ADR-021 §4 + the canonical
    /// `SyphonClientBase.m:87-93` refusal pattern.
    #[must_use]
    pub fn current_servers(&self) -> Vec<SyphonServerId> {
        Vec::new()
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod directory_pins {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — singleton invariant
    //! and PR 1 scaffold-shape pins.

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

    /// Evidence: EV-INTEGRATION-MATRIX — PR 1 scaffold posture:
    /// the directory returns an empty server list until PR 2
    /// wires the real framework binding.
    #[test]
    fn pr1_scaffold_returns_empty_server_list() {
        let directory = SyphonServerDirectory::shared();
        assert!(
            directory.current_servers().is_empty(),
            "PR 1 scaffold must return an empty server list; PR 2 wires the real implementation"
        );
    }
}
