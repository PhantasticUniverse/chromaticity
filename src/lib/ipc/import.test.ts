// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { beforeEach, describe, expect, it, vi } from 'vitest';

const { invokeMock, isTauriMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  isTauriMock: vi.fn(() => false),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  isTauri: isTauriMock,
}));

import { checkIsfImport, importShadertoySource } from './import';

describe('import IPC bridge', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(false);
  });

  it('uses deterministic browser fallback for Shadertoy source when outside Tauri', async () => {
    const result = await importShadertoySource(
      'void mainImage(out vec4 fragColor, in vec2 fragCoord) { vec2 uv = fragCoord / iResolution.xy; float t = iTime; vec4 m = iMouse; }',
      'Plasma Wave',
    );
    expect(invokeMock).not.toHaveBeenCalled();
    expect(result.ok).toBe(true);
    expect(result.source).toBe('browser-fallback');
    expect(result.title).toBe('Plasma Wave');
    expect(result.remappedUniforms).toEqual(['iTime', 'iResolution', 'iMouse']);
    expect(result.warnings).toContain('iMouse remap — no pointer bound. Using default.');
  });

  it('invokes Tauri import_shadertoy_source when running inside Tauri', async () => {
    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValueOnce({
      ok: true,
      title: 'Tauri Scene',
      sceneId: 'shadertoy-tauri',
      remappedUniforms: ['iTime'],
      warnings: [],
      source: 'tauri',
    });

    const result = await importShadertoySource('void mainImage() {}', 'Tauri Scene');
    expect(invokeMock).toHaveBeenCalledWith('import_shadertoy_source', {
      glsl: 'void mainImage() {}',
      title: 'Tauri Scene',
    });
    expect(result.source).toBe('tauri');
  });

  it('returns structured ISF V2 deferral diagnostic in browser fallback and invokes check_isf_import in Tauri', async () => {
    const fallback = await checkIsfImport('/*{"CREDIT":"demo"}*/ void main() {}');
    expect(fallback.accepted).toBe(false);
    expect(fallback.code).toBe('ISF_V2_DEFERRED');
    expect(fallback.source).toBe('browser-fallback');

    isTauriMock.mockReturnValue(true);
    invokeMock.mockResolvedValueOnce({
      accepted: false,
      code: 'ISF_V2_DEFERRED',
      message: 'Backend V1 ISF rejection',
      escapeHatch: 'Blank scene',
      source: 'tauri',
    });
    const tauriDiag = await checkIsfImport('void main() {}');
    expect(invokeMock).toHaveBeenCalledWith('check_isf_import', { source: 'void main() {}' });
    expect(tauriDiag.source).toBe('tauri');
  });
});
