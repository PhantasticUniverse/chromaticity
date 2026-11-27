// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Operator-facing OSC route registry — Wave 2.1 Lane D.3.
//!
//! Slice-2's [`crate::routes`] module pins the structural parse of a
//! `/controls/…` style address into a [`crate::routes::RouteAddr`];
//! that surface is the WIRE-side taxonomy (what the OSC packet
//! decoder honours when an inbound packet lands).
//!
//! Wave 2.1 Lane D.3 introduces an ORTHOGONAL operator-facing
//! registry surface so the OSC view in `src/lib/views/Osc.svelte`
//! can:
//!
//! 1. Enumerate the canonical routes the runtime supports (the seven
//!    taxonomy entries pulled from `docs/08-io-integrations.md` §OSC,
//!    mirrored in the frontend fixture
//!    `src/lib/data/osc-routes.ts`).
//! 2. Bind a structured operator-supplied `target` string per route
//!    using the [`crate::bindings::parse_bind_target`] grammar
//!    (`control:<bank>:<name>`, `scene:<scene_id>`, `snap:<slot>`,
//!    `preset:<id>`, `playlist:next|prev`). The string stays verbatim
//!    on the IPC wire, but malformed targets fail at bind time.
//! 3. Persist the bind table between runs (via the shell-side
//!    `AppState.settings` mutex; the engine adapter itself stays
//!    storage-free per ADR-005 §3).
//!
//! ## Why this is structurally separate from `crate::routes`
//!
//! - `RouteAddr::parse(s)` parses ARBITRARY inbound addresses against
//!   the canonical `RoutePrefix` families. Adding a Family / Direction /
//!   Kind axis there would conflate "what does this wire-shape parse
//!   into" with "what does the operator see in the route binding UI".
//! - The seven canonical routes here are a curated UX subset (the
//!   ones the bundle's OSC view fixture surfaces). Adding new
//!   variants is a UX-axis decision, not a wire-protocol decision.
//! - The frontend's [`OscRouteFixture`](src/lib/data/osc-routes.ts)
//!   carries `compatAlias` + `direction` + `family` per route; the
//!   bridge mirror in this slice carries the same shape so the
//!   frontend store can replace the fixture import with a live store
//!   read without churning the row template.
//!
//! Scope: the registry is a thread-safe in-memory map. The engine
//! adapter is intentionally storage-free; the shell-side
//! `Mutex<OscRouteRegistry>` on `AppState` is the only owner. The
//! bind / unbind operations are O(n) over the canonical-route slice
//! (n=7); no indexing optimisation is needed at this scale.

use std::collections::BTreeMap;

use crate::bindings::{BindError, parse_bind_target};

/// Direction of an OSC route relative to the chromaticity adapter.
///
/// Mirrors the frontend `OscDirection` type in
/// `src/lib/data/osc-routes.ts`. Inbound packets arrive on `In` /
/// `Both` routes; outbound packets emit on `Out` / `Both` routes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OscDirection {
    /// Adapter receives packets matching this route from external
    /// peers.
    In,
    /// Adapter emits packets on this route to external peers.
    Out,
    /// Bidirectional — used by `/controls/…` and `/scenes/…` where the
    /// runtime echoes a value back after committing an inbound mutation.
    Both,
}

/// Operator-facing classification of an OSC route's domain.
///
/// Mirrors the frontend `OscFamily` type in
/// `src/lib/data/osc-routes.ts`. The seven variants match the
/// curated UX subset shipped in the v0.2-beta bundle's OSC view JSX
/// fixture; new routes added to the canonical table here MUST surface
/// a matching variant on this enum so the frontend type-import stays
/// in lockstep with the live registry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OscRouteFamily {
    /// `/controls/…` — control-value routes.
    Control,
    /// `/scenes/…` — scene-selection routes.
    Scene,
    /// `/presets/…` — preset apply / save-as routes.
    Preset,
    /// `/favslots/…` — Snap recall / save / clear routes.
    Snap,
    /// `/playlists/…` — playlist advance / position routes.
    Playlist,
    /// `/audio/…` — audio-pipeline read-side routes.
    Audio,
    /// `/data/*` reserved prefixes per OSC-CONF-16 — inbound rejected
    /// with the `prefix-reserved` diagnostic.
    Reserved,
}

