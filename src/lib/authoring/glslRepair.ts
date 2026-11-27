// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Deterministic 1-click GLSL 4.50 / Naga auto-repair engine for Author.svelte.
 *
 * Detects and fixes the most common GLSL 1.x / Shadertoy / WebGL-to-Chromaticity
 * (`wgpu 26` + `naga 26` GLSL 4.50) incompatibilities and builds structured
 * diagnostic prompts for AI-assisted repair when manual edits are needed.
 */

export interface GlslRepairAction {
  readonly id: string;
  readonly label: string;
  readonly description: string;
  readonly count: number;
}

export interface GlslRepairResult {
  readonly source: string;
  readonly changed: boolean;
  readonly applied: readonly GlslRepairAction[];
  readonly warnings: readonly string[];
}

/**
 * Apply deterministic safe auto-repairs to bring legacy GLSL / Shadertoy code
 * closer to Chromaticity's `#version 450` + `naga 26` contract.
 */
export function autoRepairGlsl(source: string): GlslRepairResult {
  let next = source;
  const applied: GlslRepairAction[] = [];
  const warnings: string[] = [];

  // 1. Ensure `#version 450` header
  if (!/^\s*#version\s+450\b/m.test(next)) {
    const hadOtherVersion = /^\s*#version\s+\d+.*$/m.test(next);
    if (hadOtherVersion) {
      next = next.replace(/^\s*#version\s+\d+.*$/m, '#version 450');
    } else {
      next = `#version 450\n${next}`;
    }
    applied.push({
      id: 'version-450',
      label: 'Upgrade to #version 450',
      description: 'Ensured GLSL 4.50 header required by naga 26.',
      count: 1,
    });
  }

  // 2. Replace deprecated `texture2D(` with `texture(`
  const texMatches = next.match(/\btexture2D\s*\(/g);
  if (texMatches && texMatches.length > 0) {
    next = next.replace(/\btexture2D\s*\(/g, 'texture(');
    applied.push({
      id: 'texture2d-to-texture',
      label: 'Replace texture2D() with texture()',
      description: 'GLSL 4.50 uses texture() instead of legacy texture2D().',
      count: texMatches.length,
    });
  }

  // 3. Replace legacy `gl_FragColor` with `fragColor`
  const fragMatches = next.match(/\bgl_FragColor\b/g);
  if (fragMatches && fragMatches.length > 0) {
    next = next.replace(/\bgl_FragColor\b/g, 'fragColor');
    applied.push({
      id: 'gl-frag-color',
      label: 'Replace gl_FragColor with fragColor',
      description:
        'Mapped legacy built-in gl_FragColor to layout(location = 0) out vec4 fragColor.',
      count: fragMatches.length,
    });
  }

  // 4. Replace `iTime` -> `chroma_Time.x` and `iResolution` -> `chroma_Resolution`
  const iTimeMatches = next.match(/\biTime\b/g);
  if (iTimeMatches && iTimeMatches.length > 0) {
    next = next.replace(/\biTime\b/g, 'chroma_Time.x');
    applied.push({
      id: 'itime-to-chroma',
      label: 'Map iTime to chroma_Time.x',
      description: 'Rewrote Shadertoy iTime uniform to Chromaticity chroma_Time.x.',
      count: iTimeMatches.length,
    });
  }

  const iResMatches = next.match(/\biResolution\b/g);
  if (iResMatches && iResMatches.length > 0) {
    next = next.replace(/\biResolution\b/g, 'chroma_Resolution');
    applied.push({
      id: 'ires-to-chroma',
      label: 'Map iResolution to chroma_Resolution',
      description: 'Rewrote Shadertoy iResolution uniform to Chromaticity chroma_Resolution.',
      count: iResMatches.length,
    });
  }

  // 5. Advisory warnings for Naga-26 constructs requiring structural rewrite
  if (/\bmat2\b/.test(next)) {
    warnings.push(
      'Detected `mat2` (naga 26 GLSL 4.50 rejects some mat2 constructors/multiplies — prefer an explicit `vec2 rot2(vec2 v, float a)` helper).',
    );
  }
  if (/\b(dFdx|dFdy|fwidth)\s*\(/.test(next)) {
    warnings.push(
      'Detected screen-space derivatives (`dFdx`/`dFdy`/`fwidth`) — prefer analytical anti-aliasing or smoothstep for portable wgpu/Metal output.',
    );
  }

  return {
    source: next,
    changed: next !== source,
    applied,
    warnings,
  };
}

/**
 * Build a concise, structured repair prompt containing the active GLSL source
 * and the exact `naga`/`wgpu` compiler error for an AI shader copilot.
 */
export function buildAgentRepairPrompt(
  sceneId: string,
  source: string,
  compilerError: string,
): string {
  return [
    `# Chromaticity GLSL 4.50 Repair Request`,
    `- **Scene:** \`${sceneId}\``,
    `- **Compiler Error:** \`${compilerError.trim()}\``,
    `- **Contract:** GLSL 4.50 (\`naga 26\` + \`wgpu 26\`), no \`mat2\`, no \`dFdx/dFdy/fwidth\`, constant-only \`chroma_AudioFFT[0..7]\` indexing, \`+Y\` up via \`0.5 - uv.y\`.`,
    '',
    '```glsl',
    source.trim(),
    '```',
  ].join('\n');
}
