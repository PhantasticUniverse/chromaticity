// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity
//
// Typed Tauri IPC wrappers for the Import view authoring bridge:
// Shadertoy GLSL / URL import (`import_shadertoy_source`), Shadertoy
// header license inspection (`inspect_shadertoy_license`), Synesthesia
// `.synScene` conversion (`import_syn_scene`), and V1 ISF deferral
// diagnostics (`check_isf_import`). Provides deterministic browser-mode
// fallbacks when `isTauri()` is false so Vitest and pure Vite preview
// stay functional end-to-end.

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

export interface SynImportResult {
  readonly output_dir: string;
  readonly scene_title: string;
  readonly control_count: number;
  readonly has_script: boolean;
  readonly warnings: readonly string[];
}

export interface ShadertoyLicenseReport {
  readonly license: string;
  readonly is_commercial_safe: boolean;
  readonly requires_attribution: boolean;
  readonly author: string | null;
  readonly source_url: string | null;
  readonly attribution_notice: string;
  readonly warning: string | null;
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

/**
 * Import a Synesthesia `.synScene` directory into a validated `.chroma`
 * package via the Tauri `import_syn_scene` command, with a deterministic
 * browser-mode fallback outside Tauri.
 */
export async function importSynScene(
  sourceDir: string,
  targetParentDir?: string,
): Promise<SynImportResult> {
  const trimmedSource = sourceDir.trim();
  const trimmedTarget = targetParentDir?.trim();

  if (isTauri()) {
    return await invoke<SynImportResult>('import_syn_scene', {
      sourceDir: trimmedSource,
      ...(trimmedTarget ? { targetParentDir: trimmedTarget } : {}),
    });
  }

  if (!trimmedSource) {
    throw new Error('source_dir must not be empty');
  }
  if (trimmedSource.includes('\0') || trimmedTarget?.includes('\0')) {
    throw new Error('path must not contain NUL bytes');
  }
  const hasParentTraversal = (path: string): boolean => path.split(/[\\/]+/).includes('..');
  if (hasParentTraversal(trimmedSource) || (trimmedTarget && hasParentTraversal(trimmedTarget))) {
    throw new Error("path must not contain parent-directory ('..') traversal components");
  }

  const segments = trimmedSource.split(/[\\/]+/).filter(Boolean);
  const rawBase = segments.at(-1) ?? 'Imported Scene';
  const strippedBase = rawBase.replace(/\.synscene$/i, '') || rawBase;
  const sceneTitle = strippedBase.replace(/[-_]+/g, ' ').trim() || 'Imported Scene';
  const slug = slugify(sceneTitle);
  const parentDir = trimmedTarget || 'scenes';

  return {
    output_dir: `${parentDir}/${slug}.chroma`,
    scene_title: sceneTitle,
    control_count: 0,
    has_script: false,
    warnings: ['Browser preview mode — .synScene conversion simulated without filesystem write.'],
  };
}

const HEADER_SCAN_LINE_LIMIT = 60;

function extractHeaderCommentLines(source: string): string[] {
  const result: string[] = [];
  let inBlockComment = false;
  const lines = source.split(/\r?\n/).slice(0, HEADER_SCAN_LINE_LIMIT);

  for (const rawLine of lines) {
    let rest = rawLine;
    while (rest.length > 0) {
      if (inBlockComment) {
        const endIdx = rest.indexOf('*/');
        if (endIdx !== -1) {
          const segment = rest.slice(0, endIdx).trim().replace(/^\*+/, '').trim();
          if (segment) result.push(segment);
          inBlockComment = false;
          rest = rest.slice(endIdx + 2);
        } else {
          const segment = rest.trim().replace(/^\*+/, '').trim();
          if (segment) result.push(segment);
          break;
        }
      } else {
        const lineIdx = rest.indexOf('//');
        const blockIdx = rest.indexOf('/*');
        if (lineIdx !== -1 && (blockIdx === -1 || lineIdx < blockIdx)) {
          const segment = rest.slice(lineIdx + 2).trim();
          if (segment) result.push(segment);
          break;
        }
        if (blockIdx !== -1) {
          inBlockComment = true;
          rest = rest.slice(blockIdx + 2);
        } else {
          break;
        }
      }
    }
  }

  return result;
}

function containsWord(haystack: string, word: string): boolean {
  const escaped = word.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`(?<![a-z0-9])${escaped}(?![a-z0-9])`, 'i').test(haystack);
}

interface LicenseClassification {
  readonly license: string;
  readonly isCommercialSafe: boolean;
  readonly requiresAttribution: boolean;
  readonly warning: string | null;
}

