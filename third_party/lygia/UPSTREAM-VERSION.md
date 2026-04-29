<!--
SPDX-FileCopyrightText: 2026 Chromaticity
SPDX-License-Identifier: CC-BY-4.0
-->

# Lygia Vendor Snapshot

- **Origin:** https://github.com/patriciogonzalezvivo/lygia
- **Version:** 1.1.4 (upstream tag `1.1.4`; the v-prefixed `v1.1.4` form does
  not exist on the upstream tag list — only `v1.1.0` and `v1.1.6` carry the
  `v` prefix, while the contiguous `1.1.1` ... `1.1.7` series uses the bare
  numeric form. The brief's "v1.1.4" wording is best-read as "version 1.1.4")
- **Commit SHA:** `28f220b6045cbbd38ad6e23a69d21c5e67070cdd`
- **Snapshot date:** 2026-04-29
- **License:** Prosperity Public License 3.0.0 (SPDX identifier
  `LicenseRef-Prosperity-3.0.0` — Prosperity-3.0.0 is not on the SPDX license
  list as of the snapshot date, so the canonical identifier uses the
  `LicenseRef-` prefix). Verbatim text at
  `LICENSES/LicenseRef-Prosperity-3.0.0.txt`.
- **Trial status:** AO-approved trial; final commercial-use license election
  is deferred to Phase-5 `EV-LICENSE-AUDIT` per
  `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library
  Handling. The trial may resolve to election under an MIT-equivalent
  carve-out, removal entirely (clean-room shim fallback per `docs/29` line
  106), or paid Prosperity commercial. Until that decision lands, downstream
  consumption is constrained to the `LicenseRef-Prosperity-3.0.0`
  trial-narrow allow slot in `deny.toml` (per the in-file comment) and to
  the trial-period scoping documented in `engine/render/src/glsl_includes.rs`
  module-level prose.
- **Snapshot exclusions:** the upstream `.git/`, `.github/`, `examples/`,
  `samples/` trees were excluded from the snapshot. All functional GLSL
  subdirectories (`color/`, `math/`, `space/`, `lighting/`, `generative/`,
  `sample/`, `morphological/`, `filter/`, `simulate/`, `animation/`,
  `geometry/`, `sdf/`, `distort/`, `draw/`) plus the upstream `LICENSE.md`,
  `README.md`, `package.json`, and the per-language top-level dispatchers
  (`math.glsl`, `math.hlsl`, `math.cuh`, `sample.glsl`, `sample.hlsl`,
  `sdf.glsl`, `sdf.hlsl`, `version.glsl`, `version.hlsl`) are included.
- **Refresh procedure:** delete `third_party/lygia/`, re-clone at the
  desired tag, snapshot-copy with the same exclusions, update this file's
  Version + Commit SHA + Snapshot date fields, and update the corresponding
  rows in `docs/26-evidence-artifact-registry.md`
  (`EV-LIBRARY-LICENSE-TRIAL`) and
  `docs/29-cleanroom-legal-policy-baseline.md` Sec.Bundled GLSL Library
  Handling. A snapshot bump is a `policy`-scoped commit because it changes
  `deny.toml` semantics.

This vendor snapshot is consumed by `engine/render/src/glsl_includes.rs` for
trial-period scene authoring per `docs/29-cleanroom-legal-policy-baseline.md`
Sec.Bundled GLSL Library Handling. The preprocessor is the only first-party
surface that resolves `lygia/...` `#include` paths; if the AO drops Lygia
at Phase-5 `EV-LICENSE-AUDIT`, the preprocessor stays as a generic
include-resolver and `third_party/lygia/` is removed in a paired commit.
