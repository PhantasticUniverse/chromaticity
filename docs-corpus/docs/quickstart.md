<!--
SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Quickstart — first-launch recipe

This page walks a brand-new operator from "I just installed Chromaticity"
to "I have an audio-reactive scene running and I'm dragging a control to
confirm it reacts." Allow ten to fifteen minutes end-to-end.

The recipe is honest about which surfaces are live in production today
and which are scheduled follow-ups. Where the engine is still synthetic
or partially wired, the page calls that out plainly rather than
pretending the final surface is in place. The companion
[Pre-Alpha Smoke Checklist][smoke] in the release-eng tree carries the
exhaustive verification matrix; this page is the operator-facing
narrative.

## 1. First launch — the Welcome view

When Chromaticity opens for the first time it routes to **Welcome**.
You see a centred hero with three quick-start cards:

- **Quick Start** — routes to the Library view so you can pick a
  bundled scene.
- **Import Setlist** — disabled today. The Import view ports in a
  later wave; the card is reserved for that surface.
- **Empty Project** — routes to the Perform view with a blank scene
  loaded.

Underneath the cards is a hint row: "Audio input required · MIDI
optional · Ableton Link supported." The first two are accurate today;
the Link tagline is reserved for future work.

The sidebar rail along the left edge surfaces the rest of the app —
Perform, Library, Author, Playlist, Media, MIDI, OSC, Audio, Patch,
Onboard, Settings, Help. Each rail icon shows a tooltip on hover. The
status bar across the bottom carries the active tier badge, adapter
state pills, and the theme name.

!!! note "Bundled scenes ship with the app"

    The Library's built-in corpus — sixteen clean-room `.chroma`
    packages across the `welcome/`, `examples/`, and `library/`
    subdirectories — ships inside the app package via the Tauri
    `bundle.resources` map and is auto-discovered on launch. You do
    not need to install or register anything to see it. To add your
    own scenes alongside the bundled set, point Settings → Storage
    at your scene folder.

## 2. Audio setup

Chromaticity is an audio-reactive runtime. The audio path is live in
production via the Wave 1c-α-2 cross-seam wiring: the producer thread
in `engine/audio` publishes a per-tick analysis snapshot; the scheduler
emits the snapshot over IPC to the UI at sixty hertz; the four named
band tiles and BPM strip in the AudioReactivity widget read the live
snapshot end-to-end.

### Pick the input device

Open **Settings → Audio Device**. The Input Device dropdown enumerates
host audio devices through the audio adapter's input picker. Pick the
device you want the analysis pipeline to read from. The picker
persists across restarts through the TOML-backed Settings store
under Tauri's `app_data_dir`.

The dropdown's selection takes effect at the next app launch. The
analysis thread binds its capture device once at startup and does
not hot-swap devices mid-show, keeping the snapshot pipeline
deterministic.

### macOS — capturing system audio

System-output capture is built in. Open **Settings → Audio Device**
and pick **System Audio (built-in)** in the Input Device dropdown.
The engine taps the system output directly through Core Audio
process taps — macOS 14.6 and later, with a one-time
audio-recording permission prompt on first use. No virtual-loopback
driver to install: whatever your computer is playing feeds the
analysis pipeline.

On macOS older than 14.6, or if the audio-recording permission is
denied, the engine degrades to the default input device (the
microphone) rather than going silent. From there the operator has
three paths:

**Easy path — acoustic mic pickup.** Play music on your computer
speakers and let the built-in microphone hear it. Clap or speak
near the mic to confirm the named band tiles respond.

**Alternative path — virtual loopback.** To capture on older macOS,
or to route one specific app's output, install a virtual audio
device that routes system output to a virtual input:

```bash
# BlackHole (free, MIT-licensed) — install with Homebrew:
brew install blackhole-2ch

# or grab the installer from existential.audio/blackhole/
```

After installing BlackHole, configure macOS to send system audio to
it (Audio MIDI Setup → create a Multi-Output Device that includes
your speakers and BlackHole, then route system output there).
Restart Chromaticity, open Settings → Audio Device, and pick
**BlackHole 2ch** in the Input Device dropdown. Loopback (paid, more
flexible) is the GUI-driven alternative.

