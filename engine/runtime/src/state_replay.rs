// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 slice-1 EV-STATE-REPLAY canonical state replay + hash invariant.
//!
//! Provides [`replay_canonical_hash`]: takes an event sequence,
//! applies each event to a fresh [`State`] via the crate-private
//! [`State::apply_event`] path (the same code-path the
//! [`crate::InMemoryBus::commit_all_into`] commit-thread helper
//! drives), canonical-binary-encodes the resulting state, and
//! returns the FNV-1a-64 hash. The integration test corpus in
//! `engine/runtime/tests/state_replay/main.rs` calls this helper
//! with seeded command sequences and asserts cross-trial +
//! cross-commit hash convergence.
//!
//! # Why this lives in-crate
//!
//! [`State`] sub-store fields are `pub(crate)` per ADR-006 to keep
//! mutation flow inside the state commit thread. The canonical-
//! encoding pass needs deterministic field iteration over the
//! `HashMap`-backed sub-stores (sorted by key); doing that in an
//! integration test would require widening field visibility, which
//! breaks the ADR-006 invariant. Exposing only the hash through a
//! `pub fn` that owns the whole replay-and-hash flow keeps the
//! visibility contract intact while still letting the
//! `EV-STATE-REPLAY` T2 corpus pin a canonical state hash across
//! commits.
//!
//! # Canonical-binary encoding (normative for the golden hashes)
//!
//! The encoding is deliberately distinct from `Debug` output so
//! that incidental `Debug` impl drift across crates can't shift
//! the goldens. Format (matches the EV-DETERMINISM corpus's
//! event-encoding shape so future cross-row diffing is feasible):
//!
//! ```text
//! state    = be_u64(frame)
//!         || control_store
//!         || preset_store
//!         || playlist_store
//!         || snap_store
//!         || mapping_store
//!
//! control_store  = be_u32(count) || sorted_pairs[ be_u32(id.0) || be_u32(value.0.to_bits()) ]
//! preset_store   = u8(is_some) || [ be_u32(id.0) ]
//! playlist_store = be_i64(position)
//! snap_store     = u8(active_is_some) || [ u8(active.get()) ]
//!                || be_u32(saved_count) || sorted_slots[ u8(slot.get()) ]
//! mapping_store  = be_u32(count) || sorted_pairs[ be_u32(id.0) || source_tag_u8 ]
//!     source_tag 0x01 Osc, 0x02 Midi
//! ```
//!
//! Sorting deterministically (`HashMap` iteration order is
//! deliberately unspecified) is the load-bearing invariant on the
//! controlled-encoding side; FNV-1a-64 is non-cryptographic and
//! the only property consumed is `(input bytes) -> stable u64`.
//!
//! New sub-store fields, new [`crate::Command`] / [`EventPayload`]
//! variants, or new [`crate::MappingSource`] variants ALL require
//! a corresponding extension here AND a regeneration of the T2
//! goldens — the panic-arm wildcards on the encoder match
//! statements force the extension at runtime. Phase-1
//! `EV-DETERMINISM` made the same forcing choice for the
//! event-stream encoder.

use crate::command_bus::{Event, MappingSource};
use crate::state::State;

/// FNV-1a 64 offset basis. Matches the Phase-1 `EV-DETERMINISM`
/// corpus constant verbatim so cross-row corpus diffing is feasible.
const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
/// FNV-1a 64 prime. Matches the Phase-1 `EV-DETERMINISM` corpus
/// constant verbatim.
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// Apply `events` to a fresh [`State`] via [`State::apply_event`],
/// canonical-binary-encode the resulting state per the module-prose
/// shape, and return the FNV-1a-64 hash.
///
/// The integration corpus calls this helper after dispatching
/// commands on a fresh [`crate::InMemoryBus`] and committing the
/// events out via [`crate::InMemoryBus::commit_all_into`]; the
/// helper re-applies the same events to a SECOND fresh state and
/// hashes that. This is the canonical state-replay invariant: the
/// committed events are themselves a sufficient log for
/// reconstructing the canonical state from any fresh starting
/// point.
#[must_use]
pub fn replay_canonical_hash(events: &[Event]) -> u64 {
    let mut state = State::default();
    for event in events {
        state.apply_event(event);
    }
    canonical_state_hash(&state)
}

/// Canonical-binary-encode `state` per the module-prose shape and
/// return the FNV-1a-64 hash. Public so a host can hash a state it
/// already owns (for diagnostics) without going through the
/// replay path; the corpus uses [`replay_canonical_hash`] which
/// owns the full event-replay flow.
#[must_use]
pub fn canonical_state_hash(state: &State) -> u64 {
    let mut buf = Vec::with_capacity(default_buffer_capacity(state));
    encode_state_canonical(state, &mut buf);
    fnv1a_64(&buf)
}

