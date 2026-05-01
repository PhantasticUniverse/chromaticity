// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! OSC route-schema namespace types.
//!
//! Slice-2 declares the structural shape of the route namespace
//! without any runtime evaluation. A scene-author-facing `/controls/…`
//! address parses into a [`RouteAddr`] with a [`RoutePrefix`] tag and
//! a `segments` vector; the dispatch that turns a parsed address
//! into a runtime command lands with slice-3 once the route-handler
//! registry wires against `chromaticity_runtime::Command`.
//!
//! The four prefixes shipped in slice-2 are a deliberate subset of
//! the `docs/08-io-integrations.md` OSC namespace; slice-3 may extend
//! the taxonomy once the route-handler design is finalised.
//! `#[non_exhaustive]` is applied so that extension is an additive
//! change rather than a breaking variant add.
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
///
/// Slice-2 subset; slice-3 extends. See `docs/08-io-integrations.md`
/// §OSC for the full prefix enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RoutePrefix {
    /// `/controls/…` — control-value routes.
    Controls,
    /// `/scenes/…` — scene-selection routes.
    Scenes,
    /// `/audio/…` — audio-pipeline routes.
    Audio,
    /// `/render/…` — render-configuration routes.
    Render,
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

/// Parse an OSC address pattern into a [`RouteAddr`].
///
/// Returns `None` for malformed input — empty string, missing
/// leading `/`, or an unrecognised top-level prefix. Slice-2's
/// parser is deliberately strict; slice-3 may widen the accept set
/// as the route-handler design settles.
///
/// TODO(slice-3, R-05): once `poll()` parses the inbound packet's
/// OSC address, surface a `RouteMismatch` outcome here for unknown
/// prefixes. Deferred from the slice-2 R-05 hardening pass because
/// it requires (a) a new `OscPacketOutcome` variant in
/// `chromaticity-observability::catalog` (cross-subtree) and (b)
/// route-extraction from the parsed [`OscPacket`], which is
/// slice-3 dispatch scope per `lib.rs` module prose.
pub fn parse(s: &str) -> Option<RouteAddr> {
    let rest = s.strip_prefix('/')?;
    let mut parts = rest.split('/');
    let head = parts.next()?;
    if head.is_empty() {
        return None;
    }
    let prefix = match head {
        "controls" => RoutePrefix::Controls,
        "scenes" => RoutePrefix::Scenes,
        "audio" => RoutePrefix::Audio,
        "render" => RoutePrefix::Render,
        _ => return None,
    };
    let segments: Vec<String> = parts
        .filter(|seg| !seg.is_empty())
        .map(String::from)
        .collect();
    Some(RouteAddr { prefix, segments })
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

    use super::{RoutePrefix, parse};

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
    }

    #[test]
    fn parse_accepts_canonical_prefixes_with_segments() {
        let addr = parse("/controls/bank/one").expect("valid");
        assert_eq!(addr.prefix, RoutePrefix::Controls);
        assert_eq!(addr.segments, vec!["bank".to_string(), "one".to_string()]);

        let audio = parse("/audio/level").expect("valid");
        assert_eq!(audio.prefix, RoutePrefix::Audio);
        assert_eq!(audio.segments, vec!["level".to_string()]);
    }

    #[test]
    fn parse_strips_empty_segments() {
        // OSC wire conventions tolerate trailing slashes on some
        // clients; slice-2 collapses them rather than rejecting so
        // the slice-3 dispatch surface sees a canonical segment
        // vector.
        let addr = parse("/render/").expect("valid");
        assert_eq!(addr.prefix, RoutePrefix::Render);
        assert!(addr.segments.is_empty());
    }
}