**No-mic path — silent fallback.** If the machine has no input
device at all, the producer falls back to a silent source and emits
all-zero snapshots. The visualizer widgets stay on synthetic fallback;
that is expected behaviour, not a bug.

### Windows — capturing system audio

Windows operators use **VB-Cable** (free) or VoiceMeeter (free,
heavier GUI). Install VB-Cable, set your default playback device to
"CABLE Input", set Chromaticity's input device in Settings to
"CABLE Output", and audio routed to system playback is captured by
the analysis pipeline.

### Confirm the device is producing signal

Open the **Audio** view. The live monitoring meter at the top of the
view shows the input level in real time; if you see motion when
audio is playing, the device is producing signal and the analysis
pipeline is reading it.

The first ~1.2 seconds after the audio pipeline starts is a
documented **warming window** — the BpmRing, BeatDots, and band tiles
render in a dimmed placeholder state while the FFT and BPM
accumulators fill. Once the analysis window is full, the widgets
transition to live values. The new top-bar **AUDIO** pill reflects
the transition: yellow during warming, green when ready, red on a
producer-thread fault.

## 3. Load your first scene

Open the **Library** view from the sidebar (or click Quick Start on
the Welcome card). The library merges three sources: bundled scenes
shipped inside the app bundle, user scenes under your registered
storage folders, and the active Author workspace. See
[Library Workflow](library-workflow.md) for the full source taxonomy.

Scenes are arranged as a card grid. Each card shows a thumbnail
(canvas-2d procedural — see the honesty note below), a title, a
source pill, and a few tag chips. Single-click any card to surface its
detail in the right-pane sidebar with a `<SceneCanvas>` preview, the
description, credit, and the controls-count badge. Double-click —
or hit Enter on a focused card — to load the scene into the runtime.

The four bundled welcome scenes you'll see first:

- **welcome.chroma** — gradient-drift backdrop with three controls
  (`intro_scale`, `drift_speed`, `accent`). The first-launch onboarding
  scene per the scene library spec.
- **quick-start.chroma** — minimal four-control scene combining a
  brightness slider, a contrast knob, a saturation slider, and a tint
  color. Sweep every primary widget kind in one place.
- **controls-tour.chroma** — comprehensive widget showcase exercising
  every input kind the Inspector can render (slider, knob, toggle,
  color, xy, dropdown).
- **first-cue.chroma** — single-cue starter scene illustrating a
  fade-driven transition from calm baseline to emphasis. Useful when
  you want to feel how cues drive control values.

After loading, the app routes to **Perform**. The Scene Controls
header binds to the loaded scene's NAME and the Scene Controls plus
Meta Controls panels render against the loaded control schema.

!!! warning "Preview is canvas-2d procedural, not live GPU render"

    The Perform preview pane and every `<SceneCanvas>` instance render
    through a canvas-2d procedural adapter — sixteen built-in scene
    "kinds" mapped from each scene's metadata (kaleido, rings, plasma,
    orbit, tunnel, fluid, lattice, bars, and friends). The live GPU
    render pipeline (ADR-011) is live in the engine's dedicated
    output path, but the in-app preview pane has not yet been moved
    onto it. Dragging a control fires the dispatch through to the
    engine and the engine commits the value — that path is live —
    but what you see in the preview is the procedural visualisation,
    not the actual GLSL output. The procedural preview is
    wireframe-faithful for structural feedback (rotation, warp,
    audio reactivity); it is not the final rendered image.

## 4. Drag a control

In Perform, look at the **Scene Controls** panel along the left
column. Each control row is a typed widget — sliders for numeric
parameters, knobs for tighter-range numerics, toggles for booleans,
swatches for colours. Each row carries a per-control hotkey-hint
badge along its trailing edge.

Drag a Fader. Three things happen in order:

1. The widget's local drag position moves under your pointer.
2. The dispatch through `set_control` IPC fires.
3. The engine commits the value through its command bus and returns
   it; the widget snaps to the engine-committed value (which, for
   un-clamped controls, equals what you dragged to).

