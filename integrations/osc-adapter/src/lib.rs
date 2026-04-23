// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity OSC integration adapter.
//!
//! Phase-1 scaffold; no public surface yet. The first-slice
//! implementation (adapter skeleton honouring ADR-013 bind-scope and
//! rate-limit defaults) arrives in a follow-up PR.
//!
//! Governing ADRs:
//!
//! - ADR-005 — integration adapter boundary
//!   (`docs/adr/ADR-005-integration-adapter-boundary.md`).
//! - ADR-013 — networking defaults and security
//!   (`docs/adr/ADR-013-networking-defaults-and-security.md`). Fixes
//!   the bind-scope default and the rate-limit defaults the
//!   first-slice implementation must honour.