/// Failure surfaces for [`OscRouteRegistry::bind`] and
/// [`OscRouteRegistry::unbind`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OscRouteError {
    /// The supplied canonical route is not in the registry's
    /// initial seed.
    UnknownRoute(String),
    /// The route exists, but the supplied target string does not
    /// match the bind-target grammar.
    InvalidBindTarget(BindError),
}

impl core::fmt::Display for OscRouteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownRoute(route) => {
                write!(f, "osc route not registered: {route}")
            }
            Self::InvalidBindTarget(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for OscRouteError {}

/// A single operator-facing OSC route entry.
///
/// `bind_target` is `None` when the operator hasn't bound the route
/// to anything (the default state for every entry in the seed table).
/// Setting it to `Some(target)` records the exact IPC string after the
/// grammar has been validated by [`parse_bind_target`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscRouteEntry {
    /// Canonical address (plural-root v1) — `/controls/{bank}/{name}`,
    /// `/scenes/{sceneName}`, etc.
    pub canonical: String,
    /// Compat-syn-v1 single-root alias when defined; `None` for routes
    /// that exist only on the v1 canonical taxonomy.
    pub compat_alias: Option<String>,
    /// OSC type-tag summary — `f` / `i` / `s` / `[s]` / `—` per
    /// `docs/08-io-integrations.md` §OSC.
    pub type_tag: String,
    /// Human-readable value range / description.
    pub range: String,
    pub direction: OscDirection,
    pub family: OscRouteFamily,
    /// Operator-supplied validated target for this route. `None` when
    /// unbound. A non-empty string when bound.
    pub bind_target: Option<String>,
}

/// In-memory OSC route registry.
///
/// Owns the canonical-route seed table plus the operator's bind /
/// unbind mutations. The seed table is loaded on construction
/// ([`Self::with_default_seed`]) and is never mutated; the
/// `bind_target` field is the only mutable axis.
///
/// Storage is a `BTreeMap<String, OscRouteEntry>` keyed by the
/// canonical address so the snapshot iteration order is stable
/// regardless of insertion order — the frontend renders the routes
/// table in lexicographic canonical order, which matches the
/// taxonomy grouping in `docs/08` §OSC for typical canonical paths.
#[derive(Debug, Clone, Default)]
pub struct OscRouteRegistry {
    entries: BTreeMap<String, OscRouteEntry>,
}

impl OscRouteRegistry {
    /// Construct an empty registry. Used by tests that build a
    /// custom seed; the production caller is
    /// [`Self::with_default_seed`].
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Construct a registry pre-seeded with the canonical UX subset
    /// pulled from `docs/08-io-integrations.md` §OSC. The set matches
    /// the frontend's `OSC_ROUTES` fixture array verbatim (canonical /
    /// compat_alias / direction / family / type / range axes).
    #[must_use]
    pub fn with_default_seed() -> Self {
        let mut registry = Self::empty();
        for seed in OSC_DEFAULT_ROUTES {
            registry.entries.insert(
                seed.canonical.to_string(),
                OscRouteEntry {
                    canonical: seed.canonical.to_string(),
                    compat_alias: seed.compat_alias.map(str::to_string),
                    type_tag: seed.type_tag.to_string(),
                    range: seed.range.to_string(),
                    direction: seed.direction,
                    family: seed.family,
                    bind_target: None,
                },
            );
        }
        registry
    }

    /// Number of routes in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` iff the registry has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Snapshot the current registry state as a `Vec` of entries in
    /// canonical-address lexicographic order.
    ///
    /// The snapshot is owned (the registry isn't drained) so the
    /// shell-side handler can release the mutex before returning the
    /// payload across the IPC boundary.
    #[must_use]
    pub fn snapshot(&self) -> Vec<OscRouteEntry> {
        self.entries.values().cloned().collect()
    }

    /// Read a single entry by canonical address.
    #[must_use]
    pub fn get(&self, canonical: &str) -> Option<&OscRouteEntry> {
        self.entries.get(canonical)
    }

