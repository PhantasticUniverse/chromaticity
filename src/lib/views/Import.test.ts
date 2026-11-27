// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// W54 Import view tests — exercise the W19 PR2 full port: five
// converter cards (Shadertoy / .chroma / .chromaPack / .synScene / ISF
// with the V2-deferred disabled flag), per-card local input round-trip,
// the recent-imports list, and the frozen console transcript. The view
// does not call Tauri `invoke` and does not read shared stores, so no
// IPC mock or store reset is needed; per-card `$state` inputs are
// covered through the input-round-trip test.
//
// `afterEach(cleanup)` per the user-memory
// `feedback_vitest_svelte_cleanup_required.md` convention.

import '@testing-library/jest-dom/vitest';
import { cleanup, fireEvent, render, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import Import from './Import.svelte';
import {
  CONVERTER_HINT,
  IMPORT_CONSOLE,
  IMPORT_CONVERTERS,
  PREVIEW_LABEL,
  RECENT_IMPORTS,
} from '../data/import-fixtures';

describe('Import view', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the canonical headline + converter hint', () => {
    const { getByText } = render(Import);
    expect(getByText('Import · Scene Converters')).toBeInTheDocument();
    expect(getByText(CONVERTER_HINT)).toBeInTheDocument();
  });

  it('renders all five converter cards from the fixture', () => {
    const { getByRole } = render(Import);
    for (const converter of IMPORT_CONVERTERS) {
      expect(getByRole('article', { name: `${converter.title} converter` })).toBeInTheDocument();
    }
  });

  it('ISF row is V2-deferred — input + Preview + Import buttons all disabled', () => {
    const { getByRole } = render(Import);
    const isfInput = getByRole('textbox', { name: 'Interactive Shader Format (ISF) source' });
    expect(isfInput).toBeDisabled();
    const isfCard = getByRole('article', { name: /Interactive Shader Format \(ISF\) converter/ });
    const buttons = isfCard.querySelectorAll('button');
    // Browse + Preview + Import → three buttons per card; all disabled on
    // the V2-deferred ISF row (Browse is disabled across the board pending
    // the file picker, but the ISF Preview/Import disabled state is the
    // load-bearing assertion here).
    expect(buttons.length).toBe(3);
    for (const button of buttons) {
      expect(button).toBeDisabled();
    }
  });

  it('non-disabled converter cards keep Preview + Import enabled (Browse stays disabled)', () => {
    const { getByRole } = render(Import);
    const shadertoyCard = getByRole('article', { name: /Shadertoy\.com converter/ });
    const previewBtn = shadertoyCard.querySelector(
      'button.import__btn:not(.import__btn--ghost):not(.import__btn--primary)',
    );
    const importBtn = shadertoyCard.querySelector('button.import__btn--primary');
    const browseBtn = shadertoyCard.querySelector('button.import__btn--ghost');
    expect(previewBtn).not.toBeNull();
    expect(importBtn).not.toBeNull();
    expect(browseBtn).not.toBeNull();
    expect(previewBtn).not.toBeDisabled();
    expect(importBtn).not.toBeDisabled();
    // Browse is disabled across all rows pending the post-Phase-1 file
    // picker — covered here so a future file-picker landing has to
    // explicitly update this assertion.
    expect(browseBtn).toBeDisabled();
  });

  it('per-card input round-trips local $state without bleeding across rows', async () => {
    const { getByRole } = render(Import);
    const shadertoyInput = getByRole('textbox', {
      name: 'Shadertoy.com source',
    }) as HTMLInputElement;
    const chromaInput = getByRole('textbox', {
      name: 'Chromaticity Scene (.chroma) source',
    }) as HTMLInputElement;
    expect(shadertoyInput.value).toBe('');
    expect(chromaInput.value).toBe('');
    await fireEvent.input(shadertoyInput, {
      target: { value: 'https://www.shadertoy.com/view/WdVfRh' },
    });
    expect(shadertoyInput.value).toBe('https://www.shadertoy.com/view/WdVfRh');
    // Other cards' inputs remain empty — per-card state is keyed on the
    // converter id, not shared.
    expect(chromaInput.value).toBe('');
  });

  it('Conversion Preview tile renders with the canonical eyebrow label', () => {
    const { getByLabelText, getAllByText } = render(Import);
    expect(getByLabelText(PREVIEW_LABEL)).toBeInTheDocument();
    // The eyebrow renders the label text — disambiguate via getAllByText
    // since aria-label query above already matched the article.
    const matches = getAllByText(PREVIEW_LABEL);
    expect(matches.length).toBeGreaterThanOrEqual(1);
  });

  it('renders the Recent Imports panel with all six fixture rows', () => {
    const { getByText, getByRole } = render(Import);
    expect(getByRole('heading', { name: 'Recent Imports' })).toBeInTheDocument();
    for (const row of RECENT_IMPORTS) {
      expect(getByText(row.title)).toBeInTheDocument();
      expect(getByText(row.activity)).toBeInTheDocument();
    }
  });

  it('Recent Imports edit buttons are disabled (post-Phase-1 stub)', () => {
    const { getByLabelText } = render(Import);
    for (const row of RECENT_IMPORTS) {
      const editBtn = getByLabelText(`Edit ${row.title}`);
      expect(editBtn).toBeDisabled();
    }
  });

  it('console transcript renders all six entries with level data attributes', () => {
    const { container, getByText } = render(Import);
    for (const entry of IMPORT_CONSOLE) {
      expect(getByText(entry.message)).toBeInTheDocument();
    }
    const lines = container.querySelectorAll('.import__console-line');
    expect(lines.length).toBe(IMPORT_CONSOLE.length);
    // Spot-check that the level attribute is wired through — the W19 PR2
    // colour-coding contract depends on it.
    const tags = container.querySelectorAll('.import__console-tag');
    expect(tags.length).toBe(IMPORT_CONSOLE.length);
    expect(tags[0]?.getAttribute('data-level')).toBe(IMPORT_CONSOLE[0]?.level);
    // The warn entry is the last fixture row.
    const lastTag = tags[IMPORT_CONSOLE.length - 1];
    expect(lastTag?.getAttribute('data-level')).toBe('warn');
  });

  it('console clear button renders but is a visual-only stub (disabled)', () => {
    const { getByRole } = render(Import);
    const clearBtn = getByRole('button', { name: 'clear' });
    expect(clearBtn).toBeDisabled();
  });

  it('Shadertoy import invokes the IPC bridge, logs console output, and prepends a recent import', async () => {
    const { container, getByRole, getByText } = render(Import);
    const shadertoyInput = getByRole('textbox', {
      name: 'Shadertoy.com source',
    }) as HTMLInputElement;
    await fireEvent.input(shadertoyInput, {
      target: { value: 'https://www.shadertoy.com/view/Xds3zN' },
    });

    const shadertoyCard = getByRole('article', { name: /Shadertoy\.com converter/ });
    const importBtn = shadertoyCard.querySelector(
      'button.import__btn--primary',
    ) as HTMLButtonElement;
    await fireEvent.click(importBtn);

    await waitFor(() => {
      expect(getByText('Xds3zN')).toBeInTheDocument();
    });
    expect(container.querySelectorAll('.import__console-line').length).toBeGreaterThan(
      IMPORT_CONSOLE.length,
    );
    expect(getByRole('button', { name: 'clear' })).not.toBeDisabled();
  });

  it('dropping an ISF file on the ISF card logs the V1 ISF_V2_DEFERRED diagnostic', async () => {
    const { getByRole, getByText } = render(Import);
    const isfCard = getByRole('article', { name: /Interactive Shader Format \(ISF\) converter/ });
    await fireEvent.drop(isfCard, {
      dataTransfer: {
        getData: () => '/*{"DESCRIPTION":"test"}*/ void main() {}',
      },
    });

    await waitFor(() => {
      expect(getByText(/ISF_V2_DEFERRED/)).toBeInTheDocument();
    });
  });

  it('displays amber Non-Commercial (-NC) warning badge and attribution notice for CC BY-NC Shadertoy GLSL', async () => {
    const { getByRole, getByTestId } = render(Import);
    const shadertoyInput = getByRole('textbox', {
      name: 'Shadertoy.com source',
    }) as HTMLInputElement;
    const ccByNcGlsl = [
      '// SPDX-License-Identifier: CC-BY-NC-4.0',
      '// Author: Alex Rivera',
      'void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(1.0); }',
    ].join('\n');

    await fireEvent.input(shadertoyInput, { target: { value: ccByNcGlsl } });

    const shadertoyCard = getByRole('article', { name: /Shadertoy\.com converter/ });
    const previewBtn = shadertoyCard.querySelector(
      'button.import__btn:not(.import__btn--ghost):not(.import__btn--primary)',
    ) as HTMLButtonElement;
    await fireEvent.click(previewBtn);

    await waitFor(() => {
      const badge = getByTestId('import-license-badge');
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveAttribute('data-commercial-safe', 'false');
      expect(badge).toHaveTextContent('Non-Commercial (-NC)');
      expect(badge).toHaveTextContent('CC-BY-NC');
      expect(badge).toHaveTextContent('Alex Rivera');
      expect(badge).toHaveTextContent(/Commercial use is not permitted/i);
    });
  });

  it('displays green Commercial Safe badge for MIT-licensed Shadertoy GLSL', async () => {
    const { getByRole, getByTestId } = render(Import);
    const shadertoyInput = getByRole('textbox', {
      name: 'Shadertoy.com source',
    }) as HTMLInputElement;
    const mitGlsl = [
      '// The MIT License',
      '// Copyright (c) 2024 Inigo Quilez',
      'void mainImage(out vec4 fragColor, in vec2 fragCoord) { fragColor = vec4(0.0); }',
    ].join('\n');

    await fireEvent.input(shadertoyInput, { target: { value: mitGlsl } });

    const shadertoyCard = getByRole('article', { name: /Shadertoy\.com converter/ });
    const previewBtn = shadertoyCard.querySelector(
      'button.import__btn:not(.import__btn--ghost):not(.import__btn--primary)',
    ) as HTMLButtonElement;
    await fireEvent.click(previewBtn);

    await waitFor(() => {
      const badge = getByTestId('import-license-badge');
      expect(badge).toBeInTheDocument();
      expect(badge).toHaveAttribute('data-commercial-safe', 'true');
      expect(badge).toHaveTextContent('Commercial Safe');
      expect(badge).toHaveTextContent('MIT');
      expect(badge).toHaveTextContent('Inigo Quilez');
      expect(badge).not.toHaveTextContent('Non-Commercial (-NC)');
    });
  });

  it('Synesthesia .synScene import invokes import_syn_scene, logs console output, and prepends a recent import', async () => {
    const { container, getByRole, getByText } = render(Import);
    const synInput = getByRole('textbox', {
      name: 'Synesthesia Scene (.synScene) source',
    }) as HTMLInputElement;
    await fireEvent.input(synInput, {
      target: { value: '/scenes/Neon_Pulse.synScene' },
    });

    const synCard = getByRole('article', { name: /Synesthesia Scene \(\.synScene\) converter/ });
    const importBtn = synCard.querySelector('button.import__btn--primary') as HTMLButtonElement;
    await fireEvent.click(importBtn);

    await waitFor(() => {
      expect(getByText('Neon Pulse')).toBeInTheDocument();
      expect(getByText(/Synesthesia → scenes\/neon-pulse\.chroma/)).toBeInTheDocument();
    });
    expect(container.querySelectorAll('.import__console-line').length).toBeGreaterThan(
      IMPORT_CONSOLE.length,
    );
  });
});
