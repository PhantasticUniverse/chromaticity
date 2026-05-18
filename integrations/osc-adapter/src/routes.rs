// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! OSC route-schema namespace types.
//!
//! Slice-2 declares the structural shape of the route namespace
//! without any runtime evaluation. A scene-author-facing `/control/…`
//! address parses into a [`RouteAddr`] with a [`RoutePrefix`] tag and
//! a `segments` vector; the dispatch that turns a parsed address
//! into a runtime command is owned by [`crate::bindings`].
//!
//! The parser accepts the canonical seven-family vocabulary used by
//! the operator-facing registry: Control, Scene, Preset, Snap,
//! Playlist, Audio, and Reserved. Singular roots are preferred for
//! new bindings; plural and historical compatibility roots remain
//! accepted so existing route fixtures continue to parse. Reserved
//! `/data/...` routes are recognised as a distinct diagnostic
//! (`RouteParseError::PrefixReserved`) rather than folded into an
//! unknown-prefix rejection.
//!
//! Visibility was widened from `pub(crate)` to `pub` in Phase-4a
//! Wave 1a so the `EV-INTEGRATION-MATRIX` (T2) conformance corpus
//! in `integrations/osc-adapter/tests/integration_matrix_corpus/`
//! can exercise the route-namespace taxonomy from outside the
//! crate. The slice-2 module prose anticipated this trigger
//! ("Slice-3 may widen visibility when a consumer outside this
//! crate needs to reference the taxonomy"); Phase-4a's matrix
//! gate is the legitimate consumer. The shape (variants,
//! `RouteAddr` field set, `parse` signature) is unchanged — only
//! the `pub(crate)` qualifier was relaxed.

/// Top-level OSC namespace prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoutePrefix {
    /// `/control/…` — control-value routes.
    Control,
    /// `/scene/…` — scene-selection routes.
    Scene,
    /// `/preset/…` — preset apply / save-as routes.
    Preset,
    /// `/snap/…` — Snap recall / save / clear routes.
    Snap,
    /// `/playlist/…` — playlist transport routes.
    Playlist,
    /// `/audio/…` — audio-pipeline routes.
    Audio,
    /// `/data/…` — reserved future data routes.
    Reserved,
}

/// Parsed OSC address pattern.
///
/// Slice-2 stores the tagged prefix plus the tail segments without
/// any interpretation. Slice-3 route-handler dispatch consumes this
/// shape to look up the matching handler and emit the corresponding
/// runtime command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteAddr {
    /// Top-level prefix tag — see [`RoutePrefix`].
    pub prefix: RoutePrefix,
    /// Tail address segments after the prefix. Empty segments
    /// (caused by trailing or doubled slashes) are stripped during
    /// [`parse`] so callers see a canonical segment vector.
    pub segments: Vec<String>,
}

/// Structured route-parse diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RouteParseError {
    /// Address was empty, lacked a leading slash, or lacked a prefix.
    MalformedAddress,
    /// Top-level prefix is not part of the canonical OSC route
    /// vocabulary.
    UnknownPrefix(String),
    /// `/data/...` is reserved per OSC-CONF-16 and must not dispatch.
    PrefixReserved(String),
}

impl core::fmt::Display for RouteParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MalformedAddress => write!(f, "osc route address is malformed"),
            Self::UnknownPrefix(prefix) => write!(f, "unknown osc route prefix: {prefix}"),
            Self::PrefixReserved(prefix) => write!(f, "osc route prefix is reserved: {prefix}"),
        }
    }
}

impl std::error::Error for RouteParseError {}

/// Parse an OSC address pattern into a [`RouteAddr`].
///
/// Returns `None` for malformed input — empty string, missing
/// leading `/`, an unrecognised top-level prefix, or a reserved
/// prefix. Call [`parse_diagnostic`] when the caller needs the exact
/// rejection reason.
pub fn parse(s: &str) -> Option<RouteAddr> {
    parse_diagnostic(s).ok()
}

/// Parse an OSC address pattern into a [`RouteAddr`] with structured
/// diagnostics for malformed, unknown-prefix, and reserved-prefix
/// cases.
pub fn parse_diagnostic(s: &str) -> Result<RouteAddr, RouteParseError> {
    let rest = s
        .strip_prefix('/')
        .ok_or(RouteParseError::MalformedAddress)?;
    let mut parts = rest.split('/');
    let head = parts.next().ok_or(RouteParseError::MalformedAddress)?;
    if head.is_empty() {
        return Err(RouteParseError::MalformedAddress);
    }
    let prefix = parse_prefix(head)?;
    let segments: Vec<String> = parts
        .filter(|seg| !seg.is_empty())
        .map(String::from)
        .collect();
    Ok(RouteAddr { prefix, segments })
}

