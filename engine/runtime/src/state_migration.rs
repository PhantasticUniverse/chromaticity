// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 EV-STATE-MIGRATION versioned snapshot envelope.
//!
//! Provides forward-migration of canonical [`State`] snapshots
//! across schema versions. The Phase-3 narrowing covers V1→V2
//! exclusively; rollback (V2→V1) and broader transition matrices
//! are Phase-4+ scope per `docs/26-evidence-artifact-registry.md`
//! row 33.
//!
//! # Dual-encoder design
//!
//! Sibling [`crate::state_replay`] owns the canonical-binary
//! encoding consumed by EV-STATE-REPLAY (row 32) and EV-PRESET-
//! PLAYLIST (row 34) goldens. To keep those goldens stable while
//! still admitting a synthetic V1→V2 mutation for the migration
//! corpus, this module wraps the EV-STATE-REPLAY encoder output in
//! a versioned ENVELOPE rather than extending the per-sub-store
//! canonical-binary format directly:
//!
//! ```text
//! envelope_v1 = be_u32(0x0000_0001) || encode_state_canonical(state)
//!
//! envelope_v2 = be_u32(0x0000_0002)
//!            || encode_state_canonical(state)
//!            || be_u32(extension_count)
//!            || extensions[ V2SchemaMetadata::ext_id (be_u32)
//!                        || be_u32(schema_version) ]
//!     extension_count is fixed to 1 in V2 (V2SchemaMetadata only).
//! ```
//!
//! Consequences:
//! - Row 32 / row 34 goldens consume `encode_state_canonical(state)`
//!   directly via [`crate::state_replay::canonical_state_hash`] and
//!   do NOT see the version-tag prefix or the extension trailer; no
//!   regeneration needed when this module lands.
//! - The synthetic V1→V2 mutation is the addition of the
//!   `V2SchemaMetadata` extension. V1 envelopes omit the extension
//!   section entirely; V1 decode synthesizes a default
//!   `V2SchemaMetadata` (schema_version = 2) so callers always see a
//!   V2-shape `State` post-decode, satisfying the forward-migration
//!   contract.
//! - The hashing path (`canonical_state_hash`) ignores
//!   `snapshot_metadata`, so an in-memory V1→V2 migration that only
//!   sets `snapshot_metadata` does not change the canonical hash —
//!   matching the row-33 invariant that the migration's observable
//!   sub-store contents are preserved.
//!
//! # Supported transitions
//!
//! V1 = `0x0000_0001`. V2 = `0x0000_0002`. The decoder accepts V1
//! and V2 envelopes; [`migrate_v1_to_v2`] takes a V1 buffer and
//! returns the V2 form.
//!
//! New schema versions land here with a new variant on the version
//! tag space and a corresponding `migrate_vN_to_vN_plus_1` helper;
//! the [`DecodeError::UnknownVersion`] arm enforces the explicit
//! widening at runtime.

use crate::command_bus::{ControlId, ControlValue, MappingId, MappingSource, PresetId, SnapSlotId};
use crate::state::State;
use crate::state_replay::encode_state_canonical;
use std::fmt;

/// V1 snapshot version tag.
const VERSION_V1: u32 = 0x0000_0001;
/// V2 snapshot version tag (Phase-3 EV-STATE-MIGRATION baseline).
const VERSION_V2: u32 = 0x0000_0002;

/// Stable extension id for [`V2SchemaMetadata`]. Distinct values
/// land here as future extensions are added; the envelope decoder
/// uses the id to dispatch to per-extension parsers.
const EXT_ID_V2_SCHEMA_METADATA: u32 = 0x0000_0001;

/// Mapping-source tag bytes mirror [`encode_state_canonical`]'s
/// shape (Osc=0x01, Midi=0x02 per state_replay.rs lines 197-208).
const MAPPING_TAG_OSC: u8 = 0x01;
const MAPPING_TAG_MIDI: u8 = 0x02;

