// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Chromaticity engine runtime.
//!
//! Phase-1 scaffold; no public surface yet. The first-slice
//! implementation (canonical state core plus the command-bus trait)
//! arrives in a follow-up PR. The render-graph implementation starts
//! as a module in this crate and will be extracted only when
//! incremental-compile savings or license-boundary concerns justify.
//!
//! Governing ADRs:
//!
//! - ADR-001 — runtime model (`docs/adr/ADR-001-runtime-model.md`).
//! - ADR-004 — canonical state model (`docs/adr/ADR-004-state-model.md`).
//! - ADR-006 — thread and process model
//!   (`docs/adr/ADR-006-thread-and-process-model.md`).
