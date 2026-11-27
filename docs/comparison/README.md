<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Chromaticity -->

# Synesthesia parity comparison sweep

Match-or-exceed quality benchmarking of Chromaticity against the public
[Synesthesia documentation](https://app.synesthesia.live/docs/), commissioned
2026-05-22. Living document — revisited as Synesthesia ships updates and
Chromaticity advances through Phase-4d / Phase-5.

## Posture (clean-room)

Per `docs/29-cleanroom-legal-policy-baseline.md` this sweep is **observation
for quality benchmarking only**. No Synesthesia source is read; no UI / API
shapes are copied. The aim is to ensure Chromaticity matches or exceeds the
**public-documentation surface** on the user-visible features that surface
documents. Native namespace (`chroma_*`) implementations remain clean-room;
compat namespace (`syn_*`) is the documented Synesthesia-compat API surface.

## Scoring rubric

Each Synesthesia feature / pathway / topic is scored on a 5-point axis with
a status tag.

| Score | Tag        | Meaning                                                                                                          |
| :---: | :--------- | :--------------------------------------------------------------------------------------------------------------- |
|   5   | EXCEEDS+   | Chromaticity ships a meaningfully more capable / safer / more deterministic implementation than documented.      |
|   4   | EXCEEDS    | Chromaticity meets the requirement and adds quality on at least one axis (correctness, ergonomics, safety, etc). |
|   3   | MATCH      | Functional parity in the relevant scope. Same capability surface, comparable polish.                             |
|   2   | PARTIAL    | Chromaticity has scaffolding, partial coverage, or AO-deferred work; functional gap remains.                     |
|   1   | GAP        | Chromaticity does not yet cover this; needs scoping decision (a future wave or AO-deferred indefinitely).        |
|  N/A  | OUT-OF-SCOPE | Out of scope for Chromaticity by deliberate design (e.g., a compat-only surface, ISF native import, etc).      |

Per-item template:

```markdown
### <Synesthesia feature name>

- **Score:** <N> / <TAG>
- **Chromaticity equivalent:** <code surface, file path, ADR / EV-ID reference>
- **Match notes:** <what we cover at parity>
- **Exceed notes:** <where we are stronger; cite ADR / EV-ID / Phase tag>
- **Gap notes:** <where we fall short, with the planning trail or future wave reference>
```

## Two layers — overview and features

The sweep ships in two granularities so the AO can read it at either depth:

- **Overview layer** (this directory's flat `.md` files): one scored row per
  documented topic (e.g., "Audio uniforms" as a single item). ~73 items total.
  Lands the headline at-a-glance picture per Synesthesia documentation group.
- **Feature-level deep-dive layer** (`features/`): one scored row per
  individual symbol / function / setting / workflow step / capability. ~857
  items total. Used to surface clustered gaps that the overview layer
  aggregates away (e.g., 64 hg_sdf functions blocked on one vendor PR).

The two layers share the same scoring rubric and clean-room posture. They
are complementary — overview for headline, features for precision.

## Section index

| Topic        | Overview                          | Feature-level deep dive                          | Synesthesia surface                                                                                            |
| :----------- | :-------------------------------- | :----------------------------------------------- | :------------------------------------------------------------------------------------------------------------- |
| SSF format   | [ssf-format.md](ssf-format.md)    | [features/ssf-format.md](features/ssf-format.md) | Synesthesia Shader Format — audio uniforms, standard uniforms, functions, external libs, JSON, JS, best practices |
| FAQ          | [faq.md](faq.md)                  | [features/faq.md](features/faq.md)               | FAQ — general, troubleshooting, license, audio, video, settings, MIDI & OSC                                    |
| Manual       | [manual.md](manual.md)            | [features/manual.md](features/manual.md)         | Manual — Control Hotkeys, OSC                                                                                  |
| Tutorials    | [tutorials.md](tutorials.md)      | [features/tutorials.md](features/tutorials.md)   | Tutorials — quickstart, first scene, presets/favslots/playlists, hotkeys, OBS, Shadertoy convert, JS handlers, v1.16 walkthrough |
| Resources    | [resources.md](resources.md)      | [features/resources.md](features/resources.md)   | Resources — installers, changelogs, scene resources, GLSL resources                                            |

## Closure log

The sweep is followed by per-wave closure PRs that close clusters of PARTIAL /
GAP rows. Score deltas roll up here; per-row scores in the individual docs
stay frozen at the snapshot date until the next full re-sweep.

### Parity-closure wave 1 (2026-05-22, ~95 rows lifted)

- **Lane A** (`#495` / `31e798c`): release.yml macOS Intel matrix arm + tauri.conf.json `bundle.resources` widening to all 16 `.chroma` packages. Closes the Mac Intel installer GAP toward PARTIAL, moves `EV-LIBRARY-CORPUS-MIN` from InProgress to Pass.
- **Lane B** (`#496` / `d915d98`): 5 new docs-corpus pages (quickstart / livestream / best-practices / system-requirements / supported-codecs) wired into `mkdocs.yml` + `bundle.resources`. Closes ~28-30 docs-cluster PARTIAL/GAP rows across SSF / FAQ / Tutorials / Resources.
- **Lane C** (`#497` / `f93e841`): vendored Mercury Demogroup's hg_sdf under `third_party/hg_sdf/` electing MIT (upstream dual MIT OR CC-BY-NC-4.0; MIT track permanent). Closes 64 SSF external-library PARTIAL rows — the largest single-PR closure.

Lane D (optional polish) deferred: XY 2D control widget, OSC playlist bind enum widening, `BindTarget::Scene` two-arg form, `/audio/threshold` direction `Both`, OSC compat dual-emit window. Each is small but low-leverage relative to wave-1 work; revisit if AO smoke surfaces fresh issues.

### Parity-closure wave 2 — Lane D (2026-05-22)

Closed the wave-1 deferred Lane D cluster across 11 PRs (`#499`–`#509`). The five Lane D items:

- **Interactive XY 2D control pad** — additive runtime `vec2` control values (a new `ControlSet2D` command + `ControlChanged2D` event riding a V2 migration-envelope extension), preset save/recall capture, the `set_control_2d` / `get_control_2d` IPC bridge with the `ControlValue2` ts-rs type, and the interactive `Xy.svelte` widget wired into Perform.
- **OSC playlist `position` bind** — a Tier-1 absolute `Command::PlaylistSetPosition` plus the `/playlists/position {i}` OSC bind. (`play` / `pause` / `select` stay Phase-4+ per the `EV-PRESET-PLAYLIST` narrowing — they need a playlist-entry model.)
- **`BindTarget::Scene` two-arg form** — `scene:<id>:<preset>` so a single OSC route loads a scene and recalls one of its presets.
- **`/audio/threshold` bidirectional** — the route is now `OscDirection::Both`, with an announce-on-change outbound emit.
- **OSC compat dual-emit window** — an opt-in, deadline-bounded window that emits outbound OSC on both the canonical and the singular compat-alias address, built on a new outbound OSC sender (ADR-013 §1/§6 loopback-default, operator-configurable).

Optional wave-1 Lane D items beyond these five remain deferred per AO call.

### Parity-closure wave 3 (2026-11-27)

Major parity closures and Synesthesia/OBS leapfrogs shipped since Wave 2:

- **Live Metal GPU preview & 44 Metal-verified shaders** — real-time 214–704 FPS `wgpu`/Metal rendering pipeline with live Metal GPU thumbnails (`8aafd52`), QuickJS `script.js` host (`30ca792`), and `.synScene` / Shadertoy / V1-ISF-reject import IPC (`30ca792`, `7216984`).
- **Zero-install macOS Core Audio Process Tap & 64-band FFT + fused BPM** — patched `cpal 0.17.1` Core Audio Process Tap (`f2187da`, `ADR-022`) eliminates BlackHole/virtual-driver installs, paired with `AudioMeterEngine` 64-band FFT and fused BPM tracking (`ca0df53`).
- **Author IDE live preview + auto-repair & Patch visual modulation matrix** — live `naga`/`wgpu` shader preview with auto-repair diagnostics and `.chroma` save in `Author.svelte`, alongside interactive modulation patching and MIDI/playlist stores (`7216984`).
- **Real-time external data ingestion, Scene Rating + Agent Feedback loop & Offline 4K Time-Machine Replay** — `ADR-017` external data-source ingestion, structured scene feedback & agent-digest export, and deterministic offline 4K replay rendering.
- **Phase 7 OBS-killer engine primitives** — native `wgpu` multi-layer compositor (`engine/render/src/compositor.rs`), GPU BGRA→NV12 compute conversion pass (`engine/render/src/nv12_convert.rs`, `ca0df53`), and Twitch EventSub/Chat/Channel Points bridge (`engine/runtime/src/twitch_bridge.rs`).
- **Phase 8 Rekordbox 7 / `DDJ-FLX10` & Beatmatch sync** — multi-channel Master vs. Cue stereo pair extractor with Pioneer/AlphaTheta profiles (`engine/runtime/src/rekordbox_bridge.rs`, `10769c0`), zero-install Rekordbox 7 `PC MASTER OUT` capture (`f2187da`), and live DJ BPM `AUTO / LOCK / OFF` + `-0.5..+0.5` beatmatch phase scrub wired to Metal GPU uniforms (`ad343cb`, `257ab95`, `10769c0`).

## Revisiting cadence

The sweep is intended to be re-run when:

- Synesthesia ships a major version (v1.17+) that meaningfully changes the documented surface.
- Chromaticity crosses a Phase boundary (Phase-4 EXIT, Phase-5 entry, Phase-5 EXIT).
- AO commissions a follow-on smoke after a multi-wave feature cluster (precedent: 2026-05-18 mega-review).

Per-section docs carry a `_Last refreshed:_` marker at their head — bump it
when revising. Scores can change in either direction over time (a Synesthesia
release may add a feature, advancing Chromaticity from MATCH to PARTIAL; a
Chromaticity wave may close a GAP). The closure log above tracks completed
gap-closure work between full re-sweeps.