/// V2 schema metadata carried in the snapshot envelope.
///
/// The `schema_version` field is the synthetic V1→V2 mutation
/// driving the EV-STATE-MIGRATION corpus. V1 envelopes omit the
/// extension; decoded V1 buffers synthesize a default with
/// `schema_version = 2`, matching the forward-migration contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct V2SchemaMetadata {
    /// Schema version exposed to host integrations.
    pub schema_version: u32,
}

impl V2SchemaMetadata {
    /// Default V2 schema version constant.
    pub const SCHEMA_VERSION_V2: u32 = 2;

    /// V2 default carried by freshly migrated snapshots.
    #[must_use]
    pub const fn v2_default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION_V2,
        }
    }
}

/// Errors surfaced by [`decode_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeError {
    /// Buffer ended before the decoder finished parsing a required
    /// field. Includes envelope, sub-store, and extension parsing.
    Truncated,
    /// Version tag did not match a known schema version.
    UnknownVersion(u32),
    /// Mapping-source tag byte was neither `0x01` (Osc) nor `0x02`
    /// (Midi).
    InvalidMappingTag(u8),
    /// Snap-slot index exceeded `SnapSlotId::MAX` (15).
    SnapSlotOutOfRange(u8),
    /// Buffer contained additional bytes after the parser finished
    /// the declared envelope. Indicates encoder/decoder drift.
    TrailingBytes(usize),
    /// Extension id was not recognised by the V2 envelope parser.
    UnknownExtensionId(u32),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("snapshot envelope truncated"),
            Self::UnknownVersion(v) => write!(f, "unknown snapshot version 0x{v:08x}"),
            Self::InvalidMappingTag(t) => {
                write!(f, "invalid mapping-source tag 0x{t:02x}")
            }
            Self::SnapSlotOutOfRange(s) => {
                write!(f, "snap slot {s} exceeds SnapSlotId::MAX (15)")
            }
            Self::TrailingBytes(n) => {
                write!(f, "{n} trailing byte(s) after envelope")
            }
            Self::UnknownExtensionId(id) => {
                write!(f, "unknown extension id 0x{id:08x}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Errors surfaced by [`migrate_v1_to_v2`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MigrateError {
    /// The source buffer failed to decode as a V1 envelope.
    DecodeFailed(DecodeError),
    /// Migration was requested between schema versions for which
    /// no transformation is registered. Phase-3 supports V1→V2
    /// only; broader matrices land in Phase-4+.
    UnsupportedTransition { from: u32, to: u32 },
}

impl fmt::Display for MigrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecodeFailed(e) => write!(f, "snapshot decode failed: {e}"),
            Self::UnsupportedTransition { from, to } => write!(
                f,
                "unsupported snapshot migration: 0x{from:08x} -> 0x{to:08x}"
            ),
        }
    }
}

impl std::error::Error for MigrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DecodeFailed(e) => Some(e),
            Self::UnsupportedTransition { .. } => None,
        }
    }
}

/// Encode `state` as a V2 envelope (current schema version).
///
/// Layout: `be_u32(0x0000_0002) || encode_state_canonical(state)
/// || be_u32(1) || be_u32(EXT_ID_V2_SCHEMA_METADATA) ||
/// be_u32(schema_version)`. The schema_version emitted is taken
/// from `state.snapshot_metadata` if set, else defaults to V2.
#[must_use]
pub fn encode_snapshot(state: &State) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&VERSION_V2.to_be_bytes());
    encode_state_canonical(state, &mut buf);
    // Single extension: V2SchemaMetadata.
    let extension_count: u32 = 1;
    buf.extend_from_slice(&extension_count.to_be_bytes());
    buf.extend_from_slice(&EXT_ID_V2_SCHEMA_METADATA.to_be_bytes());
    let schema_version = state
        .snapshot_metadata
        .map_or(V2SchemaMetadata::SCHEMA_VERSION_V2, |m| m.schema_version);
    buf.extend_from_slice(&schema_version.to_be_bytes());
    buf
}

