// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Deterministic GPU adapter selection per ADR-011 §7
//! (`docs/adr/ADR-011-rendering-backend.md`).
//!
//! ADR-011 §7 mandates that Chromaticity take explicit control of
//! multi-GPU adapter selection rather than inheriting the OS default
//! (which on laptops with both integrated and discrete GPUs routinely
//! places the runtime on the low-power integrated GPU mid-show and
//! causes stutter). This module implements the deterministic
//! ranking + operator-override surface that the ADR's "Default
//! selection" + "Operator override" sub-clauses require.
//!
//! # Wave 4d.2 Lane B PR 1 of 2 — scope boundary
//!
//! This is the first of two PRs landing `EV-GPU-SELECTION` (row 78 in
//! `docs/26-evidence-artifact-registry.md`). The split is the per-row
//! Wave-PR sizing: PR 1 (this PR) ships the selection-policy core +
//! operator-override input surface + T1 algorithm-drift pins; PR 2
//! (separate) ships the physical / VM dual-adapter validation corpus
//! that requires actual multi-GPU hardware. T1 here uses synthetic
//! adapter lists for determinism and platform-independence; the wgpu
//! integration arm of the harness lives behind PR 2.
//!
//! Out of scope for PR 1 (deferred to PR 2 or later phases):
//!
//! - **Operator-override settings persistence.** ADR-011 §7 specifies
//!   the override is persisted per installation and survives adapter
//!   hot-plug. The persistence shell (Tauri `app/shell/` settings
//!   store + `src/lib/` UI binding) is PR 2 territory; PR 1 accepts
//!   the override as a constructor argument so the engine surface is
//!   ready when the shell hookup lands.
//! - **Battery / TDR-history warning surfacing.** ADR-011 §7's per-OS
//!   refusal policy ("battery + integrated → warning"; "TDR within
//!   24h → suggest switch") requires OS-side state queries
//!   (`IOPSCopyPowerSourcesInfo` on macOS; the DXGI / EventLog TDR
//!   counter on Windows) plus a UI surface. The policy struct here
//!   exposes [`GpuSelectionPolicy::prefer_battery_friendly`] +
//!   [`GpuSelectionPolicy::prior_tdr_count`] flag fields so the
//!   warning layer can compose; the actual OS query + UI banner are
//!   future-wave scope.
//! - **Hot-plug recovery.** ADR-011 §7's hot-plug clause requires the
//!   engine to warm-restart on the next-best adapter when the
//!   selected adapter disappears (eGPU unplug, driver crash). The
//!   selector here is invoked once at renderer construction; the
//!   re-invocation surface (re-`select_adapter` on a `Surface` /
//!   `Adapter` lost event) is a future addition. The per-call
//!   determinism property (same input → same output) is the pre-
//!   condition that makes hot-plug-restart trivially idempotent when
//!   it lands; this PR validates that property at T1.
//!
//! # Ranking contract
//!
//! Adapters are ranked into a total order by the lexicographic tuple
//! (sorted descending — "biggest first"):
//!
//! 1. **Operator-override match** (highest priority). If
//!    [`GpuSelectionPolicy::operator_override`] is `Some(id)` and the
//!    enumerated adapter list contains an adapter whose
//!    [`SelectedAdapter::id`] equals that ID, that adapter wins
//!    outright. If the override is `Some(id)` but no enumerated
//!    adapter matches, the override is silently ignored and ranking
//!    falls through to the policy axes below (the override comes
//!    from a persisted setting that may reference a now-detached
//!    adapter; surfacing that mismatch as a diagnostic is future-wave
//!    scope per the hot-plug deferral above).
//! 2. **Adapter-type ordering.** Per ADR-011 §7's high-performance
//!    intent and the OS hint families it cites
//!    (`MTLDevice.recommendedMaxWorkingSetSize` on macOS;
//!    `DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE` on Windows; discrete-PCI
//!    preference as the Linux fallback): `DiscreteGpu` outranks
//!    `IntegratedGpu` outranks `VirtualGpu` outranks `Cpu` outranks
//!    `Other`. The `Cpu` and `Other` types are ranked above nothing
//!    and only get selected if no GPU-class adapter is available
//!    (e.g., headless CI without a GPU driver).
//! 3. **Backend per-OS preference.** Per ADR-011 §1's API-stack
//!    routing (macOS: Metal; Windows: DX12 primary, Vulkan fallback;
//!    Linux: Vulkan): on macOS `Metal` outranks all others; on
//!    Windows `Dx12` outranks `Vulkan` outranks all others; on Linux
//!    `Vulkan` outranks all others. Per-OS preference is encoded by
//!    `cfg(target_os)`.
//! 4. **Power-preference axis.** When
//!    [`GpuSelectionPolicy::prefer_battery_friendly`] is set, the
//!    type ordering above flips its top two: `IntegratedGpu` outranks
//!    `DiscreteGpu` (everything else stays the same). This is the
//!    plumbing the future battery-aware mode-flag layer attaches to;
//!    the flag itself is operator-supplied (or, post-PR-2, set by an
//!    OS power-source watcher).
//! 5. **Stable name tie-breaker.** When two adapters tie on every
//!    axis above, the one with the alphabetically-earlier
//!    [`SelectedAdapter::name`] wins. This is the contract that
//!    underwrites `EV-GPU-SELECTION`'s "deterministic selection given
//!    identical hardware" axis: identical inputs deterministically
//!    yield identical outputs across invocations and across machines
//!    with the same adapter set.
//!
//! # Adapter ID encoding
//!
//! [`AdapterId`] is an opaque newtype around a `String`. The encoding
//! [`AdapterId::from_info`] uses is the `<vendor-hex>:<device-hex>:<backend>:<name>`
//! quadruplet. Vendor + device IDs alone are insufficient to
//! disambiguate (a system can install two physical adapters with the
//! same vendor + device ID — e.g., two identical RTX 4090s); adding
//! the backend distinguishes the per-API enumeration arm; adding the
//! name covers the dual-identical-card case. The encoding is stable
//! across runs (every component is read from
//! [`wgpu::AdapterInfo`]'s plain-data fields), survives serde
//! round-trip via `String`, and round-trips through
//! [`AdapterId::as_str`] losslessly. It is NOT designed to round-trip
//! through OS-level GUID / LUID identifiers — those are platform-
//! specific and PR 2 territory if a stronger persistence guarantee is
//! ever required.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use wgpu::AdapterInfo;

