// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the Import view authoring bridge:
// Shadertoy GLSL / URL import (`import_shadertoy_source`) and V1 ISF
// deferral diagnostics (`check_isf_import`). Provides deterministic
// browser-mode fallbacks when `isTauri()` is false so Vitest and pure
// Vite preview stay functional end-to-end.

import { invoke, isTauri } from '@tauri-apps/api/core';

export interface ShadertoyImportResult {
  readonly ok: boolean;
  readonly title: string;
  readonly sceneId: string;
  readonly remappedUniforms: readonly string[];
  readonly warnings: readonly string[];
  readonly source: 'tauri' | 'browser-fallback';
}

export interface IsfImportDiagnostic {
  readonly accepted: false;
  readonly code: 'ISF_V2_DEFERRED';
  readonly message: string;
  readonly escapeHatch: string;
  readonly source: 'tauri' | 'browser-fallback';
}

const SHADERTOY_UNIFORMS = [
  'iTime',
  'iResolution',
  'iMouse',
  'iFrame',
  'iTimeDelta',
  'iChannel0',
  'iChannel1',
  'iChannel2',
  'iChannel3',
] as const;

function deriveShadertoyTitle(input: string, explicitTitle?: string): string {
  const custom = explicitTitle?.trim();
  if (custom) return custom;
  const urlMatch = /shadertoy\.com\/view\/([A-Za-z0-9_-]+)/i.exec(input);
  if (urlMatch?.[1]) return urlMatch[1];
  return 'Shadertoy Scene';
}

function slugify(value: string): string {
  const slug = value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return slug || 'shadertoy-scene';
}

/**
 * Import a Shadertoy GLSL source or Shadertoy URL via the Tauri
 * `import_shadertoy_source` command, falling back to a deterministic
 * browser-mode analyzer outside Tauri.
 */
export async function importShadertoySource(
  glsl: string,
  title?: string,
): Promise<ShadertoyImportResult> {
  const trimmed = glsl.trim();
  if (isTauri()) {
    return await invoke<ShadertoyImportResult>('import_shadertoy_source', {
      glsl: trimmed,
      ...(title !== undefined ? { title } : {}),
    });
  }

  const resolvedTitle = deriveShadertoyTitle(trimmed, title);
  const sceneId = `shadertoy-${slugify(resolvedTitle)}`;
  if (!trimmed) {
    return {
      ok: false,
      title: resolvedTitle,
      sceneId,
      remappedUniforms: [],
      warnings: ['No Shadertoy URL or GLSL source provided.'],
      source: 'browser-fallback',
    };
  }

  const remappedUniforms = SHADERTOY_UNIFORMS.filter((u) => trimmed.includes(u));
  const warnings: string[] = [];
  if (remappedUniforms.includes('iMouse')) {
    warnings.push('iMouse remap — no pointer bound. Using default.');
  }
  if (remappedUniforms.some((u) => u.startsWith('iChannel'))) {
    warnings.push('iChannel slot detected — verify texture binding in scene.json.');
  }

  return {
    ok: true,
    title: resolvedTitle,
    sceneId,
    remappedUniforms,
    warnings,
    source: 'browser-fallback',
  };
}

/**
 * Evaluate an Interactive Shader Format (ISF) source via the Tauri
 * `check_isf_import` command, returning the structured V1 deferral
 * diagnostic with a deterministic browser-mode fallback outside Tauri.
 */
export async function checkIsfImport(source: string): Promise<IsfImportDiagnostic> {
  const trimmed = source.trim();
  if (isTauri()) {
    return await invoke<IsfImportDiagnostic>('check_isf_import', { source: trimmed });
  }

  const hasJsonHeader = trimmed.includes('/*{') && trimmed.includes('}*/');
  return {
    accepted: false,
    code: 'ISF_V2_DEFERRED',
    message: hasJsonHeader
      ? 'ISF header detected — Interactive Shader Format (ISF) import is deferred to V2.'
      : 'Interactive Shader Format (ISF) import is deferred to V2.',
    escapeHatch: 'Use the blank .chroma escape hatch to preserve the raw shader body.',
    source: 'browser-fallback',
  };
}
