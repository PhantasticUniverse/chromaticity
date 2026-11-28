// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

/**
 * Browser-preview catalog of the real bundled `.chroma` scenes shipped under
 * `resources/scenes/library/`. Used by `Library.svelte` when Tauri IPC is
 * unavailable (e.g. `vite preview` / `just capture-ui`) so the Library grid
 * always reflects Chromaticity's real newly authored scenes instead of legacy
 * placeholder names.
 */

import type { SceneCard } from '../ipc/bindings';

export const BUNDLED_LIBRARY_CATALOG: readonly SceneCard[] = [
  {
    id: 'gm-kerr-hologram-orrery',
    title: 'Kerr Holographic Orrery',
    credit: 'Chromaticity Grandmaster Showcase',
    description:
      'Gravitational lensing orrery with holographic diffraction rings and audio-reactive photon sphere.',
    categories: ['BUILT-IN', '3D', 'AUDIO REACTIVE'],
    tags: ['grandmaster', 'hologram', 'black-hole', '3d'],
    path: 'bundled://gm-kerr-hologram-orrery.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'gm-dyson-prismatic-lotus',
    title: 'Dyson Prismatic Lotus',
    credit: 'Chromaticity Grandmaster Showcase',
    description:
      'Refractive crystal lotus sculpture with spectral caustics and beat-synchronized mirror bloom.',
    categories: ['BUILT-IN', '3D', 'GEOMETRIC'],
    tags: ['grandmaster', 'refraction', 'crystal', 'caustics'],
    path: 'bundled://gm-dyson-prismatic-lotus.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'gm-hyperbolic-opal-cathedral',
    title: 'Hyperbolic Liquid-Opal Cathedral',
    credit: 'Chromaticity Grandmaster Showcase',
    description: 'Non-Euclidean fractal cathedral with liquid opal columns and volumetric godrays.',
    categories: ['BUILT-IN', '3D', 'ABSTRACT'],
    tags: ['grandmaster', 'cathedral', 'fractal', 'opal'],
    path: 'bundled://gm-hyperbolic-opal-cathedral.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'gm-quantum-ferro-nebula',
    title: 'Quantum Ferro-Chromatic Singularity',
    credit: 'Chromaticity Grandmaster Showcase',
    description:
      'Liquid-chrome volumetric nebula warped by bass transients and chromatic dispersion.',
    categories: ['BUILT-IN', '3D', 'AUDIO REACTIVE'],
    tags: ['grandmaster', 'quantum', 'liquid-chrome', 'volumetric'],
    path: 'bundled://gm-quantum-ferro-nebula.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'showdown-biolum-reef',
    title: 'Bioluminescent Alien Reef',
    credit: 'Chromaticity Showdown',
    description: 'Raymarched underwater alien coral reef pulsing with bioluminescent audio waves.',
    categories: ['BUILT-IN', '3D', 'ORGANIC'],
    tags: ['3d', 'raymarching', 'bioluminescent', 'organic'],
    path: 'bundled://showdown-biolum-reef.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'showdown-cyber-torus-knot',
    title: 'Cyber Torus Knot',
    credit: 'Chromaticity Showdown',
    description:
      'Iridescent chrome torus knot with neon energy filaments and beat-locked rotation.',
    categories: ['BUILT-IN', '3D', 'GEOMETRIC'],
    tags: ['3d', 'raymarching', 'torus-knot', 'iridescent'],
    path: 'bundled://showdown-cyber-torus-knot.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'showdown-prism-refraction',
    title: 'Spectral Prism Refraction',
    credit: 'Chromaticity Showdown',
    description: 'Raymarched optical glass prism splitting live audio energy into chromatic beams.',
    categories: ['BUILT-IN', '3D', 'GEOMETRIC'],
    tags: ['3d', 'raymarching', 'prism', 'refraction'],
    path: 'bundled://showdown-prism-refraction.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'psy-liquid-opal-flow',
    title: 'Psy Liquid Opal Flow',
    credit: 'Chromaticity Showcase',
    description:
      'Iridescent liquid-opal domain-warped flow reacting smoothly across all audio bands.',
    categories: ['BUILT-IN', 'ABSTRACT', 'AUDIO REACTIVE'],
    tags: ['psychedelic', 'liquid', 'opal', 'iridescent'],
    path: 'bundled://psy-liquid-opal-flow.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'wind-carved-dunes',
    title: 'Wind-Carved Dunes',
    credit: 'Chromaticity Showcase',
    description:
      'Sweeping sunlit sand dunes sculpted by low-frequency audio swells and wind harmonics.',
    categories: ['BUILT-IN', 'ORGANIC', 'AUDIO REACTIVE'],
    tags: ['dune', 'sand', 'wave'],
    path: 'bundled://wind-carved-dunes.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'demo-kaleido',
    title: 'Demo Kaleido',
    credit: 'chromaticity demo',
    description: 'Six-fold radial mirror with drifting hue and a soft warp envelope.',
    categories: ['BUILT-IN', 'ABSTRACT', 'GEOMETRIC'],
    tags: ['abstract', 'geometric'],
    path: 'bundled://demo-kaleido.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'demo-spectrum',
    title: 'Demo Spectrum',
    credit: 'chromaticity demo',
    description: 'Log-spaced audio spectrum bars driven by live 64-band FFT analysis.',
    categories: ['BUILT-IN', 'AUDIO REACTIVE'],
    tags: ['audio', 'reactive'],
    path: 'bundled://demo-spectrum.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
  {
    id: 'demo-text',
    title: 'Demo Text',
    credit: 'chromaticity demo',
    description: 'Crisp procedural SDF typography overlay for live show callouts.',
    categories: ['BUILT-IN', 'TEXT'],
    tags: ['text'],
    path: 'bundled://demo-text.chroma',
    source: 'Bundled',
    format_tag: 'native-chroma-v1',
  },
];