/// Encode `state` as a V1 envelope. V1 omits the extension section
/// entirely. Only the migration corpus + `migrate_v1_to_v2`
/// fixtures need the V1 form; production hosts always emit V2.
#[must_use]
pub fn encode_snapshot_v1(state: &State) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&VERSION_V1.to_be_bytes());
    encode_state_canonical(state, &mut buf);
    buf
}

/// Decode `buf` into a fresh [`State`]. Auto-detects V1 vs V2 from
/// the leading version tag. V1 buffers populate
/// `snapshot_metadata` with the V2 default; V2 buffers parse the
/// extension trailer.
pub fn decode_snapshot(buf: &[u8]) -> Result<State, DecodeError> {
    let mut cursor = Cursor::new(buf);
    let version = cursor.read_be_u32()?;
    // Validate the version tag BEFORE attempting to decode the
    // canonical-binary body so an unknown-version buffer surfaces
    // `UnknownVersion` cleanly even if the trailing bytes are not
    // wide enough to hold a full canonical body.
    if version != VERSION_V1 && version != VERSION_V2 {
        return Err(DecodeError::UnknownVersion(version));
    }
    let mut state = State::default();
    decode_state_canonical(&mut cursor, &mut state)?;
    match version {
        VERSION_V1 => {
            // V1 carries no extensions; synthesize the default to
            // satisfy the forward-migration contract.
            state.snapshot_metadata = Some(V2SchemaMetadata::v2_default());
        }
        VERSION_V2 => {
            decode_v2_extensions(&mut cursor, &mut state)?;
        }
        // The early-return above is exhaustive over unknown
        // versions; keeping the arm here is dead code that the
        // compiler optimises away. Use `unreachable!()` only via
        // the unused-pattern-binding form so clippy::expect_used
        // is not relevant.
        _ => return Err(DecodeError::UnknownVersion(version)),
    }
    if cursor.remaining() != 0 {
        return Err(DecodeError::TrailingBytes(cursor.remaining()));
    }
    Ok(state)
}

/// Migrate a V1 snapshot buffer into a V2 snapshot buffer.
///
/// Implementation: decode V1 (which sets `snapshot_metadata` to the
/// V2 default per the forward-migration contract), then re-encode
/// as V2. The canonical-binary sub-store contents round-trip
/// unchanged; only the envelope shape differs.
pub fn migrate_v1_to_v2(buf: &[u8]) -> Result<Vec<u8>, MigrateError> {
    // Validate the leading version tag explicitly so we surface
    // `UnsupportedTransition` (rather than `DecodeFailed`) when the
    // caller hands us a non-V1 buffer.
    if buf.len() < 4 {
        return Err(MigrateError::DecodeFailed(DecodeError::Truncated));
    }
    let mut version_bytes = [0u8; 4];
    version_bytes.copy_from_slice(&buf[..4]);
    let from = u32::from_be_bytes(version_bytes);
    if from != VERSION_V1 {
        return Err(MigrateError::UnsupportedTransition {
            from,
            to: VERSION_V2,
        });
    }
    let state = decode_snapshot(buf).map_err(MigrateError::DecodeFailed)?;
    Ok(encode_snapshot(&state))
}

// --- Decoder internals ------------------------------------------

/// Cursor over a borrowed byte slice with bounds-checked BE
/// readers. All readers return [`DecodeError::Truncated`] when the
/// requested bytes overrun the slice.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        let byte = *self.buf.get(self.pos).ok_or(DecodeError::Truncated)?;
        self.pos += 1;
        Ok(byte)
    }

    fn read_be_u32(&mut self) -> Result<u32, DecodeError> {
        let end = self.pos.checked_add(4).ok_or(DecodeError::Truncated)?;
        let bytes = self.buf.get(self.pos..end).ok_or(DecodeError::Truncated)?;
        let mut arr = [0u8; 4];
        arr.copy_from_slice(bytes);
        self.pos = end;
        Ok(u32::from_be_bytes(arr))
    }

    fn read_be_u64(&mut self) -> Result<u64, DecodeError> {
        let end = self.pos.checked_add(8).ok_or(DecodeError::Truncated)?;
        let bytes = self.buf.get(self.pos..end).ok_or(DecodeError::Truncated)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        self.pos = end;
        Ok(u64::from_be_bytes(arr))
    }

    fn read_be_i64(&mut self) -> Result<i64, DecodeError> {
        // i64 is the BE-bit-pattern reinterpretation of u64.
        Ok(self.read_be_u64()? as i64)
    }
}