/// Conservative initial buffer capacity heuristic: 64 bytes for the
/// fixed-size store header + 8 bytes per (id, value) pair across the
/// HashMap-backed sub-stores. Avoids reallocation on the typical
/// fixture sizes (corpus seeds at 50-1024 commands, producing
/// hundreds of distinct ids).
fn default_buffer_capacity(state: &State) -> usize {
    let pairs = state.control_store.values.len()
        + state.mapping_store.bindings.len()
        + state.snap_store.saved.len();
    64 + pairs * 8
}

/// Canonical-binary-encode `state` per the module-prose shape.
///
/// `pub(crate)` so the sibling [`crate::state_migration`] module
/// can wrap this output in a versioned envelope without re-
/// implementing the encoder. The output shape is the EV-STATE-
/// REPLAY contract documented above; no rows depending on this
/// shape may drift without updating the goldens in row 32 / row
/// 34 / row 33 corpora.
///
/// # Intentionally NOT observed
///
/// Several [`State`] fields are intentionally NOT observed by this
/// encoder so the EV-STATE-REPLAY (row 32), EV-PRESET-PLAYLIST
/// (row 34), and EV-STATE-MIGRATION (row 33) goldens remain
/// invariant under their addition:
///
/// - `latest_audio_snapshot` (row 65 IPC bridge): scheduler-tick
///   transient; repopulates on the next pull.
/// - `snapshot_metadata` (row 33 envelope trailer): observed only
///   on the [`crate::state_migration`] envelope path.
/// - `session_bindings` (row 80 class-d, Wave 4d.3 PR 5): consumed
///   by [`crate::crash_resume::restore_class_d`] only; adapter-
///   side rebinding lives outside the canonical-binary
///   contract.
/// - `preset_store.banks` (Wave 2.4 PERF.C): in-process preset
///   bank snapshots; preset-bank durability + on-disk persistence
///   remain Wave 2.5+ scope per the Phase-3 `EV-PRESET-PLAYLIST`
///   narrowing. Encoding the banks would drift the row 32 / row
///   34 / row 33 goldens under the additive widening; the
///   `PresetRecall` apply-event path restores controls from the
///   bank in-process and the resulting `ControlStore` mutation
///   IS observed by the canonical encoder, so any test that
///   exercises the save+recall round-trip sees a meaningful
///   downstream hash change without the bank itself being on the
///   canonical wire.
pub(crate) fn encode_state_canonical(state: &State, out: &mut Vec<u8>) {
    // be_u64(frame).
    out.extend_from_slice(&state.frame.to_be_bytes());

    encode_control_store(state, out);
    encode_preset_store(state, out);
    encode_playlist_store(state, out);
    encode_snap_store(state, out);
    encode_mapping_store(state, out);
    // `state.session_bindings` (row 80 class-d, Wave 4d.3 PR 5) is
    // intentionally NOT encoded: matches the
    // `latest_audio_snapshot` precedent — adding it to the
    // canonical-binary contract would drift row 32 / row 34 / row
    // 33 goldens. Class-d restoration consumes the field directly
    // via `crash_resume::restore_class_d`.
}

fn encode_control_store(state: &State, out: &mut Vec<u8>) {
    let mut pairs: Vec<(u32, u32)> = state
        .control_store
        .values
        .iter()
        .map(|(id, value)| (id.0, value.0.to_bits()))
        .collect();
    pairs.sort_by_key(|&(id, _)| id);
    // Saturating cast: state stores hundreds of entries in practice;
    // a hypothetical >4G-entry state would truncate the length prefix
    // but every element still gets encoded, so the hash remains a
    // unique fingerprint.
    let len = pairs.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    for (id, bits) in pairs {
        out.extend_from_slice(&id.to_be_bytes());
        out.extend_from_slice(&bits.to_be_bytes());
    }
}

fn encode_preset_store(state: &State, out: &mut Vec<u8>) {
    match state.preset_store.active {
        Some(id) => {
            out.push(0x01);
            out.extend_from_slice(&id.0.to_be_bytes());
        }
        None => out.push(0x00),
    }
}

fn encode_playlist_store(state: &State, out: &mut Vec<u8>) {
    out.extend_from_slice(&state.playlist_store.position.to_be_bytes());
}

fn encode_snap_store(state: &State, out: &mut Vec<u8>) {
    match state.snap_store.active {
        Some(slot) => {
            out.push(0x01);
            out.push(slot.get());
        }
        None => out.push(0x00),
    }
    let mut saved: Vec<u8> = state.snap_store.saved.iter().map(|s| s.get()).collect();
    saved.sort_unstable();
    let len = saved.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&saved);
}

