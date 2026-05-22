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

## Revisiting cadence

The sweep is intended to be re-run when:

- Synesthesia ships a major version (v1.17+) that meaningfully changes the documented surface.
- Chromaticity crosses a Phase boundary (Phase-4 EXIT, Phase-5 entry, Phase-5 EXIT).
- AO commissions a follow-on smoke after a multi-wave feature cluster (precedent: 2026-05-18 mega-review).

Per-section docs carry a `_Last refreshed:_` marker at their head — bump it
when revising. Scores can change in either direction over time (a Synesthesia
release may add a feature, advancing Chromaticity from MATCH to PARTIAL; a
Chromaticity wave may close a GAP).
