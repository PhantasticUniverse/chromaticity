// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Xavier Butler <xjbutler@gmail.com>
//
// W31 Patch view tests — exercise the click-to-select / inspector-
// livedata-binding contract, the toolbar Undo button availability,
// and the static fixture render shape. Drag geometry is not covered
// here; jsdom's `getBoundingClientRect()` returns zero-sized rects so
// pointer-coordinate-driven drag math can't be reliably simulated.
// Cross-platform Playwright drag tests can be added in a follow-up
// wave if needed; the in-component pointer-drag $effect itself is
// pattern-shared with `CueTransitionCurve.svelte` which already has
// its own drag harness.
//
// `afterEach(cleanup)` is required (per the
// `feedback_vitest_svelte_cleanup_required.md` user memory) because
// `@testing-library/svelte`'s `render` does not auto-cleanup between
// tests in this Vitest 4 + jsdom configuration; without it
// multi-render tests collide on global queries.

import '@testing-library/jest-dom/vitest';
import { within } from '@testing-library/dom';
import { cleanup, fireEvent, render } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Patch from './Patch.svelte';
import { PATCH_NODES, PATCH_WIRES, patchPinPosFrom } from '../data/patch-fixtures';
import { patchModulationStore } from '../modulation/store.svelte';

describe('Patch view', () => {
  afterEach(() => {
    patchModulationStore.stop();
    patchModulationStore.resetDefaults();
    cleanup();
  });

  it('renders all six nodes from the fixture', () => {
    const { getByRole } = render(Patch);
    expect(getByRole('button', { name: /Audio Input — 5 pins/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Scene: Glitch — 6 pins/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Color Processor — 4 pins/ })).toBeInTheDocument();
    // The "— N pins" suffix disambiguates nodes from the same-prefix palette
    // entries (palette has standalone "LFO" / "MIDI" / "Audio Input" items).
    expect(getByRole('button', { name: /LFO · 1\/4 — 1 pins/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /MIDI · Launchpad X — 3 pins/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Stage Out — 3 pins/ })).toBeInTheDocument();
  });

  it('renders all six wires as signal-colored SVG paths terminating dead-center on solid sockets (no arrowheads)', () => {
    const { container } = render(Patch);
    const paths = container.querySelectorAll('.patch__wires path.patch__wire-path');
    expect(paths.length).toBe(6);

    // Signal-type color mapping: audio (#38bdf8), mod (#a78bfa), midi (#a3e635), video (#fbbf24).
    const strokes = Array.from(paths).map((p) => p.getAttribute('stroke'));
    expect(strokes).toEqual(['#38bdf8', '#38bdf8', '#a78bfa', '#a3e635', '#fbbf24', '#fbbf24']);

    // No SVG arrowhead markers on cables or in <defs>.
    expect(container.querySelectorAll('.patch__wires defs marker').length).toBe(0);
    for (const path of paths) {
      expect(path.getAttribute('marker-end')).toBeNull();
    }

    // Every cable terminates at the exact pin coordinates with matching solid r="4.5" endpoint circles.
    const wires = container.querySelectorAll('.patch__wire');
    PATCH_WIRES.forEach((w, idx) => {
      const [x1, y1] = patchPinPosFrom(PATCH_NODES, w.from[0], w.from[1], 'out');
      const [x2, y2] = patchPinPosFrom(PATCH_NODES, w.to[0], w.to[1], 'in');
      const wireEl = wires[idx] as SVGGElement;
      const pathD = wireEl.querySelector('path.patch__wire-path')?.getAttribute('d') ?? '';
      expect(pathD.startsWith(`M${x1},${y1} `)).toBe(true);
      expect(pathD.endsWith(` ${x2},${y2}`)).toBe(true);

      const startCircle = wireEl.querySelector('circle[data-endpoint="start"]');
      const endCircle = wireEl.querySelector('circle[data-endpoint="end"]');
      expect(startCircle?.getAttribute('cx')).toBe(String(x1));
      expect(startCircle?.getAttribute('cy')).toBe(String(y1));
      expect(startCircle?.getAttribute('r')).toBe('4.5');
      expect(startCircle?.getAttribute('fill')).toBe(strokes[idx]);
      expect(endCircle?.getAttribute('cx')).toBe(String(x2));
      expect(endCircle?.getAttribute('cy')).toBe(String(y2));
      expect(endCircle?.getAttribute('r')).toBe('4.5');
      expect(endCircle?.getAttribute('fill')).toBe(strokes[idx]);
    });
  });

  it('renders connected port sockets as solid signal-colored dots and leaves unconnected ports hollow', () => {
    const { container } = render(Patch);
    const connectedPorts = container.querySelectorAll('.patch__pin-port--connected');
    // 6 wires × 2 endpoints = 12 connected port sockets.
    expect(connectedPorts.length).toBe(12);

    const audioOut = container.querySelector('[data-port-key="audio:bass:out"]') as HTMLElement;
    const sceneIn = container.querySelector('[data-port-key="scene:pusher:in"]') as HTMLElement;
    const lfoOut = container.querySelector('[data-port-key="lfo:out:out"]') as HTMLElement;
    const midiOut = container.querySelector('[data-port-key="midi:cc 14:out"]') as HTMLElement;
    const stageIn = container.querySelector('[data-port-key="out:display 2:in"]') as HTMLElement;
    const unconnected = container.querySelector('[data-port-key="audio:high:out"]') as HTMLElement;

    expect(audioOut).toHaveClass('patch__pin-port--connected');
    expect(audioOut.style.getPropertyValue('--port-color')).toBe('#38bdf8');
    expect(sceneIn).toHaveClass('patch__pin-port--connected');
    expect(sceneIn.style.getPropertyValue('--port-color')).toBe('#38bdf8');
    expect(lfoOut.style.getPropertyValue('--port-color')).toBe('#a78bfa');
    expect(midiOut.style.getPropertyValue('--port-color')).toBe('#a3e635');
    expect(stageIn.style.getPropertyValue('--port-color')).toBe('#fbbf24');

    expect(unconnected).not.toHaveClass('patch__pin-port--connected');
    expect(unconnected.getAttribute('data-connected')).toBe('false');
    expect(unconnected.style.getPropertyValue('--port-color')).toBe('');
  });

  it('activates the TouchDesigner-style signal-flow overlay stroke when live modulation is running', async () => {
    const { container, getByRole } = render(Patch);
    const flows = container.querySelectorAll('path.patch__wire-flow');
    expect(flows.length).toBe(6);
    for (const flow of flows) {
      expect(flow).not.toHaveClass('patch__wire-flow--active');
      expect(flow.getAttribute('stroke-dasharray')).toBe('6 14');
    }

    await fireEvent.click(getByRole('button', { name: /Start Live Mod/ }));
    for (const flow of flows) {
      expect(flow).toHaveClass('patch__wire-flow--active');
    }

    await fireEvent.click(getByRole('button', { name: /Stop Live Mod/ }));
    for (const flow of flows) {
      expect(flow).not.toHaveClass('patch__wire-flow--active');
    }
  });

  it('renders the signal legend pill strip in the canvas header', () => {
    const { container } = render(Patch);
    const legend = container.querySelector('.patch__legend');
    expect(legend).not.toBeNull();
    if (legend === null) return;
    const inLegend = within(legend as HTMLElement);
    expect(inLegend.getByText('Audio')).toBeInTheDocument();
    expect(inLegend.getByText('Mod')).toBeInTheDocument();
    expect(inLegend.getByText('MIDI/OSC')).toBeInTheDocument();
    expect(inLegend.getByText('Video')).toBeInTheDocument();
  });

  it('hovering a wire highlights it, dims unrelated wires, and shows the connection readout', async () => {
    const { container } = render(Patch);
    const wires = container.querySelectorAll('.patch__wire');
    expect(wires.length).toBe(6);
    expect(container.querySelector('.patch__wire-tooltip')).toBeNull();

    const firstWire = wires[0] as SVGGElement;
    await fireEvent.pointerEnter(firstWire);

    expect(firstWire).toHaveClass('patch__wire--hovered');
    expect(wires[1]).toHaveClass('patch__wire--dimmed');
    const tooltip = container.querySelector('.patch__wire-tooltip');
    expect(tooltip).not.toBeNull();
    expect(tooltip?.textContent).toContain('audio.bass → scene.pusher');

    await fireEvent.pointerLeave(firstWire);
    expect(firstWire).not.toHaveClass('patch__wire--hovered');
    expect(wires[1]).not.toHaveClass('patch__wire--dimmed');
    expect(container.querySelector('.patch__wire-tooltip')).toBeNull();
  });

  it('inspector defaults to the bundle Color Processor frame', () => {
    const { container, getByText } = render(Patch);
    expect(getByText('Inspector · Color Processor')).toBeInTheDocument();
    // Pin names ("hue" / "saturation" / etc.) appear both in the Color
    // Processor node body AND its inspector rows — scope queries to the
    // inspector body to disambiguate.
    const inspector = container.querySelector('.patch__inspector-body');
    expect(inspector).not.toBeNull();
    if (inspector === null) return;
    const inInspector = within(inspector as HTMLElement);
    expect(inInspector.getByText('hue')).toBeInTheDocument();
    expect(inInspector.getByText('saturation')).toBeInTheDocument();
    expect(inInspector.getByText('brightness')).toBeInTheDocument();
    expect(inInspector.getByText('contrast')).toBeInTheDocument();
  });

  it('keyboard-activating a node updates inspector to that node', async () => {
    const { container, getByRole, getByText } = render(Patch);
    const audioNode = getByRole('button', { name: /Audio Input — 5 pins/ });
    audioNode.focus();
    await fireEvent.keyDown(audioNode, { key: 'Enter' });
    expect(getByText('Inspector · Audio Input')).toBeInTheDocument();
    // Pin names ("bass" / "rms" / etc.) appear both in the Audio Input node
    // body AND its inspector rows after selection — scope queries to the
    // inspector body to confirm the rows track selection.
    const inspector = container.querySelector('.patch__inspector-body');
    expect(inspector).not.toBeNull();
    if (inspector === null) return;
    const inInspector = within(inspector as HTMLElement);
    expect(inInspector.getByText('bass')).toBeInTheDocument();
    expect(inInspector.getByText('rms')).toBeInTheDocument();
  });

  it('selected node gets aria-pressed=true', async () => {
    const { getByRole } = render(Patch);
    const sceneNode = getByRole('button', { name: /Scene: Glitch — 6 pins/ });
    expect(sceneNode).toHaveAttribute('aria-pressed', 'false');
    sceneNode.focus();
    await fireEvent.keyDown(sceneNode, { key: ' ' });
    expect(sceneNode).toHaveAttribute('aria-pressed', 'true');
  });

  it('toolbar Undo button is disabled before any drag', () => {
    const { getByRole } = render(Patch);
    const undoBtn = getByRole('button', { name: /Undo last node move/ });
    expect(undoBtn).toBeDisabled();
  });

  it('Auto Layout and Commit toolbar buttons render but stay stub-only', () => {
    const { getByRole } = render(Patch);
    expect(getByRole('button', { name: /Auto Layout/ })).toBeInTheDocument();
    expect(getByRole('button', { name: /Commit/ })).toBeInTheDocument();
  });

  it('palette renders all five categories', () => {
    const { getByText } = render(Patch);
    expect(getByText('Sources')).toBeInTheDocument();
    expect(getByText('Modulators')).toBeInTheDocument();
    expect(getByText('Scenes')).toBeInTheDocument();
    expect(getByText('Processors')).toBeInTheDocument();
    expect(getByText('Outputs')).toBeInTheDocument();
  });

  it('inspector unroute clears the source and shows "no source"', async () => {
    const { getByRole, getAllByText, queryByText } = render(Patch);
    // Default frame is Color Processor; "hue" row has a MIDI source.
    // brightness + contrast already render "no source" → count starts at 2.
    expect(queryByText('MIDI · CC 14')).toBeInTheDocument();
    expect(getAllByText('no source').length).toBe(2);
    const unrouteBtn = getByRole('button', { name: /Unroute hue/ });
    await fireEvent.click(unrouteBtn);
    expect(queryByText('MIDI · CC 14')).toBeNull();
    expect(getAllByText('no source').length).toBe(3);
  });

  it('live modulation bar toggles running state and route enable flags', async () => {
    const { getByRole, getByText } = render(Patch);
    const modBtn = getByRole('button', { name: /Start Live Mod/ });
    expect(modBtn).toHaveAttribute('aria-pressed', 'false');
    expect(getByText('3/3 active')).toBeInTheDocument();

    await fireEvent.click(modBtn);
    expect(getByRole('button', { name: /Stop Live Mod/ })).toHaveAttribute('aria-pressed', 'true');

    const bassRouteBtn = getByRole('button', { name: /Toggle route audio\.bass to ctrl_0/ });
    expect(bassRouteBtn).toHaveAttribute('aria-pressed', 'true');
    await fireEvent.click(bassRouteBtn);
    expect(bassRouteBtn).toHaveAttribute('aria-pressed', 'false');
    expect(getByText('2/3 active')).toBeInTheDocument();

    await fireEvent.click(getByRole('button', { name: /Stop Live Mod/ }));
    expect(getByRole('button', { name: /Start Live Mod/ })).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });

  it('mounts SVG wires and DOM nodes inside a shared 1:1 stage coordinate space', () => {
    const { container } = render(Patch);
    const stage = container.querySelector('.patch__stage') as HTMLElement | null;
    expect(stage).not.toBeNull();
    const svg = stage?.querySelector('svg.patch__wires');
    const nodesLayer = stage?.querySelector('.patch__nodes');
    expect(svg).not.toBeNull();
    expect(nodesLayer).not.toBeNull();
    // Explicit 1:1 pixel dimensions matching viewBox eliminate preserveAspectRatio drift.
    expect(svg?.getAttribute('width')).toBe('1100');
    expect(svg?.getAttribute('height')).toBe('600');
    expect(svg?.getAttribute('viewBox')).toBe('0 0 1100 600');
    expect(svg?.getAttribute('preserveAspectRatio')).toBeNull();
    expect(stage?.style.width).toBe('1100px');
    expect(stage?.style.height).toBe('600px');
  });

  it('keeps user-facing tooltips free of internal phase or wave jargon', () => {
    const { container } = render(Patch);
    const titled = container.querySelectorAll('[title]');
    for (const el of titled) {
      const title = el.getAttribute('title') ?? '';
      expect(title).not.toMatch(/Phase-\d|W\d+|EV-|ADR-/);
    }
  });
});