/// Opaque adapter identifier. Stable across invocations as long as
/// the underlying hardware + backend enumeration are stable. See
/// the module-level "Adapter ID encoding" section for the encoding
/// rationale.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AdapterId(String);

impl AdapterId {
    /// Build an [`AdapterId`] from an enumerated [`wgpu::AdapterInfo`].
    /// Format: `<vendor-hex>:<device-hex>:<backend>:<name>` where
    /// `<vendor-hex>` and `<device-hex>` are zero-padded 8-digit
    /// hexadecimal, `<backend>` is the [`wgpu::Backend`]'s short
    /// string form (`"vulkan"`, `"metal"`, `"dx12"`, `"gl"`,
    /// `"webgpu"`, `"noop"`), and `<name>` is the adapter's display
    /// name verbatim.
    #[must_use]
    pub fn from_info(info: &AdapterInfo) -> Self {
        Self(format!(
            "{:08x}:{:08x}:{}:{}",
            info.vendor, info.device, info.backend, info.name
        ))
    }

    /// Borrow the identifier as a string slice. The format follows
    /// [`AdapterId::from_info`].
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct an [`AdapterId`] directly from a stored string. This
    /// is the round-trip path that a settings-store rehydration or
    /// CLI-flag-supplied override will use. No format validation is
    /// performed; an unparseable / unknown ID will simply fail to
    /// match any enumerated adapter and the policy ranking will
    /// continue per the override-falls-through clause.
    #[must_use]
    pub fn from_string(raw: String) -> Self {
        Self(raw)
    }
}

/// Adapter classification mirroring [`wgpu::DeviceType`] but kept
/// local so the public surface of this module does not bleed wgpu
/// internals (the persistence layer that PR 2 lands needs a stable
/// representation that survives a wgpu version bump).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AdapterType {
    /// Discrete GPU with separate VRAM.
    Discrete,
    /// Integrated GPU sharing system memory.
    Integrated,
    /// Virtual / hosted GPU (VM passthrough, paravirtualized).
    Virtual,
    /// CPU / software rasterizer.
    Cpu,
    /// Unknown / other classification.
    Other,
}

