<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# OSC Route Reference

Chromaticity exposes Open Sound Control (OSC) inbound and outbound
routes for show-control automation and external surface integration.
This page enumerates the canonical route taxonomy honoured by the OSC
adapter at the top-level prefixes the slice-2 route schema declares.

## Defaults and discovery

By default the OSC adapter binds the inbound listener to the loopback
interface (127.0.0.1) and treats the outbound target as loopback until
the operator configures a routable address. Exposing the inbound
listener on a routable interface is an explicit operator opt-in. The
operator can confirm the bound interface and current rate-limit posture
through the AdapterPill chrome at the top of any view.

Two payload-hardening defaults are pinned by the network adapter:

- The maximum accepted OSC packet size is 64 KiB. Oversize packets are
  rejected with an `osc-malformed`-class diagnostic and no state
  mutation.
- The maximum nested OSC bundle depth is 4. Deeper payloads are
  rejected for the same reason.

Per-route inbound traffic is rate-limited at a default ceiling of 100
messages per second per `(adapter, route)` pair. Over-rate messages are
dropped with an `osc-rate-limited` diagnostic emitted no more than once
per second per route to prevent the diagnostics channel itself from
saturating.

## Top-level prefixes

The slice-2 route schema declares four canonical top-level prefixes;
addresses outside this set are rejected at parse time.

| Prefix | Purpose |
| --- | --- |
| `/controls` | Scene-author parameter control (control-bank reads and writes) |
| `/scenes` | Scene-selection and scene-launch routes |
| `/audio` | Audio-pipeline routes (threshold, level discovery) |
| `/render` | Render-loop and resolution control |

Companion route families used by the operator hotkey vocabulary
(`/playlist`, `/playlists`, `/favslots`, `/presets`) are part of the
broader IO surface specified in the integrations document; see the
plurality and alias rules below.

## Scene routes

| Route | Direction | Args | Description |
| --- | --- | --- | --- |
| `/scenes` | outbound | (list payload) | Library scene-name listing emitted on bind and on library-change |
| `/scenes/{name} {preset?}` | inbound | scene name; optional preset name | Launch scene; unsupplied preset resolves to Snap 0 / gold |
| `/scenes/{name}` | outbound | scene name | Scene-change announcement emitted on every cut or transition completion |

The legacy singular-root forms `/scene/{name}` and `/scene/{name}
{preset}` are accepted as `compat-syn-v1` aliases. Outbound emission
during the dual-emit window publishes both the canonical plural form
and the singular alias.

## Snap (favslots) routes

Snap is the operator-facing name for per-scene quick-recall slots; the
wire-level literal `favslots` is retained under `compat-syn-v1` for
compatibility.

| Route | Direction | Args | Description |
| --- | --- | --- | --- |
| `/favslots/{n}` | inbound (apply) | slot index `n` in `[0, 15]` | Recall Snap slot `n` for the current scene |
| `/favslots/{n}/save` | inbound (save) | slot index `n` | Overwrite Snap slot `n` with the current state |
| `/favslots/{n}/clear` | inbound (clear) | slot index `n` | Delete Snap slot `n`; the slot returns to unbound |
| `/favslots/{n}/name {string}` | outbound | slot index, name string | Snap-slot name announcement on show load and after any save or clear |

The apply / save / clear distinction is wire-level explicit through the
route suffix; there is no implicit mode selection from message
arguments. The legacy `/favslots/{n} {mode}` form with a string argument
of `"apply"`, `"save"`, or `"clear"` is accepted as a `compat-syn-v1`
alias.

## Playlist routes

| Route | Direction | Args | Description |
| --- | --- | --- | --- |
| `/playlists` | outbound | (list payload) | Show-playlist listing on bind |
| `/playlists/next` | inbound | (none) | Advance to next scene in playlist |
| `/playlists/previous` | inbound | (none) | Step back to previous scene in playlist |
| `/playlists/position {i}` | inbound | integer index; negative counts from end | Jump to absolute playlist position |
| `/playlists/play` | inbound | (none) | Resume playlist transport |
| `/playlists/toggleplay` | inbound | (none) | Toggle playlist play / pause |
| `/playlists/select {name}` | inbound | playlist name | Select a named playlist as the active one |
| `/playlists/current {name}` | outbound | active playlist name | Emitted on playlist change |
| `/playlists/position {i}` | outbound | current position | Emitted on every position change |

The singular-root action forms `/playlist/next`, `/playlist/previous`,
`/playlist/toggleplay` are accepted as `compat-syn-v1` aliases for the
plural-root canonical forms. The hotkey overlay surfaces these singular
forms in its OSC column for compatibility with reference scenes.

## Render routes

| Route | Direction | Args | Description |
| --- | --- | --- | --- |
| `/render/enabled {0|1}` | inbound | flag | Toggle the render loop on or off; the GPU scheduler halts scene rendering while UI and adapter threads stay responsive |
| `/render/resolution {w} {h}` | inbound | width, height | Resize the render output surface (capability-gated `render.custom-resolution`) |
| `/render/fps {float}` | inbound | target FPS | Override the render FPS target |

`/render/enabled 0` is non-destructive: outbound audio, MIDI, and OSC
state continue to publish, and shader pipelines remain warm so the
restoration frame on `/render/enabled 1` is within one frame budget.
Useful for SMPTE-slaved setups, A/V calibration, and operator-triggered
"hold current output" cues.

## Audio routes

| Route | Direction | Args | Description |
| --- | --- | --- | --- |
| `/audio/threshold {float}` | inbound and outbound | clamped to `[0, 1]` | Set or announce the audio noise-floor gate; updates `syn_Threshold` within one audio hop |
| `/audio/threshold/sample` | inbound | (none) | Sample the current 200 ms rolling RMS of `syn_Level` and set the threshold to that value; emits `/audio/threshold {float}` outbound on completion |

Audio hops with RMS below the threshold publish zeroed `syn_Level*` and
`syn_*Hits` family uniforms; the threshold is the operator's primary
tool for rejecting room noise and HVAC bleed.

## Controls routes

The `/controls` namespace is the runtime parameter-write surface used
by external surfaces (TouchOSC, MIDI-to-OSC bridges, custom show
runners). The canonical shape is `/controls/{bank}/{control}` followed
by a typed value argument; the slice-2 router parses these into a
canonical segment vector for the slice-3 dispatch surface that lands
the typed-command translation. Author-side scenes declare their
expected control bank in the scene script; the runtime rejects writes
to unknown banks with a `route-mismatch` diagnostic.

## Plurality convention

The collection roots use the plural form (`/scenes`, `/presets`,
`/playlists`, `/favslots`, `/controls`); action verbs and listing
queries live under the collection root. Chromaticity V1 canonical form
is plural-root throughout inbound and outbound emission. The singular
reference-app forms (`/scene/{name}`, `/preset/{name}`,
`/playlist/next`) are accepted as `compat-syn-v1` aliases.

---

*Authored from `integrations/osc-adapter/src/routes.rs` (slice-2
route-prefix taxonomy) and `integrations/osc-adapter/src/defaults.rs`
(network defaults), against
`docs/adr/ADR-013-networking-defaults-and-security.md` and the
`docs/08-io-integrations.md` OSC sections. Clean-room per
`docs/29-cleanroom-legal-policy-baseline.md`.*