/// Mirror of [`crate::state_replay::encode_state_canonical`] in
/// reverse. Reads the EV-STATE-REPLAY canonical-binary segment
/// from `cursor` into the sub-stores of `state`.
fn decode_state_canonical(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    state.frame = cursor.read_be_u64()?;
    decode_control_store(cursor, state)?;
    decode_preset_store(cursor, state)?;
    decode_playlist_store(cursor, state)?;
    decode_snap_store(cursor, state)?;
    decode_mapping_store(cursor, state)?;
    Ok(())
}

fn decode_control_store(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    let count = cursor.read_be_u32()?;
    for _ in 0..count {
        let id = ControlId(cursor.read_be_u32()?);
        let bits = cursor.read_be_u32()?;
        state
            .control_store
            .values
            .insert(id, ControlValue(f32::from_bits(bits)));
    }
    Ok(())
}

fn decode_preset_store(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    let flag = cursor.read_u8()?;
    state.preset_store.active = match flag {
        0x00 => None,
        0x01 => Some(PresetId(cursor.read_be_u32()?)),
        // Unknown flag byte falls back to `None` is unsafe — treat
        // as truncation/drift to surface a loud error.
        _ => return Err(DecodeError::Truncated),
    };
    Ok(())
}

fn decode_playlist_store(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    state.playlist_store.position = cursor.read_be_i64()?;
    Ok(())
}

fn decode_snap_store(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    let active_flag = cursor.read_u8()?;
    state.snap_store.active = match active_flag {
        0x00 => None,
        0x01 => {
            let raw = cursor.read_u8()?;
            Some(SnapSlotId::new(raw).ok_or(DecodeError::SnapSlotOutOfRange(raw))?)
        }
        _ => return Err(DecodeError::Truncated),
    };
    let saved_count = cursor.read_be_u32()?;
    for _ in 0..saved_count {
        let raw = cursor.read_u8()?;
        let slot = SnapSlotId::new(raw).ok_or(DecodeError::SnapSlotOutOfRange(raw))?;
        state.snap_store.saved.insert(slot);
    }
    Ok(())
}

fn decode_mapping_store(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    let count = cursor.read_be_u32()?;
    for _ in 0..count {
        let id = MappingId(cursor.read_be_u32()?);
        let tag = cursor.read_u8()?;
        let source = match tag {
            MAPPING_TAG_OSC => MappingSource::Osc,
            MAPPING_TAG_MIDI => MappingSource::Midi,
            other => return Err(DecodeError::InvalidMappingTag(other)),
        };
        state.mapping_store.bindings.insert(id, source);
    }
    Ok(())
}