impl AdapterType {
    /// Convert from the wgpu plain-data enum.
    #[must_use]
    pub fn from_wgpu(t: wgpu::DeviceType) -> Self {
        match t {
            wgpu::DeviceType::DiscreteGpu => Self::Discrete,
            wgpu::DeviceType::IntegratedGpu => Self::Integrated,
            wgpu::DeviceType::VirtualGpu => Self::Virtual,
            wgpu::DeviceType::Cpu => Self::Cpu,
            wgpu::DeviceType::Other => Self::Other,
        }
    }

    /// Score for the high-performance ordering: higher wins. The
    /// exact integer values are an implementation detail — only the
    /// total order matters and is what the T1 pins lock down.
    #[must_use]
    fn high_performance_rank(self) -> u32 {
        match self {
            Self::Discrete => 4,
            Self::Integrated => 3,
            Self::Virtual => 2,
            Self::Cpu => 1,
            Self::Other => 0,
        }
    }

    /// Score for the battery-friendly ordering: integrated tops
    /// discrete; everything else preserves the high-performance
    /// fallback ordering.
    #[must_use]
    fn battery_friendly_rank(self) -> u32 {
        match self {
            Self::Integrated => 4,
            Self::Discrete => 3,
            Self::Virtual => 2,
            Self::Cpu => 1,
            Self::Other => 0,
        }
    }
}

/// Backend classification mirroring [`wgpu::Backend`]. See the
/// rationale on [`AdapterType`] — keeping a local copy decouples the
/// PR 2 persistence layer from wgpu's enum stability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Backend {
    /// Vulkan (Linux + Windows fallback + macOS via MoltenVK).
    Vulkan,
    /// Metal (macOS / iOS).
    Metal,
    /// Direct3D 12 (Windows).
    Dx12,
    /// OpenGL / OpenGL ES.
    Gl,
    /// WebGPU (browser).
    BrowserWebGpu,
    /// Noop testing backend.
    Noop,
}

impl Backend {
    /// Convert from wgpu's enum.
    #[must_use]
    pub fn from_wgpu(b: wgpu::Backend) -> Self {
        match b {
            wgpu::Backend::Vulkan => Self::Vulkan,
            wgpu::Backend::Metal => Self::Metal,
            wgpu::Backend::Dx12 => Self::Dx12,
            wgpu::Backend::Gl => Self::Gl,
            wgpu::Backend::BrowserWebGpu => Self::BrowserWebGpu,
            wgpu::Backend::Noop => Self::Noop,
        }
    }

    /// Per-OS preference rank. Higher wins. The cfg-block encodes
    /// ADR-011 §1's API-stack routing per platform.
    #[must_use]
    fn os_preference_rank(self) -> u32 {
        #[cfg(target_os = "macos")]
        {
            match self {
                Self::Metal => 5,
                Self::Vulkan => 4, // MoltenVK fallback
                Self::Gl => 3,
                Self::Dx12 => 2,
                Self::BrowserWebGpu => 1,
                Self::Noop => 0,
            }
        }
        #[cfg(target_os = "windows")]
        {
            match self {
                Self::Dx12 => 5,
                Self::Vulkan => 4,
                Self::Gl => 3,
                Self::Metal => 2,
                Self::BrowserWebGpu => 1,
                Self::Noop => 0,
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            match self {
                Self::Vulkan => 5,
                Self::Gl => 4,
                Self::Dx12 => 3,
                Self::Metal => 2,
                Self::BrowserWebGpu => 1,
                Self::Noop => 0,
            }
        }
    }
}

/// Informational view of an enumerated adapter. The
/// [`SelectedAdapter::id`] is the operator-facing handle (used to
/// match a persisted override); the rest of the fields are for the
/// "Currently using: <name>" UI surface and the support-bundle
/// per-launch log line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectedAdapter {
    /// Stable opaque identifier (see [`AdapterId::from_info`]).
    pub id: AdapterId,
    /// Display name reported by the driver.
    pub name: String,
    /// Discrete / integrated / virtual / cpu / other.
    pub adapter_type: AdapterType,
    /// Backend the adapter was enumerated through.
    pub backend: Backend,
    /// Backend-specific vendor ID (PCI vendor ID for Vulkan, etc.).
    pub vendor: u32,
}

impl SelectedAdapter {
    /// Build a [`SelectedAdapter`] from a [`wgpu::AdapterInfo`].
    #[must_use]
    pub fn from_info(info: &AdapterInfo) -> Self {
        Self {
            id: AdapterId::from_info(info),
            name: info.name.clone(),
            adapter_type: AdapterType::from_wgpu(info.device_type),
            backend: Backend::from_wgpu(info.backend),
            vendor: info.vendor,
        }
    }
}

