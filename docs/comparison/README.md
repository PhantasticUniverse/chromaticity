<!-- SPDX-License-Identifier: CC-BY-4.0 -->
<!-- SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com> -->

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

### Parity-closure wave 2 — Lane D (2026-05-22, ~15 rows lifted)

- **PR 1** (`#499`): XY 2D interactive control pad (`XyPad.svelte`) — widget + preset capture + IPC bridge (`vec2` type, `set_control_xy` / `get_control_xy` commands). Closes XY-control PARTIAL rows across SSF and Manual sections.
- **PR 2** (`#500`): XY controls survive preset save/recall — `State::apply_event` preserves vec2 control values through `Command::PresetSave` / `EventPayload::PresetSaved`. Closes XY-preset-round-trip rows.
- **PR 3** (`#501`): `BindTarget::Scene` two-arg form — OSC `/scenes/{name} {preset}` inbound route binds to a (scene, preset) tuple so external surfaces can jump to a named scene at a named preset in one message. Closes the compat scene-with-preset bind PARTIAL rows.
- **PR 4** (`#502`): Vec2 control IPC bridge and shell commands — Tauri commands `set_control_xy` / `get_control_xy` + ts-rs `Vec2ControlPayload`. Engine round-trip tested.
- **PR 5** (`#503`): `PlaylistSetPosition` command (Tier 1) — `Command::PlaylistSetPosition(i32)` + `EventPayload::PlaylistPositionChanged` + negative-index counting-from-end semantics. Closes playlist-absolute-position rows.
- **PR 6** (`#504`): Interactive XY 2D control pad UI (closes the UI surface for PRs 1+4).
- **PR 7** (`#505`): OSC outbound sender (`OscSender` + `OscSenderHandle`) with `emit` / `close` interface per ADR-013 §1/§6. Closes OSC-outbound PARTIAL rows.
- **PR 8** (`#506`): `/audio/threshold` OSC route bidirectional — engine emits current threshold outbound on change + adapter bind. Closes the `/audio/threshold` direction PARTIAL rows.
- **PR 9** (`#507`): OSC playlist `position` bind — `/playlists/position {i}` inbound + outbound routes wired through the `PlaylistSetPosition` command. Closes playlist-position OSC rows.
- **PR 10** (`#508`): OSC compat dual-emit window — `OscSender::emit_with_compat_alias`, `dual_emit_enabled` / `dual_emit_until` settings fields, `compat_alias_emits` health counter. Closes compat-emit PARTIAL rows.
- **PR 11** (this PR): OSC view dual-emit controls + `/playlists/position` route fixture + docs-corpus and parity-gap refresh.

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
