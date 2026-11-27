// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

import { describe, expect, it } from 'vitest';

import { autoRepairGlsl, buildAgentRepairPrompt } from './glslRepair';

describe('glslRepair', () => {
  it('upgrades legacy GLSL and Shadertoy uniforms to Chromaticity GLSL 4.50', () => {
    const legacy = [
      '#version 300 es',
      'void main() {',
      '  vec2 p = gl_FragCoord.xy / iResolution.xy;',
      '  vec4 c = texture2D(tex, p + sin(iTime));',
      '  gl_FragColor = c;',
      '}',
    ].join('\n');

    const res = autoRepairGlsl(legacy);
    expect(res.changed).toBe(true);
    expect(res.source).toContain('#version 450');
    expect(res.source).toContain('texture(tex,');
    expect(res.source).toContain('chroma_Time.x');
    expect(res.source).toContain('chroma_Resolution.xy');
    expect(res.source).toContain('fragColor = c;');
    expect(res.applied.length).toBe(5);
  });

  it('flags mat2 and derivative usage as Naga-26 advisory warnings', () => {
    const src = '#version 450\nvoid main() { mat2 m = mat2(1.0); float w = fwidth(1.0); }';
    const res = autoRepairGlsl(src);
    expect(res.changed).toBe(false);
    expect(res.warnings.length).toBe(2);
  });

  it('builds a structured agent repair prompt with scene id and error', () => {
    const prompt = buildAgentRepairPrompt(
      'my-scene',
      '#version 450\nvoid main(){}',
      'syntax error',
    );
    expect(prompt).toContain('my-scene');
    expect(prompt).toContain('syntax error');
    expect(prompt).toContain('```glsl');
  });
});