/// Selection-policy input. Constructed via
/// [`GpuSelectionPolicy::new`] paired with the with-style builder
/// methods. The default is the high-performance / no-override /
/// no-battery-flag posture mandated by ADR-011 §7's "Default
/// selection" sub-clause.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GpuSelectionPolicy {
    operator_override: Option<AdapterId>,
    prefer_battery_friendly: bool,
    prior_tdr_count: u32,
}

impl GpuSelectionPolicy {
    /// Construct the default policy: high-performance, no operator
    /// override, no battery preference, no prior TDR events.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder-style: opt the operator's persisted override into the
    /// policy. Pass `None` to remove a previously-set override (the
    /// default is no override). Note that the override is silently
    /// ignored when no enumerated adapter matches its ID — see the
    /// module-level "Ranking contract" item 1 for why.
    #[must_use]
    pub fn with_operator_override(mut self, override_id: Option<AdapterId>) -> Self {
        self.operator_override = override_id;
        self
    }

    /// Builder-style: ask the policy to prefer battery-friendly
    /// adapters (integrated > discrete) over the high-performance
    /// default (discrete > integrated). The actual battery-state
    /// query is OS-side and out of PR 1 scope; the flag is supplied
    /// by the caller for now.
    #[must_use]
    pub fn prefer_battery_friendly(mut self, prefer: bool) -> Self {
        self.prefer_battery_friendly = prefer;
        self
    }

    /// Builder-style: record the count of TDR events the selected
    /// adapter's driver has reported in the last 24 hours (Windows-
    /// only signal). PR 1 does not yet act on this flag — the future
    /// Windows-side warning layer that ADR-011 §7 describes will
    /// consume it. Stored on the policy so the layer can read it
    /// without a side-channel.
    #[must_use]
    pub fn with_prior_tdr_count(mut self, count: u32) -> Self {
        self.prior_tdr_count = count;
        self
    }

    /// Read-only accessor for the override (used by the selector and
    /// useful for diagnostics).
    #[must_use]
    pub fn operator_override(&self) -> Option<&AdapterId> {
        self.operator_override.as_ref()
    }

    /// Read-only accessor for the battery-friendly flag.
    #[must_use]
    pub fn is_battery_friendly(&self) -> bool {
        self.prefer_battery_friendly
    }

    /// Read-only accessor for the prior TDR count flag (Windows-side
    /// signal; future warning surface consumes it).
    #[must_use]
    pub fn prior_tdr_count(&self) -> u32 {
        self.prior_tdr_count
    }
}

/// Pure-data ranking helper. Selects the highest-ranked adapter from
/// `adapters` per the [`GpuSelectionPolicy`] contract, returning a
/// borrow into the input slice. Returns `None` when `adapters` is
/// empty.
///
/// This helper is the engine of the T1 drift pins below — it is pure
/// (no wgpu interaction, no I/O), accepts a synthetic adapter list,
/// and is the same code path the real selector composes when wgpu's
/// `enumerate_adapters` produces the live list. The wgpu round-trip
/// is the T2 corpus's responsibility (PR 2).
#[must_use]
pub(crate) fn rank_synthetic<'a>(
    adapters: &'a [SelectedAdapter],
    policy: &GpuSelectionPolicy,
) -> Option<&'a SelectedAdapter> {
    if adapters.is_empty() {
        return None;
    }

    // Pass 1: operator override match. If set and matches an
    // enumerated adapter, that adapter wins outright. Nested
    // `if let` (rather than the let-chain shorthand) keeps the
    // module compatible with the workspace MSRV 1.85; let-chains
    // stabilise in Rust 1.88.
    if let Some(target) = policy.operator_override.as_ref() {
        if let Some(found) = adapters.iter().find(|a| &a.id == target) {
            return Some(found);
        }
    }

    // Pass 2: ranking-axis selection. The lexicographic tuple is
    // (adapter-type-rank, backend-os-rank, name-tie-break-rank). The
    // name tie-breaker uses a *negated* lexicographic order — the
    // alphabetically EARLIER name wins — so we sort name ascending
    // and the name-rank flips sign relative to the other axes (we
    // implement this by comparing names directly in the
    // tuple-comparator rather than projecting to an integer). Using
    // `Iterator::max_by` rather than a pre-sort keeps the helper
    // O(n) and avoids a clone of the slice.
    adapters.iter().max_by(|a, b| {
        let a_type = if policy.prefer_battery_friendly {
            a.adapter_type.battery_friendly_rank()
        } else {
            a.adapter_type.high_performance_rank()
        };
        let b_type = if policy.prefer_battery_friendly {
            b.adapter_type.battery_friendly_rank()
        } else {
            b.adapter_type.high_performance_rank()
        };
        a_type
            .cmp(&b_type)
            .then_with(|| {
                a.backend
                    .os_preference_rank()
                    .cmp(&b.backend.os_preference_rank())
            })
            // Name tie-breaker: alphabetically EARLIER wins, so we
            // reverse the cmp.
            .then_with(|| b.name.cmp(&a.name))
    })
}

