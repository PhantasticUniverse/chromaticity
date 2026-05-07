<!--
SPDX-License-Identifier: CC-BY-4.0
SPDX-FileCopyrightText: 2026 Chromaticity
-->

# Chromaticity Vendor Note: Syphon-Framework

## Source

- Upstream repository: <https://github.com/Syphon/Syphon-Framework>
- Vendored upstream commit: `71351d4b484cd2d1917867f7846a5cdca724552d`
- Vendoring date: 2026-05-07
- Vendored by: Phase 1.7 of Wave 1c-β (`docs/wave-1c-beta-acceptance-and-vendor`)

## License

BSD-3-Clause. See `License.txt` (canonical upstream text) and
`LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt` (chromaticity-side LICENSES/
mirror referenced by the `REUSE.toml` annotation for `third_party/Syphon-Framework/**`).

`BSD-3-Clause` is already permitted by `deny.toml` `[licenses].allow`; no policy
change required for this vendor.

## Why vendored

Wave 1c-β `EV-INTEGRATION-MATRIX` Syphon arm (ADR-021) requires the canonical
Syphon-Framework objc/c source to build a static `Syphon.framework` once via
Xcode for `integrations/syphon-adapter/` to link against. Vendoring at a pinned
commit gives clean-room provenance + redistribution under BSD-3-Clause without
a runtime dependency on the developer's local clone.

The framework binary itself (`*.framework` build artifact) is NOT vendored —
only the source tree. The binary is produced by a dedicated Phase-3 sub-PR
under Wave 1c-β.

## Re-vendoring

If a later wave needs to bump the pinned upstream commit:

```bash
# 1. Refresh the reference clone (or clone fresh under reference-repos/).
git clone https://github.com/Syphon/Syphon-Framework \
  /path/to/reference-repos/Syphon-Framework

# 2. Verify the desired upstream commit hash.
git -C /path/to/reference-repos/Syphon-Framework log -1 --format=%H

# 3. From the chromaticity repo root, replace the vendored tree.
rm -rf third_party/Syphon-Framework
cp -R /path/to/reference-repos/Syphon-Framework third_party/
rm -rf third_party/Syphon-Framework/.git

# 4. Restore (or refresh) this CHROMATICITY_VENDOR_NOTE.md with the new
#    `Vendored upstream commit` and `Vendoring date` fields.

# 5. If the upstream License.txt has changed, mirror the changes into
#    LICENSES/LicenseRef-Syphon-BSD-3-Clause.txt verbatim.

# 6. If upstream copyright lines have shifted (e.g. additional contributors),
#    update the SPDX-FileCopyrightText list in the
#    `path = "third_party/Syphon-Framework/**"` annotation in REUSE.toml.

# 7. Run policy verification.
bash scripts/evidence-tag-lint.sh
cargo deny check licenses
npx prettier --check REUSE.toml
```

## Modifications

Chromaticity does not modify upstream Syphon-Framework source files. The
vendored tree is a verbatim snapshot. Any patches required for chromaticity
integration MUST land under `integrations/syphon-adapter/` as
chromaticity-proprietary code linking against the unmodified framework.