    /// Bind `target` to the route identified by `canonical`.
    ///
    /// Returns [`OscRouteError::UnknownRoute`] when the canonical
    /// address isn't in the seed table and
    /// [`OscRouteError::InvalidBindTarget`] when `target` does not
    /// match the OSC bind-target grammar.
    pub fn bind(&mut self, canonical: &str, target: &str) -> Result<(), OscRouteError> {
        match self.entries.get_mut(canonical) {
            Some(entry) => {
                parse_bind_target(target).map_err(OscRouteError::InvalidBindTarget)?;
                entry.bind_target = Some(target.to_string());
                Ok(())
            }
            None => Err(OscRouteError::UnknownRoute(canonical.to_string())),
        }
    }

    /// Clear any bind target on the route identified by `canonical`.
    ///
    /// Idempotent — unbinding an already-unbound route is not an
    /// error. Returns [`OscRouteError::UnknownRoute`] only when the
    /// canonical address itself isn't in the seed table.
    pub fn unbind(&mut self, canonical: &str) -> Result<(), OscRouteError> {
        match self.entries.get_mut(canonical) {
            Some(entry) => {
                entry.bind_target = None;
                Ok(())
            }
            None => Err(OscRouteError::UnknownRoute(canonical.to_string())),
        }
    }
}

/// Static seed-table entry used by [`OscRouteRegistry::with_default_seed`].
///
/// Layout matches [`OscRouteEntry`] minus `bind_target` (which always
/// starts as `None`). Stored as `&'static str` slices so the table
/// lives in `.rodata` with zero per-entry allocation; the registry's
/// `with_default_seed` constructor allocates owned `String`s once at
/// construction.
#[derive(Debug, Clone, Copy)]
struct OscRouteSeed {
    canonical: &'static str,
    compat_alias: Option<&'static str>,
    type_tag: &'static str,
    range: &'static str,
    direction: OscDirection,
    family: OscRouteFamily,
}