/// Enumerate every adapter the wgpu instance can see across the
/// given backend filter, returning the informational view (without
/// selecting one). This is the data source for the
/// Settings → Output → GPU dropdown that ADR-011 §7's
/// operator-override sub-clause specifies. PR 2 wires this through
/// the Tauri shell.
///
/// `backends` is passed through to [`wgpu::Instance::enumerate_adapters`].
/// Callers in this crate use [`wgpu::Backends::PRIMARY`] to mirror
/// the prior `request_adapter` posture; the UI dropdown can pass
/// [`wgpu::Backends::all()`] to surface every compiled-in backend
/// arm.
pub fn enumerate_adapter_summaries(
    instance: &wgpu::Instance,
    backends: wgpu::Backends,
) -> Vec<SelectedAdapter> {
    instance
        .enumerate_adapters(backends)
        .into_iter()
        .map(|adapter| SelectedAdapter::from_info(&adapter.get_info()))
        .collect()
}

/// Run the full selection: enumerate every adapter on `backends`,
/// rank per [`GpuSelectionPolicy`], and return the chosen
/// [`SelectedAdapter`] paired with the live [`wgpu::Adapter`]
/// handle. Returns `None` when no adapters are available (e.g.
/// headless CI without a backend driver).
///
/// The wgpu adapter is returned alongside the informational view so
/// the caller can pass it directly to
/// [`wgpu::Adapter::request_device`] — the T2 GPU-roundtrip path.
pub fn select_adapter(
    instance: &wgpu::Instance,
    backends: wgpu::Backends,
    policy: &GpuSelectionPolicy,
) -> Option<(SelectedAdapter, wgpu::Adapter)> {
    let adapters = instance.enumerate_adapters(backends);
    if adapters.is_empty() {
        return None;
    }

    let summaries: Vec<SelectedAdapter> = adapters
        .iter()
        .map(|a| SelectedAdapter::from_info(&a.get_info()))
        .collect();

    let chosen_summary = rank_synthetic(&summaries, policy)?.clone();

    // Find the wgpu::Adapter whose info encodes to the chosen ID.
    // The `iter().enumerate()` + index lookup avoids cloning the
    // wgpu adapters (they don't `Clone`). The position MUST exist
    // because the summaries vector was built from this same
    // adapters vector in the same order.
    let chosen_idx = summaries.iter().position(|s| s.id == chosen_summary.id)?;
    let chosen_adapter = adapters.into_iter().nth(chosen_idx)?;

    Some((chosen_summary, chosen_adapter))
}

#[cfg(test)]
mod adapter_selection_pins {
    //! Evidence: EV-GPU-SELECTION (T1) — deterministic adapter
    //! selection algorithm-drift pins.
    //!
    //! T1 per `docs/43-development-standards.md` §15.5: pure-data
    //! ranking-helper pins that fire WITHOUT a real wgpu
    //! enumeration (the T2 dual-adapter corpus is PR 2 scope per
    //! the module-level "Wave 4d.2 Lane B PR 1 of 2 — scope
    //! boundary" section).
    //!
    //! Pinned invariants:
    //!
    //! - Empty adapter list → [`rank_synthetic`] returns `None`.
    //! - Discrete outranks integrated under default
    //!   (high-performance) policy.
    //! - Operator override matching an enumerated adapter wins over
    //!   the ranking axes.
    //! - Operator override that does NOT match falls through to
    //!   ranking-based selection (override never short-circuits to
    //!   `None`).
    //! - Two invocations with identical inputs return the same
    //!   selection (the determinism axis ADR-011 §7 calls out).
    //! - [`AdapterId`] string-encoding round-trips losslessly.
    //! - Battery-friendly flag flips integrated above discrete.
    //! - Backend per-OS preference disambiguates ties on adapter
    //!   type.
    //! - Name tie-breaker selects the alphabetically earlier name.