function classifyHeaderLicense(commentLines: readonly string[]): LicenseClassification {
  // 1. Explicit SPDX-License-Identifier header first (preserving -NC before generic CC-BY).
  for (const line of commentLines) {
    const spdxMatch = /spdx-license-identifier:\s*(.+)$/i.exec(line);
    if (spdxMatch?.[1]) {
      const expr = spdxMatch[1].trim().toLowerCase();
      if (expr.includes('cc-by-nc-sa')) {
        return {
          license: 'CC-BY-NC-SA',
          isCommercialSafe: false,
          requiresAttribution: true,
          warning:
            'Shader is licensed under CC-BY-NC-SA (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
        };
      }
      if (expr.includes('cc-by-nc')) {
        return {
          license: 'CC-BY-NC',
          isCommercialSafe: false,
          requiresAttribution: true,
          warning:
            'Shader is licensed under CC-BY-NC (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
        };
      }
      if (expr.includes('cc0') || expr.includes('unlicense')) {
        return {
          license: 'CC0-1.0',
          isCommercialSafe: true,
          requiresAttribution: false,
          warning: null,
        };
      }
      if (expr.includes('apache-2')) {
        return {
          license: 'Apache-2.0',
          isCommercialSafe: true,
          requiresAttribution: true,
          warning: null,
        };
      }
      if (containsWord(expr, 'mit')) {
        return {
          license: 'MIT',
          isCommercialSafe: true,
          requiresAttribution: true,
          warning: null,
        };
      }
      if (expr.includes('cc-by-sa')) {
        return {
          license: 'CC-BY-SA',
          isCommercialSafe: true,
          requiresAttribution: true,
          warning: null,
        };
      }
      if (expr.includes('cc-by')) {
        return {
          license: 'CC-BY',
          isCommercialSafe: true,
          requiresAttribution: true,
          warning: null,
        };
      }
      return {
        license: 'Unknown',
        isCommercialSafe: false,
        requiresAttribution: true,
        warning:
          'Shader license could not be determined. Verify licensing terms with the author before commercial use.',
      };
    }
  }

  const lower = commentLines.map((l) => l.toLowerCase()).join('\n');
  const hasNc =
    lower.includes('non-commercial') ||
    lower.includes('noncommercial') ||
    lower.includes('by-nc') ||
    lower.includes('by nc');

  if (
    lower.includes('cc by-nc-sa') ||
    lower.includes('cc-by-nc-sa') ||
    lower.includes('cc by nc sa') ||
    lower.includes('by-nc-sa') ||
    lower.includes('attribution-noncommercial-sharealike') ||
    lower.includes('creativecommons.org/licenses/by-nc-sa')
  ) {
    return {
      license: 'CC-BY-NC-SA',
      isCommercialSafe: false,
      requiresAttribution: true,
      warning:
        'Shader is licensed under CC-BY-NC-SA (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
    };
  }

  if (
    lower.includes('cc by-nc') ||
    lower.includes('cc-by-nc') ||
    lower.includes('cc by nc') ||
    lower.includes('by-nc') ||
    lower.includes('attribution-noncommercial') ||
    lower.includes('creativecommons.org/licenses/by-nc')
  ) {
    return {
      license: 'CC-BY-NC',
      isCommercialSafe: false,
      requiresAttribution: true,
      warning:
        'Shader is licensed under CC-BY-NC (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
    };
  }

  if (lower.includes('all rights reserved')) {
    return {
      license: 'All Rights Reserved',
      isCommercialSafe: false,
      requiresAttribution: true,
      warning:
        'Shader declares All Rights Reserved. Commercial use and redistribution are not permitted without explicit permission from the author.',
    };
  }

  if (
    containsWord(lower, 'cc0') ||
    lower.includes('cc0-1.0') ||
    lower.includes('creative commons zero') ||
    lower.includes('public domain') ||
    containsWord(lower, 'unlicense')
  ) {
    return {
      license: 'CC0-1.0',
      isCommercialSafe: true,
      requiresAttribution: false,
      warning: null,
    };
  }

  if (
    lower.includes('mit license') ||
    lower.includes('mit licence') ||
    lower.includes('licensed under mit') ||
    lower.includes('licensed under the mit') ||
    lower.includes('license: mit') ||
    lower.includes('permission is hereby granted, free of charge') ||
    commentLines.some((line) => {
      const l = line.toLowerCase();
      return (l.includes('license') || l.includes('licence')) && containsWord(l, 'mit');
    })
  ) {
    return {
      license: 'MIT',
      isCommercialSafe: true,
      requiresAttribution: true,
      warning: null,
    };
  }

  if (
    lower.includes('apache-2.0') ||
    lower.includes('apache 2.0') ||
    lower.includes('apache license, version 2.0') ||
    lower.includes('apache license 2.0') ||
    lower.includes('licensed under the apache license')
  ) {
    return {
      license: 'Apache-2.0',
      isCommercialSafe: true,
      requiresAttribution: true,
      warning: null,
    };
  }

  if (
    lower.includes('cc by-sa') ||
    lower.includes('cc-by-sa') ||
    lower.includes('attribution-sharealike') ||
    lower.includes('creativecommons.org/licenses/by-sa')
  ) {
    if (hasNc) {
      return {
        license: 'CC-BY-NC-SA',
        isCommercialSafe: false,
        requiresAttribution: true,
        warning:
          'Shader is licensed under CC-BY-NC-SA (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
      };
    }
    return {
      license: 'CC-BY-SA',
      isCommercialSafe: true,
      requiresAttribution: true,
      warning: null,
    };
  }

  if (
    lower.includes('cc by') ||
    lower.includes('cc-by') ||
    lower.includes('creative commons attribution') ||
    lower.includes('creativecommons.org/licenses/by/')
  ) {
    if (hasNc) {
      return {
        license: 'CC-BY-NC',
        isCommercialSafe: false,
        requiresAttribution: true,
        warning:
          'Shader is licensed under CC-BY-NC (Non-Commercial). Commercial use is not permitted without separate permission from the author.',
      };
    }
    return {
      license: 'CC-BY',
      isCommercialSafe: true,
      requiresAttribution: true,
      warning: null,
    };
  }

  return {
    license: 'CC-BY-NC-SA-3.0 (Shadertoy Default)',
    isCommercialSafe: false,
    requiresAttribution: true,
    warning:
      'No explicit license found in shader header comments. Shadertoy shaders default to CC BY-NC-SA 3.0 (Non-Commercial, ShareAlike), which prohibits commercial use unless the author explicitly grants a permissive license such as MIT, CC0, or Apache-2.0.',
  };
}