fn decode_v2_extensions(cursor: &mut Cursor<'_>, state: &mut State) -> Result<(), DecodeError> {
    let extension_count = cursor.read_be_u32()?;
    let mut metadata: Option<V2SchemaMetadata> = None;
    for _ in 0..extension_count {
        let ext_id = cursor.read_be_u32()?;
        match ext_id {
            EXT_ID_V2_SCHEMA_METADATA => {
                let schema_version = cursor.read_be_u32()?;
                metadata = Some(V2SchemaMetadata { schema_version });
            }
            other => return Err(DecodeError::UnknownExtensionId(other)),
        }
    }
    state.snapshot_metadata = Some(metadata.unwrap_or_else(V2SchemaMetadata::v2_default));
    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod migration_pins {
    //! Evidence: EV-STATE-MIGRATION (T1) — Phase-3 envelope drift
    //! pins for the EV-STATE-MIGRATION canonical surface.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pins on the
    //! envelope shape (V2 prefix is a BE-u32 `0x02`), the
    //! decode-then-canonical-hash round-trip invariant for the
    //! default state, the V1→V2 migration default-metadata
    //! invariant, and the unknown-version rejection. The T2 corpus
    //! in `engine/runtime/tests/state_migration_corpus/main.rs`
    //! depends on these pins for fixture sanity.
    //!
    //! Anchors:
    //! - `docs/26-evidence-artifact-registry.md` row 33 — `EV-STATE-MIGRATION`.
    //! - `docs/adr/ADR-004-state-model.md` — canonical state model.
    //! - `docs/43-development-standards.md` §15.5 — T-tier definitions.

    use super::{
        DecodeError, V2SchemaMetadata, VERSION_V2, decode_snapshot, encode_snapshot,
        encode_snapshot_v1, migrate_v1_to_v2,
    };
    use crate::state::State;
    use crate::state_replay::canonical_state_hash;

    #[test]
    fn version_tag_v2_prefix_is_be_u32_two() {
        // Drift pin: encoded V2 envelope MUST begin with the
        // big-endian u32 `0x0000_0002`. A backend or compiler-
        // version drift that flipped endianness (or shifted the
        // version constant) trips here loudly before the corpus
        // would catch it as a noisy hash divergence.
        let buf = encode_snapshot(&State::default());
        assert!(
            buf.len() >= 4,
            "envelope must hold at least the version tag"
        );
        let mut prefix = [0u8; 4];
        prefix.copy_from_slice(&buf[..4]);
        assert_eq!(
            u32::from_be_bytes(prefix),
            VERSION_V2,
            "V2 envelope prefix must be BE-u32(2)",
        );
    }

    #[test]
    fn decode_default_state_round_trips_via_canonical_hash() {
        // Drift pin: encode + decode of the default state MUST
        // round-trip; the canonical-hash invariant is the strongest
        // observable: every sub-store survives the envelope path.
        // The hashing path ignores `snapshot_metadata`, so the
        // default-state hash matches even though the decoded state
        // carries Some(V2SchemaMetadata::v2_default()).
        let original = State::default();
        let original_hash = canonical_state_hash(&original);
        let buf = encode_snapshot(&original);
        let decoded = decode_snapshot(&buf).expect("default state must decode");
        let decoded_hash = canonical_state_hash(&decoded);
        assert_eq!(
            original_hash, decoded_hash,
            "envelope round-trip must preserve canonical hash"
        );
    }

    #[test]
    fn migrate_empty_v1_buffer_yields_v2_default_metadata() {
        // Drift pin: V1 → migrate → decode V2 MUST surface
        // `V2SchemaMetadata { schema_version = 2 }`. The Phase-3
        // forward-migration contract: any V1 buffer post-migration
        // looks V2 to host integrations.
        let v1 = encode_snapshot_v1(&State::default());
        let v2 = migrate_v1_to_v2(&v1).expect("v1 default must migrate");
        let decoded = decode_snapshot(&v2).expect("migrated v2 must decode");
        assert_eq!(
            decoded.snapshot_metadata,
            Some(V2SchemaMetadata::v2_default()),
            "V1 → V2 must seed default metadata",
        );
    }

    #[test]
    fn decode_rejects_unknown_version_in_buffer_prefix() {
        // Drift pin: a synthetic buffer whose version tag is not in
        // {V1, V2} MUST surface `DecodeError::UnknownVersion(_)`.
        // Pins that the version-tag arm is exhaustive and routes
        // unknown versions through the explicit error path rather
        // than into a sub-store decode.
        let mut buf = vec![0u8; 4];
        buf[..4].copy_from_slice(&0xFFFF_FFFF_u32.to_be_bytes());
        let err = decode_snapshot(&buf).expect_err("unknown version must reject");
        assert!(
            matches!(err, DecodeError::UnknownVersion(0xFFFF_FFFF)),
            "expected UnknownVersion(0xFFFFFFFF), got {err:?}",
        );
    }
}