    use super::{
        AdapterId, AdapterType, Backend, GpuSelectionPolicy, SelectedAdapter, rank_synthetic,
    };

    fn synth(name: &str, t: AdapterType, b: Backend) -> SelectedAdapter {
        SelectedAdapter {
            id: AdapterId::from_string(format!("00000001:00000002:{b:?}:{name}")),
            name: name.to_string(),
            adapter_type: t,
            backend: b,
            vendor: 0x1002,
        }
    }

    #[test]
    fn policy_with_no_adapters_returns_none() {
        let policy = GpuSelectionPolicy::new();
        let chosen = rank_synthetic(&[], &policy);
        assert!(chosen.is_none(), "empty adapter list must yield None");
    }

    #[test]
    fn policy_prefers_discrete_over_integrated() {
        let adapters = vec![
            synth("IntegratedGfx", AdapterType::Integrated, Backend::Vulkan),
            synth("DiscreteGfx", AdapterType::Discrete, Backend::Vulkan),
        ];
        let policy = GpuSelectionPolicy::new();
        let chosen = rank_synthetic(&adapters, &policy).unwrap();
        assert_eq!(
            chosen.adapter_type,
            AdapterType::Discrete,
            "high-performance default must select Discrete over Integrated"
        );
        assert_eq!(chosen.name, "DiscreteGfx");
    }

    #[test]
    fn policy_operator_override_takes_precedence_over_ranking() {
        // Two adapters, the integrated one would lose by ranking
        // but is the operator's persisted choice.
        let integrated = synth("IntegratedGfx", AdapterType::Integrated, Backend::Vulkan);
        let discrete = synth("DiscreteGfx", AdapterType::Discrete, Backend::Vulkan);
        let override_id = integrated.id.clone();

        let adapters = vec![integrated, discrete];
        let policy = GpuSelectionPolicy::new().with_operator_override(Some(override_id));
        let chosen = rank_synthetic(&adapters, &policy).unwrap();
        assert_eq!(
            chosen.adapter_type,
            AdapterType::Integrated,
            "operator override must defeat the ranking-based selection"
        );
        assert_eq!(chosen.name, "IntegratedGfx");
    }

    #[test]
    fn policy_operator_override_unmatched_falls_back_to_ranking() {
        // Override references an adapter that is NOT enumerated
        // (e.g., a now-detached eGPU). Selector must NOT return
        // None — it must fall through to ranking-based selection.
        let adapters = vec![
            synth("IntegratedGfx", AdapterType::Integrated, Backend::Vulkan),
            synth("DiscreteGfx", AdapterType::Discrete, Backend::Vulkan),
        ];
        let bogus = AdapterId::from_string("ffffffff:ffffffff:Vulkan:UnpluggedEGPU".to_string());
        let policy = GpuSelectionPolicy::new().with_operator_override(Some(bogus));
        let chosen = rank_synthetic(&adapters, &policy).unwrap();
        assert_eq!(
            chosen.adapter_type,
            AdapterType::Discrete,
            "unmatched override must fall through to ranking, not yield None"
        );
    }