If the control name maps to one of the renderer's known
pre-multiplier axes — `speed`, `intensity`, `tint`, `rotation_speed`,
`warp_amount`, `invert`, `mirror` — the canvas-2d procedural preview
responds visibly. The Wave 2.8/2.9 SC.A widening added the
`pulse(lvl) = 0.6 + lvl * 0.4` coupling envelope so the procedural
visuals couple to the live audio level (sixty percent intensity at
silent audio, full intensity at peak).

Controls whose names are NOT in the renderer's known set still fire
the engine dispatch correctly but produce no visible change in the
preview. Those controls render with a **`data-preview-wired="false"`
affordance** — the row dims to seventy percent opacity and a hover
tooltip explains: "engine-only — preview adapter not implemented."
That is the honest signal that the engine sees the value but the
canvas-2d preview adapter has no visualisation for that name. The
live GPU render pipeline (ADR-011) is where those controls land
their visible effect; the preview-pane port onto that pipeline is
still to come.

### Quick action buttons

Above each control panel sit three action buttons that operate
on the whole panel at once:

- **Default** — resets every control in the panel to its declared
  default.
- **Random** — randomises every control to a value in its declared
  range. Locked rows are skipped.
- **Lock-all** — toggles a per-row lock on every control in the
  panel. Locked rows dim and short-circuit the dispatch so the
  engine never sees suppressed values.

The hotkey vocabulary also reserves per-row keys — **`L`** toggles
the focused row's lock, **`D`** sets the focused control to default,
**`R`** randomises it, and **`P`** recalls its value from the active
preset — but the row-focused key listener is not wired in this
build; use the panel action buttons above. See
[Hotkey Reference](hotkeys-reference.md) for the full vocabulary.

## 5. Try Perform mode

You're already in Perform after loading the scene. A few things
to exercise.

### STANDARD versus CONSOLE variant

The top bar carries a **STANDARD / CONSOLE** toggle. STANDARD is the
classic two-column layout — Scene Controls and Meta Controls on the
left, Preview hero and Audio Reactivity panel in the middle, scene
rail on the right. CONSOLE collapses to a denser three-column layout
intended for operators who want more density at the cost of breathing
room. The choice persists per launch.

### Snap recall

The **Snap bar** runs across the bottom of Perform with eight
quick-recall slots, addressed by mouse or by Tab-focusing a slot and
pressing `Enter` / `Space` (the same `Cmd` / `Alt` modifiers as the
clicks below apply). The digit-key bindings listed in the
[Hotkey Reference](hotkeys-reference.md) (`1`–`9`, `0`, and the
`Cmd+0` / `Ctrl+0` gold preset) are vocabulary only and are not yet
wired to Snap recall.

- **Click slot N** — recalls the saved state.
- **Cmd+click slot N** (macOS) / **Ctrl+click slot N** (Windows) —
  saves the current control state into slot N.
- **Alt+click slot N** — clears the slot.
- **Right-click slot N** — opens an inline rename input for a custom
  label ("Intro" / "Drop" / "Outro"). Labels persist across restarts
  through localStorage.

Try it: drag a Fader, Cmd-click slot 1 to save, drag the Fader
elsewhere, click slot 1 to ping back to the saved state.

### Preset and playlist transport

The right-column scene rail carries a Prev / Next / Shuffle row
that mirrors playlist transport. Click **Next** to advance one slot;
click **Previous** to step back. The preset rail above it lets you
click a preset to recall it; Cmd-click an empty preset slot to save
the current control state to it (in-process snapshot — see the
honesty note).

!!! warning "Shuffle advances by +1, not random"

    The engine does not yet carry a shuffle command. The Perform
    Shuffle button advances by +1 like Next; the label parity with
    the bundle is preserved but the ordering is not randomised.
    A future-wave engine `Command::Shuffle` widening is on the
    follow-up list.

!!! note "Preset save is in-process only"

    Cmd-click preset save captures the current control state into an
    in-memory preset bank. Settings-TOML disk persistence is a
    deferred follow-up; quit and relaunch will not restore the saved
    preset.

## 6. Confirm audio reactivity

