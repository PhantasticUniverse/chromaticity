<!--
SPDX-FileCopyrightText: 2026 Chromaticity
SPDX-License-Identifier: CC-BY-4.0
-->

# hg_sdf Vendor Snapshot

- **Origin:** https://mercury.sexy/hg_sdf/
- **Direct source URL:** http://mercury.sexy/hg_sdf/hg_sdf.glsl
- **Version:** 2021-07-28 (the canonical revision date Mercury Demogroup
  carries inline at the top of the `hg_sdf.glsl` file under "version
  2021-07-28")
- **Snapshot date:** 2026-05-22
- **License:** **MIT** (Chromaticity elects the MIT track of the upstream
  dual license).
  - Upstream is dual-licensed under MIT OR CC-BY-NC-4.0 per the inline
    SPDX header in `hg_sdf.glsl` (line 23). The MIT track is the only
    track compatible with Chromaticity's commercial distribution posture
    per `docs/29-cleanroom-legal-policy-baseline.md` — CC-BY-NC-4.0 fails
    the non-commercial guard. Election decision per the same upstream's
    "MIT License" track is unambiguous and irreversible at our end.
  - Copyright (c) 2011-2021 Mercury Demogroup (per the inline MIT block
    in `hg_sdf.glsl`).
  - Verbatim MIT license text at `LICENSES/MIT.txt`.
  - REUSE 3.2 mapping at `REUSE.toml`'s `third_party/hg_sdf/**` override.
  - `deny.toml` `licenses.allow` already covers MIT (no `deny.toml` change
    needed).
- **Trial status:** Vendor is in **production-use election state** from
  the day of this snapshot. Unlike Lygia (which is trial-vendored under
  `LicenseRef-Prosperity-3.0.0` with election deferred to Phase-5
  `EV-LICENSE-AUDIT`), hg_sdf's MIT track is permissive enough that no
  further election is required.

## What we vendored

A single file: `hg_sdf.glsl` (830 lines, ~27 KB). It is a self-contained
GLSL library of signed-distance-function primitives (boxes, spheres,
torus, prism, polyhedra), domain operations (repeat, fold, mirror,
twist), and combination operators (smooth-union, intersection,
subtraction). No external dependencies, no preprocessing required.

## What's NOT included

- Mercury Demogroup's example scenes that demonstrate hg_sdf in use
  (separate downloads on the upstream site; we don't need them — the
  library file is the asset).
- Build tooling / preprocessor wrappers.

## Why we vendor this

The Synesthesia parity sweep flagged 64 PARTIAL rows in
`docs/comparison/features/ssf-format.md` for SDF primitive coverage
that hg_sdf would close at MIT. The library is the de facto standard
for SDF-based rendering in the demoscene and is referenced as such in
Inigo Quilez's writeups + `docs/56-shader-sourcing-research.md`. The
file is small, well-audited (Mercury Demogroup has shipped it since
2011), and the MIT track removes commercial-use friction.

## Upstream-provenance verification (clean-room hygiene)

- The vendored file was downloaded directly from
  `http://mercury.sexy/hg_sdf/hg_sdf.glsl` on 2026-05-22 (no Synesthesia
  intermediation, no third-party mirror).
- File contents are bit-for-bit identical to the upstream as of the
  snapshot date.
- The vendored file carries the upstream's inline SPDX header
  unmodified.
- Per `docs/29-cleanroom-legal-policy-baseline.md`, source tooling
  MUST NOT parse `Contents/` or `chromaticity-planning/` — neither
  was consulted for this vendor.

## Future updates

- New upstream versions on `mercury.sexy/hg_sdf/` would land here as a
  new snapshot, bumping the `Version` and `Snapshot date` fields above
  and replacing `hg_sdf.glsl` verbatim. No diff / patch tracking
  required — the upstream is small enough that diff inspection at
  upgrade time is the right cost.