fn parse_prefix(head: &str) -> Result<RoutePrefix, RouteParseError> {
    match head {
        "control" | "controls" => Ok(RoutePrefix::Control),
        "scene" | "scenes" => Ok(RoutePrefix::Scene),
        "preset" | "presets" => Ok(RoutePrefix::Preset),
        "snap" | "snaps" | "favslot" | "favslots" => Ok(RoutePrefix::Snap),
        "playlist" | "playlists" => Ok(RoutePrefix::Playlist),
        "audio" => Ok(RoutePrefix::Audio),
        "data" => Err(RouteParseError::PrefixReserved(head.to_string())),
        _ => Err(RouteParseError::UnknownPrefix(head.to_string())),
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod drift_pins {
    //! Evidence: EV-NET-HARDENING (T1) — route-namespace taxonomy
    //! parse invariants.
    //!
    //! T1 per docs/43 §15.5: each test pins an invariant the
    //! slice-2 parser MUST honour so slice-3 dispatch can rely on
    //! a stable pre-parsed shape. Slice-2's strict-accept posture
    //! rejects malformed addresses outright; compat expansion (if
    //! any) is a slice-3 decision.

    use super::{RouteParseError, RoutePrefix, parse, parse_diagnostic};

    #[test]
    fn parse_rejects_empty_and_unprefixed() {
        assert!(parse("").is_none());
        assert!(parse("controls/bank/one").is_none()); // missing /
        assert!(parse("/").is_none()); // no prefix
    }

    #[test]
    fn parse_rejects_unknown_prefix() {
        assert!(parse("/unknown/foo").is_none());
        assert!(parse("/foo").is_none());
        assert_eq!(
            parse_diagnostic("/unknown/foo").unwrap_err(),
            RouteParseError::UnknownPrefix("unknown".to_string()),
        );
    }

    #[test]
    fn parse_accepts_singular_canonical_prefixes_with_segments() {
        let addr = parse("/control/bank/one").expect("valid");
        assert_eq!(addr.prefix, RoutePrefix::Control);
        assert_eq!(addr.segments, vec!["bank".to_string(), "one".to_string()]);

        let scene = parse("/scene/demo").expect("valid");
        assert_eq!(scene.prefix, RoutePrefix::Scene);
        assert_eq!(scene.segments, vec!["demo".to_string()]);

        let preset = parse("/preset/7").expect("valid");
        assert_eq!(preset.prefix, RoutePrefix::Preset);
        assert_eq!(preset.segments, vec!["7".to_string()]);

        let snap = parse("/snap/4").expect("valid");
        assert_eq!(snap.prefix, RoutePrefix::Snap);
        assert_eq!(snap.segments, vec!["4".to_string()]);

        let playlist = parse("/playlist/next").expect("valid");
        assert_eq!(playlist.prefix, RoutePrefix::Playlist);
        assert_eq!(playlist.segments, vec!["next".to_string()]);

        let audio = parse("/audio/level").expect("valid");
        assert_eq!(audio.prefix, RoutePrefix::Audio);
        assert_eq!(audio.segments, vec!["level".to_string()]);
    }

    #[test]
    fn parse_keeps_plural_and_compat_prefix_aliases() {
        assert_eq!(
            parse("/controls/bank/one").expect("valid").prefix,
            RoutePrefix::Control,
        );
        assert_eq!(
            parse("/scenes/demo").expect("valid").prefix,
            RoutePrefix::Scene,
        );
        assert_eq!(
            parse("/presets/7").expect("valid").prefix,
            RoutePrefix::Preset,
        );
        assert_eq!(
            parse("/favslots/4").expect("valid").prefix,
            RoutePrefix::Snap,
        );
        assert_eq!(
            parse("/playlists/next").expect("valid").prefix,
            RoutePrefix::Playlist,
        );
    }

    #[test]
    fn parse_reports_reserved_data_prefix() {
        assert!(parse("/data/subscribe/foo").is_none());
        assert_eq!(
            parse_diagnostic("/data/subscribe/foo").unwrap_err(),
            RouteParseError::PrefixReserved("data".to_string()),
        );
    }

    #[test]
    fn parse_strips_empty_segments() {
        // OSC wire conventions tolerate trailing slashes on some
        // clients; slice-2 collapses them rather than rejecting so
        // the slice-3 dispatch surface sees a canonical segment
        // vector.
        let addr = parse("/control/").expect("valid");
        assert_eq!(addr.prefix, RoutePrefix::Control);
        assert!(addr.segments.is_empty());
    }
}