Make sure your audio source is playing (music near the mic, or —
with **System Audio (built-in)** selected — whatever your computer
is playing). Watch the top-bar HUD and the Audio Reactivity panel
below the preview hero.

- The **VU meter** in the top bar pulses with the live RMS.
- The **BpmRing** rotates phase-locked to the estimated tempo.
- The **BeatDots** strip flashes on each beat boundary.
- The **Audio Reactivity** panel's four named band tiles
  ("bass" / "low-mid" / "mid" / "high") move in proportion to the
  spectrum energy in each band. The BPM strip beneath them tracks
  the live BPM.

The four named tiles are derived from the engine's eight log-spaced
FFT bands via a documented pairwise average — see
[Audio Reactivity](audio-reactivity.md) for the reduction formula
and the full uniform catalog.

!!! note "Some audio surfaces stay synthetic"

    The forty-eight bar FFT visualizer inside the Audio Reactivity
    panel and the inline sixty-four bar Spectrum surface in the Audio
    Uniforms view both stay synthetic-driven in the V1 release. They
    paint a deterministic visual placeholder, not the live FFT — that
    retrofit is post-MVP UX work. The four named band tiles and the
    BPM strip are the live-driven surfaces today.

### Set the noise-floor threshold

If you're in a noisy room and the band tiles are jittering on HVAC
or crowd murmur, set the noise-floor gate through OSC at
`/audio/threshold` (see [OSC Routes](osc-routes.md)) — values clamp
to `[0, 1]`, and the accepted value is announced back out on the
same route for show-control automation.

!!! note "The `Shift+T` sample-threshold hotkey is not wired yet"

    The hotkey vocabulary reserves **`Shift+T`** for "set audio
    threshold to current RMS level", but the handler is not wired in
    this release — pressing it does nothing today. The backend
    `set_audio_threshold` command is in place awaiting the frontend
    hotkey wire; use the OSC route to set the threshold.

## 7. Where to go next

The rest of the help corpus expands on each surface:

- **[Audio Reactivity](audio-reactivity.md)** — the full uniform
  catalog, the eight-to-four band reduction, and a worked radial-pulse
  shader example.
- **[Scene Workflow](scene-workflow.md)** — the `.chroma` package
  format, the load lifecycle, and the live-reload policy authors rely
  on while iterating.
- **[Library Workflow](library-workflow.md)** — scene sources, the
  four-subdirectory bundled taxonomy, thumbnails, and the indexer.
- **[Perform Mode](perform-mode.md)** — the five panels, the Snap
  bar, the scene rail, and the adapter-pill status surface.
- **[Hotkey Reference](hotkeys-reference.md)** — the canonical
  fifty-seven row hotkey vocabulary.
- **[OSC Routes](osc-routes.md)** — wire-level routing and the
  network-conservative defaults.
- **[Settings Reference](settings-reference.md)** — every setting in
  the Settings view, with the audio / graphics / storage / network /
  entitlement sections enumerated.

For an exhaustive verification checklist of every Wave 2.x
acceptance gate plus the known synthetic surfaces, the release-eng
pre-alpha smoke checklist (in the repository at
`release-eng/pre-alpha-smoke-checklist.md`) is the source of truth.
This quickstart is the operator-facing narrative; the smoke checklist
is the test matrix.

---

<!-- Clean-room authored from src/lib/views/Welcome.svelte (W16 PR3 Welcome view content),
src/lib/views/Settings.svelte (Wave 2.7 Settings layout), src/lib/views/Library.svelte
(Wave 2.5 LIB.A-E wiring), src/lib/views/Perform.svelte (Wave 2.4 PERF.A-F wiring),
engine/scene/tests/fixtures/scenes/welcome/README.md (welcome scene corpus),
src/lib/widgets/SceneCanvas.svelte + scene-renderers.ts (canvas-2d procedural adapter
+ Wave 2.8/2.9 SC.A pre-multiplier),
release-eng/pre-alpha-smoke-checklist.md (verification matrix),
and docs/29-cleanroom-legal-policy-baseline.md (clean-room SOP);
never derived from Contents/ or chromaticity-planning/ per docs/29. -->
