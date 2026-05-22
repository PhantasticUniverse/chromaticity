<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

# Synesthesia Manual parity comparison

_Last refreshed:_ 2026-05-22

Match-or-exceed quality benchmarking of Chromaticity against the public
[Synesthesia Manual](https://app.synesthesia.live/docs/) — the two
documented topics under that group are **Control Hotkeys** and **OSC**.
Per `docs/29-cleanroom-legal-policy-baseline.md` this sweep is
observation-only: hotkey assignments and OSC route strings are described
by shape and family rather than transcribed. The native `chroma_*`
namespace stays clean-room; the documented `syn_*` / `favslot` /
plural-root surface remains the `compat-syn-v1` wire-side alias set.

## TL;DR

| Topic                          | Score | Notes                                                                                                |
| :----------------------------- | :---: | :--------------------------------------------------------------------------------------------------- |
| Control Hotkeys — vocabulary   |   5   | 57 rows across 10 groups vs 4 rows in a single list                                                  |
| Control Hotkeys — overlay      |   4   | `?`-toggle modal with filter + focus-trap + ESC-to-close                                             |
| Control Hotkeys — entitlement  |   5   | Per-row `cap` field wired to `HONORED_TOKEN_CAPABILITIES` (no analogue documented)                   |
| Control Hotkeys — conflict     |   5   | T2 corpus pinning 6-context taxonomy + Cmd↔Ctrl alias normalisation (`EV-HOTKEY-CONFLICT`)           |
| Control Hotkeys — rebinding    |  N/A  | Read-only V1 in both products; V2 scope                                                              |
| OSC — default ports            |   4   | 9000 in / 9001 out pinned by ADR-013 default constants vs operator-chosen ports                      |
| OSC — route taxonomy           |   3   | 7 families with parse-time prefix discrimination (Synesthesia documents a similar family-tree shape) |
| OSC — bind-to-action grammar   |   5   | Typed `BindTarget` enum + parse-at-bind-time validation; no operator-side bind UX documented upstream |
| OSC — prefix collision         |   4   | Reserved `/data/*` prefix with distinct diagnostic; `EV-IMPORT-REFERENCE-SHOW` heuristic for imports |
| OSC — health diagnostics       |   5   | `osc_health_snapshot` IPC + AdapterPill chrome (Synesthesia documents only a "log OSC" checkbox)     |
| OSC — network defaults         |   5   | Loopback-default + opt-in subnet exposure + per-route rate limit + payload caps per ADR-013          |
| OSC — payload hardening        |   5   | 64 KiB packet cap + bundle-depth-4 cap, ADR-013 §5 pinned                                            |

---

## Control Hotkeys

### Vocabulary size and coverage areas

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `src/lib/data/hotkeys.ts:33` (57 rows across 10 groups), `docs-corpus/docs/hotkeys-reference.md:1`, normative spec `docs/42-operator-hotkey-map.md` (cited indirectly via the data file's header comment).
- **Match notes:** The Synesthesia public manual documents control-focused mode as four single-letter actions (D / R / P / L). Chromaticity covers the same four-letter control-focused subset verbatim at `hotkeys.ts:37-42`, with identical semantics (default value / random in range / value from current preset / lock toggle) and the same modal-exit convention (`Esc` exits control-focused mode).
- **Exceed notes:** Beyond control-focused mode, Chromaticity ships nine additional groups covering scene/playlist transport, Snap slots (per-scene quick-recall, the operator-facing rename of `favslot`), display/output, rendering, meta colour pipeline (M/B/G/C/H/Y/I/V/E meta uniforms), audio threshold, navigation panels (command palette / library / settings / mappings), diagnostics, and show-state (new/open/save/undo/redo for `.chromaShow` bundles). The vocabulary is mirrored verbatim in the in-app help page at `docs-corpus/docs/hotkeys-reference.md:6-141`. Public Synesthesia documentation surfaces only the four control-focused keys; Chromaticity's documented vocabulary is roughly an order of magnitude larger.
- **Gap notes:** None against the documented Synesthesia surface. Synesthesia may have a broader undocumented hotkey vocabulary in its commercial build; the public manual is the reference target per this sweep's scope.

### Modifier conventions

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `src/lib/data/hotkeys.ts:33` row literals + Cmd↔Ctrl / Opt↔Alt cross-platform normalisation enforced by `src/lib/__tests__/hotkeys-conflict.test.ts` (`EV-HOTKEY-CONFLICT` T2).
- **Match notes:** Synesthesia's documented control-focused hotkeys are unmodified single keys; Chromaticity matches that posture exactly for the D/R/P/L row family.
- **Exceed notes:** Chromaticity defines explicit cross-platform aliasing (every Cmd binding lists its Ctrl twin; every Opt binding lists its Alt twin) per `docs/42` Sec.Conflict-and-priority-rules, validated by 8 T2 fixtures in `hotkeys-conflict.test.ts`. The 6-context resolution taxonomy (global / control-focused / modal-meta / authoring-env / command-palette / text-input) governs which binding fires when multiple match a given key. Public Synesthesia documentation mentions no modifier conventions or focus-context resolution.
- **Gap notes:** None.

### Per-row entitlement / license tier field

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `src/lib/data/hotkeys.ts:24` `cap?: string` field; ADR-009 `HONORED_TOKEN_CAPABILITIES` 13-cap honored set (`engine/entitlement/src/enforcement.rs`).
- **Match notes:** N/A — Synesthesia public documentation does not surface a per-hotkey entitlement / license field.
- **Exceed notes:** Every row carries a `cap` annotation indicating either `free` MVP capability or a specific entitlement string (e.g. `render.custom-resolution` on the resolution-step rows at `hotkeys.ts:177-189`). The capability strings resolve to the 13-cap honored set per Phase-4a Wave 1b-B closure (`EV-ENTITLEMENT-ENFORCEMENT` row in `docs/26-evidence-artifact-registry.md:103`). Capability gating is fail-closed: a binding whose capability is absent does not fire, surfaced visually via `LockedRow` chrome from the W34b adapter UI suite. This is a structural safety axis that has no public Synesthesia analogue.
- **Gap notes:** Production capability swap-in from the live `EntitlementBitmask` IPC stays Phase-3+/4a `EV-ENTITLEMENT-ENFORCEMENT` scope per `CLAUDE.md`; chrome currently renders as if a fully-activated license were installed.

### Hotkey overlay UX

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `src/lib/HotkeyOverlay.svelte:1` (`?`-toggle modal); the data driver is `src/lib/data/hotkeys.ts`.
- **Match notes:** Synesthesia's public documentation references a video tutorial but does not surface an in-app overlay; Chromaticity ships one as the primary discoverability surface.
- **Exceed notes:** The overlay is a modal dialog (`role="dialog"`, `aria-modal="true"`) with a live filter input over key combos / action text / OSC route / GLSL uniform, ESC-to-close, autofocused search field, focus-restoration on close, and an inline two-element focus trap across search field + close button (`HotkeyOverlay.svelte:49-71`). The overlay can be toggled at any time from any view via the global `?` key.
- **Gap notes:** The durable focus-trap primitive (`lib/a11y/focusTrap.ts`) called out in the overlay's header comment is W9-deferred; the inline implementation suffices for the two-tabbable case.

### Conflict-detection and parity-conformance evidence

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `EV-HOTKEY-PARITY-CONFORMANCE` (`docs/26-evidence-artifact-registry.md:88`), `EV-HOTKEY-COMPLETENESS` (line 89), `EV-HOTKEY-CONFLICT` (line 90), all three at Phase-4a Wave 1a closure (merge `a665d72` via PR `#322`, anchor run `25212775046`).
- **Match notes:** N/A — Synesthesia public documentation does not publish conformance evidence for its four-key surface.
- **Exceed notes:** Three independent evidence rows pin three independent axes: (i) parity-conformance — 6 fixtures in `src/lib/__tests__/hotkeys-parity.test.ts` pin the D/R/P/L bindings to the `control_set_default` / `control_randomize` / `control_recall_preset` / `control_toggle_lock` IPC contract via `vi.hoisted` invoke mock; (ii) completeness — 8 fixtures in `hotkeys-completeness.test.ts` validate exactly 10 groups, non-empty keys+action, capability namespace subset, no verbatim duplicates; (iii) conflict — 8 fixtures in `hotkeys-conflict.test.ts` encode the 6-context taxonomy with rules 1-5 from the spec's Conflict-and-priority-rules section. All three Phase-4a Wave 1a scope full.
- **Gap notes:** The runtime keyboard-event listener that wires DOM events to the IPC dispatcher lands in a future Phase-4a PR — the parity corpus uses a self-contained dispatcher harness that re-points at the real symbol when the listener lands without assertion changes.

### Rebinding capability

- **Score:** N/A / OUT-OF-SCOPE
- **Chromaticity equivalent:** `src/lib/data/hotkeys.ts:16-17` header comment ("V1 posture per `docs/42` §V1 rebinding: rows are read-only; rebinding ships in V2").
- **Match notes:** Both products ship V1 hotkey maps as read-only. Synesthesia public documentation provides no rebinding affordance; Chromaticity V1 is explicit about deferring rebinding to V2.
- **Exceed notes:** The data driver is structured for future rebinding (`HotkeyRow` is a readonly interface; the overlay reads it as data, so the underlying source can be swapped to a user-overrideable Settings store).
- **Gap notes:** Rebinding UX is a V2 feature; not in scope for this sweep.

---

## OSC

### Default ports

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `integrations/osc-adapter/src/defaults.rs:39-46` (`DEFAULT_OSC_IN_PORT = 9000` / `DEFAULT_OSC_OUT_PORT = 9001`), pinned by PR `#489` per `CLAUDE.md` Round 3 closure.
- **Match notes:** N/A — Synesthesia public documentation does not specify default ports; operators select arbitrary ports per session.
- **Exceed notes:** Chromaticity pins 9000 inbound / 9001 outbound as out-of-the-box defaults so operator setups work without first-run port-selection friction. The 9000 convention is the de-facto OSC controller convention (TouchOSC default, OSC ecosystem); 9001 separates the outbound socket so local controller feedback can run beside inbound traffic without same-socket binding. The defaults are pinned by drift-test in `defaults.rs:111-114` so any change requires a `policy`-scoped review against ADR-013.
- **Gap notes:** None — the documented Synesthesia surface is the operator-chooses-port model, and our pinned-defaults posture is strictly safer for first-run UX.

### Route taxonomy / route families

- **Score:** 3 / MATCH
- **Chromaticity equivalent:** `integrations/osc-adapter/src/routes.rs:33-50` (7-variant `RoutePrefix` enum: Control / Scene / Preset / Snap / Playlist / Audio / Reserved), `integrations/osc-adapter/src/route_registry.rs:81-97` (7-variant `OscRouteFamily` mirror for the operator-facing registry), `docs-corpus/docs/osc-routes.md:36`.
- **Match notes:** Both products organise routes into nested families by collection root (controls / scenes / presets / playlist / etc.), with action verbs under the collection root. Chromaticity accepts both singular and plural prefix roots (e.g. `/scene` and `/scenes` both parse to `RoutePrefix::Scene`) per `routes.rs:124-133`, retaining the compat-syn-v1 alias forms as documented in `docs-corpus/docs/osc-routes.md:141-148`. The five families that overlap (Control / Scene / Preset / Snap / Playlist) carry equivalent semantics.
- **Exceed notes:** Chromaticity adds two families not surfaced in the documented Synesthesia OSC group: Audio (audio-pipeline read-side routes, including `/audio/threshold` for noise-floor gating) and Reserved (`/data/*` prefix carries a distinct `RouteParseError::PrefixReserved` diagnostic per `routes.rs:78-79` rather than folding into a generic unknown-prefix rejection). The wire-side `routes.rs` parser is structurally separate from the operator-facing `route_registry.rs`: the former parses arbitrary inbound addresses against the canonical prefix vocabulary, the latter curates the operator-visible bind table (8 default seed entries at `route_registry.rs:294-358`).
- **Gap notes:** Synesthesia's public OSC documentation suggests a more elaborate output route surface (real-time audio-engine feedback streams); Chromaticity's `/audio/var/{uniform}` outbound seed at `route_registry.rs:303-310` covers the same shape but the full per-uniform output cadence pipeline is a Phase-4+ wave.

### Bind-to-action dispatch

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `integrations/osc-adapter/src/bindings.rs:22-37` (`BindTarget` typed enum: Control / Scene / Snap / Preset / Playlist), parser `bindings.rs:88-108`, runtime dispatcher `app/shell/src/commands/osc.rs:167-202`.
- **Match notes:** N/A — Synesthesia public documentation does not surface an operator-facing bind-to-action affordance; routes are fixed wire shapes.
- **Exceed notes:** Operators bind any canonical route to a typed runtime action through a `<family>:<param>` grammar (`control:<bank>:<name>`, `scene:<scene_id>`, `snap:<slot>`, `preset:<id>`, `playlist:next|prev`). The parser validates at bind time per `bindings.rs:88-108`, so malformed targets fail loudly with structured `BindError` diagnostics (`bindings.rs:42-55`) instead of becoming silent no-ops. Inbound dispatch through `app/shell/src/commands/osc.rs:184-202` flows through the runtime command bus for ControlSet / PresetRecall / SnapRecall / PlaylistAdvance, with scene-load handled through the shell loader. This unification means a single OSC message can drive any operator action the runtime supports without per-action wiring.
- **Gap notes:** The Reserved (`/data/*`) family rejects at dispatch time with the `PrefixReserved` diagnostic at `bindings.rs:316-322`, leaving room for future structured data streams without breaking the seven-family invariant.

### Route prefix collision detection

- **Score:** 4 / EXCEEDS
- **Chromaticity equivalent:** `routes.rs:78-79` (`RouteParseError::PrefixReserved` discriminant), `EV-IMPORT-REFERENCE-SHOW` heuristic per `docs/26-evidence-artifact-registry.md:101` narrowing 3 (`/chroma/` route-prefix collision detection during reference-show import).
- **Match notes:** Synesthesia documentation acknowledges that scene / control / preset names must exactly match (case rules differ per family) but does not surface a structural collision-detection model.
- **Exceed notes:** The `/data/*` prefix is reserved at parse time with a distinct diagnostic, so any future reserved-prefix expansion (sibling to `/data/`) extends through the same single discriminant rather than diffusing into generic UnknownPrefix rejections. Inbound dispatcher at `bindings.rs:316-322` re-throws PrefixReserved so reserved-route attempts produce an actionable error, not a silent drop. For reference-show imports, the `EV-IMPORT-REFERENCE-SHOW` Wave 4b.7 PR `#368` `engine/authoring/src/import_reference/osc.rs` heuristic flags `/chroma/` prefix collisions during OSC alias import.
- **Gap notes:** The reference-show import heuristic is explicitly synthetic-stand-in per the row's narrowing 3 — a canonical-route registry lookup ships in a post-MVP V1 PR.

### Health / connection diagnostics

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `app/shell/src/commands/osc.rs:67-84` (`osc_health_snapshot` Tauri command), `src/lib/views/Osc.svelte:64` (live consumption), AdapterPill OSC slot per `CLAUDE.md` Wave 2.4 PERF.D closure.
- **Match notes:** Synesthesia's public documentation surfaces a "log OSC" checkbox in the Synesthesia console as the only debug affordance documented; Chromaticity exceeds this with a structured per-poll health snapshot.
- **Exceed notes:** The health snapshot carries three axes: state (Healthy / Recovering / Degraded), attempted-bind counter, and the bound `local_addr` string. The AdapterPill OSC slot in the top bar reads the snapshot live and renders `ok` / `warn` / `error` per the Wave 2.4 PERF.D wiring, so adapter health is visible from every view without opening a dedicated diagnostics panel. The live route registry surface (`osc_routes_snapshot` + `bind_osc_route` + `unbind_osc_route` at `app/shell/src/commands/osc.rs:398-419`) lets operators inspect the bind table and edit it without re-binding the adapter socket.
- **Gap notes:** A structured live OSC packet log (the Synesthesia "log OSC" checkbox equivalent) is currently a hardcoded fixture in the Routes view per `src/lib/views/Osc.svelte:42` comment ("Live log is a hardcoded fixture; real streaming is Phase-3").

### Reserved-prefix safety

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `routes.rs:78-79` + `routes.rs:131` (`/data/` rejected with `PrefixReserved`), `bindings.rs:316-322` (dispatch-time rejection), `bindings.rs:555-575` (test pin).
- **Match notes:** N/A — Synesthesia public documentation does not surface a reserved-prefix concept.
- **Exceed notes:** Reserved prefix is a first-class diagnostic, not folded into unknown-prefix rejection, so the runtime can extend the reserved namespace without changing the unknown-prefix behaviour. Dispatcher rejects inbound messages on a bound reserved route even if the bind table somehow contains one, providing defence-in-depth.
- **Gap notes:** None.

### Reachable from network or loopback-only

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `integrations/osc-adapter/src/defaults.rs:24-31` (`DEFAULT_INBOUND_BIND` / `DEFAULT_OUTBOUND_TARGET` both `Ipv4Addr::LOCALHOST`), `docs/adr/ADR-013-networking-defaults-and-security.md` §1.
- **Match notes:** Synesthesia's documented OSC posture allows any of loopback (`127.0.0.1`), broadcast (`255.255.255.255`), or private network IPs as operator selection; there is no documented default-safety posture.
- **Exceed notes:** Out of the box Chromaticity binds OSC inbound to loopback only. Exposing the inbound listener on a routable interface is an explicit operator opt-in per ADR-013 §1. Outbound also defaults to loopback until the operator configures a specific target. The Settings panel that surfaces the override carries an explicit "may expose Chromaticity to the network" warning per ADR-013 §6. Source-IP allowlist with CIDR notation (`192.168.1.0/24`) ships per ADR-013 §2 with `osc-source-rejected` diagnostic on violations. Per-route rate ceiling 100 msg/s default + per-adapter 100 msg/s floor + 1000 msg/s hard cap per ADR-013 §3, with `osc-rate-limited` throttled diagnostic. Payload hardening pins 64 KiB packet cap and bundle-depth 4 per ADR-013 §5, validated by `defaults.rs:118-122` drift test.
- **Gap notes:** Discovery scope (Bonjour/mDNS) is intentionally not advertised at MVP per ADR-013 §4 — that is a deliberate scope decision, not a gap.

### Payload hardening

- **Score:** 5 / EXCEEDS+
- **Chromaticity equivalent:** `integrations/osc-adapter/src/defaults.rs:53-60` (packet + bundle-depth caps), `integrations/osc-adapter/src/adapter.rs` `validate_packet` (called out in `lib.rs:11-13`).
- **Match notes:** N/A — Synesthesia public documentation does not surface packet-size or bundle-depth caps.
- **Exceed notes:** Oversize packets and over-depth bundles reject with structured diagnostics and no state mutation, blocking trivial stack-exhaustion and resource-exhaustion payloads. The values are pinned by the `EV-NET-HARDENING` evidence row (`docs/26-evidence-artifact-registry.md` Phase-4a Wave 1a closure under anchor run `25213038199`) so any drift fails CI at the exact ADR-013 §5 boundary.
- **Gap notes:** None.

---

## Cross-cutting observations

1. **Surface-area asymmetry.** The public Synesthesia manual covers only the four-letter control-focused hotkey subset and a high-level OSC route family overview. Chromaticity ships a 57-row hotkey vocabulary across 10 groups and a 7-family OSC taxonomy with typed bind-to-action grammar, structured diagnostics, network-default safety, and three independent evidence-tagged conformance corpora. The benchmarking gap is large in our favour on the documented axes.

2. **Clean-room compat-syn-v1 surface is intact.** The `favslot` wire literal is retained under `compat-syn-v1` (rename to operator-facing "Snap" was completed per the `feedback_snap_naming_canonical.md` auto-memory); singular-root and plural-root forms both parse to the canonical `RoutePrefix` enum (`routes.rs:124-133`). This means existing reference-app OSC controllers and TouchOSC layouts continue to function against Chromaticity without operator-side reconfiguration. New native authoring uses the `chroma_*` namespace per the `feedback_syn_prefix_synesthesia_compat.md` clean-room concern.

3. **Entitlement gating is a structural axis the upstream documentation lacks.** Every hotkey row carries a `cap` field that resolves to the 13-cap `HONORED_TOKEN_CAPABILITIES` honored set per Phase-4a Wave 1b-B; capability-gated rows fail closed under absent capabilities and surface `LockedRow` chrome. There is no upstream analogue documented; this widens the safety surface meaningfully for operators on tiered licenses.

4. **Evidence-tagged conformance closes the documentation gap.** Where Synesthesia describes shapes in prose, Chromaticity pins shapes in test code: `EV-HOTKEY-PARITY-CONFORMANCE` / `EV-HOTKEY-COMPLETENESS` / `EV-HOTKEY-CONFLICT` for hotkeys; `EV-NET-HARDENING` / `EV-INTEGRATION-MATRIX` for OSC; `EV-IMPORT-REFERENCE-SHOW` for the bind-target prefix collision heuristic. All in Phase-4a / Phase-4b closure with anchor run IDs and merge commits per `docs/26-evidence-artifact-registry.md`.

5. **Two upstream pages 404'd at fetch time.** `/docs/manual/control_hotkeys.html` and `/docs/manual/osc.html` (the primary manual pages indexed by the top-level docs page) returned 404 from public-internet fetches on 2026-05-22. Coverage shape for those pages was reconstructed from the index page (`/docs/`), the tutorial sibling page (`/docs/tutorials/control_hotkeys.html`), and the MIDI & OSC FAQ (`/docs/faq/midi_osc.html`). When the upstream pages become reachable, this comparison should be revisited per the README's revisiting cadence.