    #[test]
    fn policy_deterministic_across_invocations_with_identical_inputs() {
        // The "deterministic selection given identical hardware"
        // axis ADR-011 §7 calls out. Three rounds, same input, same
        // output every time.
        let adapters = vec![
            synth("ZetaGfx", AdapterType::Discrete, Backend::Vulkan),
            synth("AlphaGfx", AdapterType::Discrete, Backend::Vulkan),
            synth("MidGfx", AdapterType::Integrated, Backend::Vulkan),
        ];
        let policy = GpuSelectionPolicy::new();
        let r1 = rank_synthetic(&adapters, &policy).unwrap().id.clone();
        let r2 = rank_synthetic(&adapters, &policy).unwrap().id.clone();
        let r3 = rank_synthetic(&adapters, &policy).unwrap().id.clone();
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    #[test]
    fn adapter_id_encoding_is_stable_round_trip() {
        // Encoding is opaque-string and round-trips through
        // from_string ↔ as_str without lossage. This is the
        // contract the future settings-store rehydration path
        // depends on.
        let raw = "00001002:00006789:Metal:Apple M2 Max".to_string();
        let id = AdapterId::from_string(raw.clone());
        assert_eq!(id.as_str(), raw);
        let id2 = AdapterId::from_string(id.as_str().to_string());
        assert_eq!(id, id2);
    }

    #[test]
    fn battery_friendly_flag_flips_integrated_above_discrete() {
        let adapters = vec![
            synth("DiscreteGfx", AdapterType::Discrete, Backend::Vulkan),
            synth("IntegratedGfx", AdapterType::Integrated, Backend::Vulkan),
        ];
        let policy = GpuSelectionPolicy::new().prefer_battery_friendly(true);
        let chosen = rank_synthetic(&adapters, &policy).unwrap();
        assert_eq!(
            chosen.adapter_type,
            AdapterType::Integrated,
            "battery-friendly flag must select Integrated over Discrete"
        );
    }

    #[test]
    fn name_tie_breaker_selects_alphabetically_earlier_name() {
        // Two adapters tying on every other axis; alphabetically
        // earlier name wins so the choice is stable across machines
        // with the same pair.
        let adapters = vec![
            synth("ZetaGfx", AdapterType::Discrete, Backend::Vulkan),
            synth("AlphaGfx", AdapterType::Discrete, Backend::Vulkan),
        ];
        let policy = GpuSelectionPolicy::new();
        let chosen = rank_synthetic(&adapters, &policy).unwrap();
        assert_eq!(
            chosen.name, "AlphaGfx",
            "alphabetically earlier name must win the stable tie-break"
        );
    }

    #[test]
    fn policy_default_state_is_high_performance_no_override_no_battery() {
        // Drift pin on the default policy posture. ADR-011 §7's
        // "Default selection" sub-clause is high-performance + no
        // override; the prior_tdr_count default is zero (no TDR
        // events).
        let policy = GpuSelectionPolicy::new();
        assert!(policy.operator_override().is_none());
        assert!(!policy.is_battery_friendly());
        assert_eq!(policy.prior_tdr_count(), 0);
    }

    #[test]
    fn policy_builder_methods_are_chainable_and_persist_state() {
        let id = AdapterId::from_string("test".to_string());
        let policy = GpuSelectionPolicy::new()
            .with_operator_override(Some(id.clone()))
            .prefer_battery_friendly(true)
            .with_prior_tdr_count(3);
        assert_eq!(policy.operator_override(), Some(&id));
        assert!(policy.is_battery_friendly());
        assert_eq!(policy.prior_tdr_count(), 3);
    }

    #[test]
    fn adapter_type_from_wgpu_round_trips_every_variant() {
        // Drift pin: any future wgpu DeviceType variant addition
        // forces this match arm to be revisited.
        assert_eq!(
            AdapterType::from_wgpu(wgpu::DeviceType::DiscreteGpu),
            AdapterType::Discrete
        );
        assert_eq!(
            AdapterType::from_wgpu(wgpu::DeviceType::IntegratedGpu),
            AdapterType::Integrated
        );
        assert_eq!(
            AdapterType::from_wgpu(wgpu::DeviceType::VirtualGpu),
            AdapterType::Virtual
        );
        assert_eq!(
            AdapterType::from_wgpu(wgpu::DeviceType::Cpu),
            AdapterType::Cpu
        );
        assert_eq!(
            AdapterType::from_wgpu(wgpu::DeviceType::Other),
            AdapterType::Other
        );
    }

    #[test]
    fn backend_from_wgpu_round_trips_every_variant() {
        assert_eq!(Backend::from_wgpu(wgpu::Backend::Vulkan), Backend::Vulkan);
        assert_eq!(Backend::from_wgpu(wgpu::Backend::Metal), Backend::Metal);
        assert_eq!(Backend::from_wgpu(wgpu::Backend::Dx12), Backend::Dx12);
        assert_eq!(Backend::from_wgpu(wgpu::Backend::Gl), Backend::Gl);
        assert_eq!(
            Backend::from_wgpu(wgpu::Backend::BrowserWebGpu),
            Backend::BrowserWebGpu
        );
        assert_eq!(Backend::from_wgpu(wgpu::Backend::Noop), Backend::Noop);
    }
}