/// Canonical OSC route seed table.
///
/// Curated UX subset mirroring `src/lib/data/osc-routes.ts` (the
/// bundle JSX `OSC_ROUTES` constant). New canonical routes added to
/// `docs/08-io-integrations.md` §OSC MUST be appended here AND to the
/// frontend fixture in lockstep so the registry shape stays the
/// source of truth for the Routes view.
///
/// Length intentionally small — this is the operator-visible subset,
/// NOT the wire-protocol parse vocabulary (which lives in
/// `crate::routes` and parses every `/controls/…` address whether
/// the operator has bound it or not).
const OSC_DEFAULT_ROUTES: &[OscRouteSeed] = &[
    OscRouteSeed {
        canonical: "/audio/threshold {f}",
        compat_alias: None,
        type_tag: "f",
        range: "0.0 – 1.0",
        direction: OscDirection::Both,
        family: OscRouteFamily::Audio,
    },
    OscRouteSeed {
        canonical: "/audio/var/{uniform}",
        compat_alias: None,
        type_tag: "f",
        range: "per-uniform cadence",
        direction: OscDirection::Out,
        family: OscRouteFamily::Audio,
    },
    OscRouteSeed {
        canonical: "/controls/{bank}/{name}",
        compat_alias: Some("/controls/{addr}"),
        type_tag: "f",
        range: "0.0 – 1.0",
        direction: OscDirection::Both,
        family: OscRouteFamily::Control,
    },
    OscRouteSeed {
        canonical: "/data/*",
        compat_alias: None,
        type_tag: "—",
        range: "reserved post-MVP",
        direction: OscDirection::In,
        family: OscRouteFamily::Reserved,
    },
    OscRouteSeed {
        canonical: "/favslots/{n}",
        compat_alias: Some("/favslot/{n}"),
        type_tag: "—",
        range: "Snap recall (n 0–15)",
        direction: OscDirection::In,
        family: OscRouteFamily::Snap,
    },
    OscRouteSeed {
        canonical: "/playlists/next",
        compat_alias: Some("/playlist/next"),
        type_tag: "—",
        range: "trigger",
        direction: OscDirection::In,
        family: OscRouteFamily::Playlist,
    },
    OscRouteSeed {
        canonical: "/playlists/position {i}",
        compat_alias: None,
        type_tag: "i",
        range: "absolute zero-based index",
        direction: OscDirection::In,
        family: OscRouteFamily::Playlist,
    },
    OscRouteSeed {
        canonical: "/presets/{name}",
        compat_alias: Some("/preset/{name}"),
        type_tag: "—",
        range: "apply",
        direction: OscDirection::In,
        family: OscRouteFamily::Preset,
    },
    OscRouteSeed {
        canonical: "/scenes/{sceneName}",
        compat_alias: Some("/scene/{sceneName}"),
        type_tag: "—",
        range: "announce / trigger",
        direction: OscDirection::Both,
        family: OscRouteFamily::Scene,
    },
];

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — Wave 2.1 Lane D.3 OSC
    //! route registry seed + bind / unbind drift pins.
    //!
    //! T1 per `docs/43 §15.5`: each test pins an in-source state-machine
    //! invariant the [`OscRouteRegistry`] must honour for the
    //! shell-side `osc_routes_snapshot` / `bind_osc_route` /
    //! `unbind_osc_route` Tauri commands to compose correctly.

    use super::{OscDirection, OscRouteError, OscRouteFamily, OscRouteRegistry};

    #[test]
    fn empty_registry_has_no_entries() {
        let registry = OscRouteRegistry::empty();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn default_seed_populates_canonical_routes() {
        let registry = OscRouteRegistry::with_default_seed();
        let snapshot = registry.snapshot();
        assert!(!snapshot.is_empty());
        // Canonical addresses sorted lexicographically; the first
        // entry is the `/audio/threshold` family.
        let first = snapshot.first().expect("seed table non-empty");
        assert!(first.canonical.starts_with("/audio"));
        // Every seeded entry starts unbound.
        assert!(
            snapshot.iter().all(|e| e.bind_target.is_none()),
            "no entry should be bound at construction"
        );
    }

    #[test]
    fn default_seed_carries_expected_family_set() {
        let registry = OscRouteRegistry::with_default_seed();
        let families: std::collections::HashSet<_> =
            registry.snapshot().into_iter().map(|e| e.family).collect();
        // The seed table covers all seven family discriminants.
        for family in [
            OscRouteFamily::Control,
            OscRouteFamily::Scene,
            OscRouteFamily::Preset,
            OscRouteFamily::Snap,
            OscRouteFamily::Playlist,
            OscRouteFamily::Audio,
            OscRouteFamily::Reserved,
        ] {
            assert!(
                families.contains(&family),
                "seed table missing family {family:?}"
            );
        }
    }

    #[test]
    fn bind_sets_target_on_known_route() {
        let mut registry = OscRouteRegistry::with_default_seed();
        registry
            .bind("/controls/{bank}/{name}", "control:scene:fader-1")
            .expect("known route");
        let entry = registry
            .get("/controls/{bank}/{name}")
            .expect("seeded route");
        assert_eq!(entry.bind_target.as_deref(), Some("control:scene:fader-1"),);
    }

    #[test]
    fn bind_returns_unknown_route_error_for_unseeded_canonical() {
        let mut registry = OscRouteRegistry::with_default_seed();
        let err = registry
            .bind("/this/route/does/not/exist", "anything")
            .unwrap_err();
        assert_eq!(
            err,
            OscRouteError::UnknownRoute("/this/route/does/not/exist".to_string())
        );
    }

    #[test]
    fn unbind_clears_target_on_known_route() {
        let mut registry = OscRouteRegistry::with_default_seed();
        registry
            .bind("/scenes/{sceneName}", "scene:s-12")
            .expect("known route");
        registry.unbind("/scenes/{sceneName}").expect("known route");
        let entry = registry.get("/scenes/{sceneName}").expect("seeded route");
        assert!(entry.bind_target.is_none());
    }

    #[test]
    fn bind_rejects_invalid_target_grammar() {
        let mut registry = OscRouteRegistry::with_default_seed();
        let err = registry
            .bind("/controls/{bank}/{name}", "fader-1")
            .unwrap_err();
        assert!(matches!(err, OscRouteError::InvalidBindTarget(_)));
        let entry = registry
            .get("/controls/{bank}/{name}")
            .expect("seeded route");
        assert!(entry.bind_target.is_none());
    }

    #[test]
    fn unbind_is_idempotent_on_already_unbound_route() {
        let mut registry = OscRouteRegistry::with_default_seed();
        // The seed sets every entry to `bind_target = None`; this
        // unbind should not error.
        registry
            .unbind("/audio/threshold {f}")
            .expect("idempotent unbind");
        registry
            .unbind("/audio/threshold {f}")
            .expect("double unbind is fine");
    }

    #[test]
    fn unbind_returns_unknown_route_error_for_unseeded_canonical() {
        let mut registry = OscRouteRegistry::with_default_seed();
        let err = registry.unbind("/not/a/real/route").unwrap_err();
        assert_eq!(
            err,
            OscRouteError::UnknownRoute("/not/a/real/route".to_string())
        );
    }

    #[test]
    fn snapshot_is_ordered_lexicographically_by_canonical() {
        let registry = OscRouteRegistry::with_default_seed();
        let snapshot = registry.snapshot();
        let mut sorted = snapshot.clone();
        sorted.sort_by(|a, b| a.canonical.cmp(&b.canonical));
        assert_eq!(
            snapshot
                .iter()
                .map(|e| e.canonical.as_str())
                .collect::<Vec<_>>(),
            sorted
                .iter()
                .map(|e| e.canonical.as_str())
                .collect::<Vec<_>>(),
            "snapshot must be lexicographically ordered"
        );
    }

    #[test]
    fn direction_variants_cover_expected_seeded_set() {
        let registry = OscRouteRegistry::with_default_seed();
        let directions: std::collections::HashSet<_> = registry
            .snapshot()
            .into_iter()
            .map(|e| e.direction)
            .collect();
        // The seed table exercises all three direction discriminants.
        for direction in [OscDirection::In, OscDirection::Out, OscDirection::Both] {
            assert!(
                directions.contains(&direction),
                "seed table missing direction {direction:?}"
            );
        }
    }

    // Evidence: EV-INTEGRATION-MATRIX (T1) — PR7 playlist position seed pin.

    #[test]
    fn playlist_position_route_is_in_default_seed() {
        // The `/playlists/position {i}` route must be present in the
        // canonical seed so operators can bind `playlist:position:<i>`
        // targets through the OSC view.
        let registry = OscRouteRegistry::with_default_seed();
        let entry = registry
            .get("/playlists/position {i}")
            .expect("/playlists/position {{i}} must be in the default seed");
        assert_eq!(entry.direction, OscDirection::In);
        assert_eq!(entry.family, OscRouteFamily::Playlist);
        assert!(entry.bind_target.is_none(), "must start unbound");
    }

    #[test]
    fn playlist_position_route_accepts_playlist_position_bind_target() {
        // The seeded position route must accept a `playlist:position:<i>`
        // bind target via the registry bind path.
        let mut registry = OscRouteRegistry::with_default_seed();
        registry
            .bind("/playlists/position {i}", "playlist:position:3")
            .expect("/playlists/position {{i}} accepts playlist:position:3");
        let entry = registry
            .get("/playlists/position {i}")
            .expect("seeded route");
        assert_eq!(entry.bind_target.as_deref(), Some("playlist:position:3"),);
    }

    // Evidence: EV-INTEGRATION-MATRIX (T1) — PR9 audio-threshold
    // bidirectionality seed pin.

    #[test]
    fn audio_threshold_seed_direction_is_both() {
        // The `/audio/threshold {f}` route is bidirectional: the adapter
        // receives the inbound set and ANNOUNCES the new value outbound.
        // This pin ensures the seed declaration matches the documented
        // wire contract.
        let registry = OscRouteRegistry::with_default_seed();
        let entry = registry
            .get("/audio/threshold {f}")
            .expect("audio threshold route must be in the default seed");
        assert_eq!(
            entry.direction,
            OscDirection::Both,
            "/audio/threshold {{f}} must be Both (inbound set + outbound announce)"
        );
        assert_eq!(entry.family, OscRouteFamily::Audio);
    }
}