function sanitizeAuthor(raw: string): string | null {
  let s = raw.trim();
  const lower = s.toLowerCase();
  for (const marker of ['all rights reserved', 'http://', 'https://', ' - ', ' | ', ' -- ']) {
    const idx = lower.indexOf(marker);
    if (idx !== -1) {
      s = s.slice(0, idx);
      break;
    }
  }
  const cleaned = s
    .trim()
    .replace(/[.,;:\-—–]+$/g, '')
    .trim();
  return cleaned || null;
}

function extractHeaderAuthor(commentLines: readonly string[]): string | null {
  for (const line of commentLines) {
    const labeled = /(?:author:|created by:?|written by:?|shader by:?)\s*(.+)$/i.exec(line);
    if (labeled?.[1]) {
      const cleaned = sanitizeAuthor(labeled[1]);
      if (cleaned) return cleaned;
    }
  }
  for (const line of commentLines) {
    const copyright =
      /copyright\s*(?:\(c\)|©|:)?\s*(?:\d{4}(?:\s*[-–—,]\s*\d{4})*\s*)?(?:by\s+)?(.+)$/i.exec(line);
    if (copyright?.[1]) {
      const cleaned = sanitizeAuthor(copyright[1]);
      if (cleaned) return cleaned;
    }
  }
  for (const line of commentLines) {
    const byLine = /^by[:\s]+(.+)$/i.exec(line.trim());
    if (byLine?.[1]) {
      const cleaned = sanitizeAuthor(byLine[1]);
      if (cleaned) return cleaned;
    }
  }
  return null;
}

function extractHeaderShadertoyUrl(commentLines: readonly string[]): string | null {
  for (const line of commentLines) {
    const match = /https?:\/\/[^\s)>"',;]*shadertoy\.com\/view\/[A-Za-z0-9_-]+/i.exec(line);
    if (match?.[0]) return match[0].replace(/\.+$/, '');
  }
  return null;
}

/**
 * Inspect a Shadertoy GLSL header for license classification, attribution,
 * and non-commercial (`-NC`) warnings via the Tauri `inspect_shadertoy_license`
 * command, with a deterministic client-side fallback heuristic outside Tauri.
 */
export async function inspectShadertoyLicense(
  glslSource: string,
  fallbackAuthor?: string,
  sourceUrl?: string,
): Promise<ShadertoyLicenseReport> {
  const cleanFallbackAuthor = fallbackAuthor?.trim() || undefined;
  const cleanSourceUrl = sourceUrl?.trim() || undefined;

  if (isTauri()) {
    return await invoke<ShadertoyLicenseReport>('inspect_shadertoy_license', {
      glslSource,
      ...(cleanFallbackAuthor !== undefined ? { fallbackAuthor: cleanFallbackAuthor } : {}),
      ...(cleanSourceUrl !== undefined ? { sourceUrl: cleanSourceUrl } : {}),
    });
  }

  const commentLines = extractHeaderCommentLines(glslSource);
  const classified = classifyHeaderLicense(commentLines);
  const author = extractHeaderAuthor(commentLines) ?? cleanFallbackAuthor ?? null;
  const resolvedUrl = cleanSourceUrl ?? extractHeaderShadertoyUrl(commentLines) ?? null;
  const authorLabel = author ?? 'Unknown author';
  const attributionNotice = resolvedUrl
    ? `Shader by ${authorLabel} — License: ${classified.license} (${resolvedUrl})`
    : `Shader by ${authorLabel} — License: ${classified.license}`;

  return {
    license: classified.license,
    is_commercial_safe: classified.isCommercialSafe,
    requires_attribution: classified.requiresAttribution,
    author,
    source_url: resolvedUrl,
    attribution_notice: attributionNotice,
    warning: classified.warning,
  };
}