fn encode_mapping_store(state: &State, out: &mut Vec<u8>) {
    let mut pairs: Vec<(u32, u8)> = state
        .mapping_store
        .bindings
        .iter()
        .map(|(id, source)| (id.0, source_tag(*source)))
        .collect();
    pairs.sort_by_key(|&(id, _)| id);
    let len = pairs.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    for (id, tag) in pairs {
        out.extend_from_slice(&id.to_be_bytes());
        out.push(tag);
    }
}

fn source_tag(source: MappingSource) -> u8 {
    // Within the defining crate `#[non_exhaustive]` does not require
    // a wildcard; the compiler-enforced exhaustiveness check forces
    // any new `MappingSource` variant to land a tag byte here AND a
    // T2 corpus regeneration. Phase-1 EV-DETERMINISM's encoder uses
    // a wildcard panic because it lives in a sibling integration-test
    // crate; this file is in `engine/runtime/src/` so the strict
    // exhaustive form is preferable.
    match source {
        MappingSource::Osc => 0x01,
        MappingSource::Midi => 0x02,
    }
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod state_replay_pins {
    //! Evidence: EV-STATE-REPLAY (T1) — canonical state-replay
    //! drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins on
    //! the canonical-encoding shape and the replay invariant that
    //! the T2 corpus in `engine/runtime/tests/state_replay/main.rs`
    //! relies on. Catalog drift on either surface fires the moment
    //! `cargo test` runs, before the T2 corpus would catch the same
    //! regression as a noisier multi-fixture failure.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 32 — `EV-STATE-REPLAY`.
    //! - `docs/adr/ADR-004-state-model.md` — canonical state model.
    //! - `docs/adr/ADR-006-thread-and-process-model.md` — state commit thread.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

    use super::{canonical_state_hash, replay_canonical_hash};
    use crate::command_bus::{
        ControlId, ControlValue, Event, EventPayload, EventSeq, MappingId, MappingSource, PresetId,
        SnapSlotId,
    };
    use crate::state::State;

    #[test]
    fn empty_event_stream_replay_matches_default_state_hash() {
        // Drift pin: replaying zero events into a fresh state must
        // produce the same canonical hash as hashing a fresh
        // `State::default()` directly. The replay path and the
        // direct path agree on the empty baseline.
        let direct = canonical_state_hash(&State::default());
        let replay = replay_canonical_hash(&[]);
        assert_eq!(direct, replay, "empty-event-stream replay invariant");
    }

    #[test]
    fn replay_is_pure_same_events_same_hash() {
        // Drift pin: replaying the same event slice twice must
        // produce the same hash. This is the load-bearing
        // determinism invariant for the T2 corpus.
        let events = sample_event_stream();
        let h1 = replay_canonical_hash(&events);
        let h2 = replay_canonical_hash(&events);
        assert_eq!(h1, h2, "replay must be a pure function of the event stream",);
    }

    #[test]
    fn replay_diverges_for_distinct_event_streams() {
        // Drift pin: distinct event streams MUST hash differently.
        // A trivially-broken canonical encoder that returned
        // `FNV_OFFSET` for every input would pass the empty +
        // pure-function pins but trip here.
        let stream_a = sample_event_stream();
        let mut stream_b = sample_event_stream();
        stream_b.push(Event {
            seq: EventSeq(stream_a.len() as u64),
            payload: EventPayload::PlaylistAdvanced { step: 7 },
        });
        let ha = replay_canonical_hash(&stream_a);
        let hb = replay_canonical_hash(&stream_b);
        assert_ne!(ha, hb, "distinct event streams must hash differently");
    }

    fn sample_event_stream() -> Vec<Event> {
        // Six events covering every payload variant so the encoder
        // exercise spans the whole `EventPayload` enum surface.
        let snap = SnapSlotId::new(3).expect("valid snap slot");
        vec![
            Event {
                seq: EventSeq(0),
                payload: EventPayload::ControlChanged {
                    id: ControlId(7),
                    new_value: ControlValue(0.75),
                },
            },
            Event {
                seq: EventSeq(1),
                payload: EventPayload::PresetRecalled { id: PresetId(2) },
            },
            Event {
                seq: EventSeq(2),
                payload: EventPayload::PlaylistAdvanced { step: 4 },
            },
            Event {
                seq: EventSeq(3),
                payload: EventPayload::SnapSaved { slot: snap },
            },
            Event {
                seq: EventSeq(4),
                payload: EventPayload::MappingBound {
                    id: MappingId(9),
                    source: MappingSource::Midi,
                },
            },
            Event {
                seq: EventSeq(5),
                payload: EventPayload::SnapCleared { slot: snap },
            },
        ]
    }
}
