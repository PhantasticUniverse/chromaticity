// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-4d `EV-ANALYTICS-ZERO-DEFAULT` static-analysis harness.
//!
//! Row 95 of `docs/26-evidence-artifact-registry.md` declares the
//! invariant: "fresh install with untouched settings produces zero
//! packets to third-party analytics domains over 14-day observation
//! window". This module ships the static-analysis half of the
//! soak harness — a source-tree walker that fails if any known
//! third-party analytics tracker hostname appears as a substring
//! in the production source tree.
//!
//! Two assertion modes ride on top of this module:
//!
//! - **Static analysis (this module).** [`audit_source_tree`] walks
//!   `engine/`, `app/`, `integrations/`, and `src/lib/` looking for
//!   any [`known_analytics_hostnames`] entry as a substring in any
//!   `.rs` / `.ts` / `.svelte` / `.svelte.ts` file. Findings are
//!   line-anchored. The corresponding test
//!   `engine/observability/tests/analytics_zero_default.rs` asserts
//!   the report is empty against the live tree on every PR.
//!
//! - **Runtime code-shape audit (sibling test).** Enumerates every
//!   subsystem that COULD make a network call at boot and asserts
//!   each is in its zero-egress configuration. See
//!   `engine/observability/tests/analytics_zero_default.rs` for the
//!   detail.
//!
//! ADR-013 §Network defaults and ADR-016 §Health metrics frame the
//! posture: zero outbound HTTPS telemetry at any phase except the
//! Phase-4d loopback-only health endpoint. The blocklist below is
//! curated to a representative set of widely-deployed third-party
//! analytics, error-reporting, and product-analytics SaaS domains;
//! it is NOT exhaustive (no static blocklist can be), but the
//! 14-day soak observation methodology in
//! `docs/observability/analytics-zero-default-soak-methodology.md`
//! folds nightly runs of this audit into the calendar gate so that
//! a regression introduced under a hostname not on the curated
//! blocklist still has 13 more chances to be caught by a blocklist
//! widening before the row's Pass transition lands.
//!
//! Self-exclusion. The audit module's own source file plus the
//! sibling integration test plus the soak-methodology doc legitimately
//! cite these hostnames (the blocklist IS the list of forbidden
//! tokens). [`audit_source_tree`] hardcodes a small exclusion set
//! covering exactly those harness files so the audit does not self-
//! trip. Adding new entries to the exclusion set requires AO sign-
//! off.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

/// Curated list of widely-deployed third-party analytics, product-
/// analytics, error-reporting, and session-replay SaaS hostnames.
/// Ordering reflects rough deployment frequency in the wild; the
/// audit treats every entry equally.
///
/// Selection rationale, per
/// `docs/observability/analytics-zero-default-soak-methodology.md`:
///
/// - `google-analytics.com` / `googletagmanager.com` — GA4 + GTM
///   (Google's product-analytics + tag-manager pair).
/// - `facebook.net` — Meta Pixel (third-party ad / analytics
///   pixel; `facebook.com` excluded — too broad, would catch
///   legitimate UA / contact-link references).
/// - `segment.io` — Segment customer-data platform.
/// - `mixpanel.com` — Mixpanel product analytics.
/// - `amplitude.com` — Amplitude product analytics.
/// - `posthog.com` — PostHog (open-source product analytics SaaS).
/// - `sentry.io` — Sentry error-reporting (the closest legitimate
///   call for a desktop app, hence its presence here as a guardrail
///   rather than an absolute prohibition: Phase-4d crash payload
///   redaction is `EV-CRASH-PAYLOAD-REDACTION` row 92, NOT this
///   row; absence of `sentry.io` references in the production tree
///   is the Phase-4d default until ADR-016 is amended).
/// - `hotjar.com` — Hotjar session replay.
/// - `fullstory.com` — FullStory session replay.
/// - `datadoghq.com` — Datadog RUM.
/// - `newrelic.com` — New Relic browser agent.
/// - `matomo.cloud` — Matomo Cloud (self-hosted Matomo not on
///   the list since `matomo` alone has too many false positives).
#[must_use]
pub fn known_analytics_hostnames() -> &'static [&'static str] {
    KNOWN_ANALYTICS_HOSTNAMES
}

const KNOWN_ANALYTICS_HOSTNAMES: &[&str] = &[
    "google-analytics.com",
    "googletagmanager.com",
    "facebook.net",
    "segment.io",
    "mixpanel.com",
    "amplitude.com",
    "posthog.com",
    "sentry.io",
    "hotjar.com",
    "fullstory.com",
    "datadoghq.com",
    "newrelic.com",
    "matomo.cloud",
];

/// A single audit finding: a forbidden analytics hostname appearing
/// in a production-tree source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyticsFinding {
    /// Repo-relative path of the offending file (forward-slash form
    /// for cross-platform stability of the report).
    pub path: String,
    /// 1-indexed line number where the hostname substring appeared.
    pub line: usize,
    /// The blocklist entry that matched. Multiple entries on one
    /// line produce one finding per entry.
    pub hostname: &'static str,
}

/// Aggregate result of an [`audit_source_tree`] sweep.
#[derive(Debug, Clone, Default)]
pub struct AnalyticsAuditReport {
    /// Every finding surfaced during the walk, in directory-walk
    /// order. An empty `findings` vector is the Pass condition for
    /// the static-analysis half of `EV-ANALYTICS-ZERO-DEFAULT`.
    pub findings: Vec<AnalyticsFinding>,
}

impl AnalyticsAuditReport {
    /// Convenience: `true` when no findings surfaced.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Audit-walk roots relative to the repo root. The set covers every
/// production-source location the audit cares about and excludes
/// `tests/`, `docs/`, `chromaticity-planning/`, `Contents/`,
/// `target/`, `node_modules/`, `dist/`, `LICENSES/`, and
/// `third_party/` by omission.
const AUDIT_ROOTS: &[&str] = &["engine", "app", "integrations", "src/lib"];

/// File extensions the audit reads. Anything else is ignored
/// (lockfiles, manifests, binary assets, configuration, vendored
/// data).
const AUDIT_EXTENSIONS: &[&str] = &["rs", "ts", "svelte"];

/// Repo-relative paths the audit deliberately does NOT walk. These
/// are the harness files that legitimately cite the blocklist
/// hostnames (the blocklist itself is the list of forbidden
/// tokens). Adding to this set requires AO sign-off.
const AUDIT_SELF_EXCLUSIONS: &[&str] = &[
    "engine/observability/src/analytics_audit.rs",
    "engine/observability/tests/analytics_zero_default.rs",
];

/// Walk the production source tree under `repo_root` looking for
/// any [`known_analytics_hostnames`] entry as a substring in any
/// `.rs` / `.ts` / `.svelte` / `.svelte.ts` file under the
/// [`AUDIT_ROOTS`] set, returning every match line-anchored.
///
/// Comments are NOT stripped before matching: an analytics URL in
/// a comment is still a leak signal (a developer who knows the
/// hostname has documented intent to talk to it; that intent is
/// what the audit objects to).
///
/// I/O errors during the walk surface as `Err`; a clean walk that
/// finds nothing returns `Ok(AnalyticsAuditReport { findings:
/// vec![] })`.
pub fn audit_source_tree(repo_root: &Path) -> io::Result<AnalyticsAuditReport> {
    let mut report = AnalyticsAuditReport::default();
    for root in AUDIT_ROOTS {
        let root_path = repo_root.join(root);
        if !root_path.exists() {
            // Roots are best-effort: if the workspace layout shifts
            // (e.g. `src/lib/` migrates under `app/ui/` per the
            // CLAUDE.md "may migrate later" note), missing roots are
            // not a hard failure. The audit still runs against the
            // surviving roots.
            continue;
        }
        walk_dir(&root_path, repo_root, &mut report)?;
    }
    // Stable ordering for assertion legibility.
    report.findings.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.hostname.cmp(b.hostname))
    });
    Ok(report)
}

fn walk_dir(dir: &Path, repo_root: &Path, report: &mut AnalyticsAuditReport) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            // Skip target/ and node_modules/ if they ever appear under
            // an audit root (defensive; AUDIT_ROOTS doesn't currently
            // intersect them, but a future workspace member could).
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                if matches!(name, "target" | "node_modules" | "dist") {
                    continue;
                }
            }
            walk_dir(&path, repo_root, report)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if !has_audited_extension(&path) {
            continue;
        }
        let rel = repo_relative(&path, repo_root);
        if AUDIT_SELF_EXCLUSIONS.contains(&rel.as_str()) {
            continue;
        }
        scan_file(&path, &rel, report)?;
    }
    Ok(())
}

fn has_audited_extension(path: &Path) -> bool {
    let name = match path.file_name().and_then(OsStr::to_str) {
        Some(n) => n,
        None => return false,
    };
    // `.svelte.ts` is not a stdlib `Path::extension` target — the
    // extension function returns `ts`. We accept that under `ts`
    // bucket below; the explicit name check here keeps the
    // intent clear.
    if name.ends_with(".svelte.ts") {
        return true;
    }
    let ext = match path.extension().and_then(OsStr::to_str) {
        Some(e) => e,
        None => return false,
    };
    AUDIT_EXTENSIONS.contains(&ext)
}

fn repo_relative(path: &Path, repo_root: &Path) -> String {
    let rel = path.strip_prefix(repo_root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

fn scan_file(path: &Path, rel_path: &str, report: &mut AnalyticsAuditReport) -> io::Result<()> {
    let bytes = fs::read(path)?;
    // Skip files that aren't valid UTF-8 (binary assets sneaking
    // through under one of the audited extensions). The audit only
    // makes sense over textual source.
    let text = match std::str::from_utf8(&bytes) {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };
    for (idx, line) in text.lines().enumerate() {
        for hostname in KNOWN_ANALYTICS_HOSTNAMES {
            if line.contains(hostname) {
                report.findings.push(AnalyticsFinding {
                    path: rel_path.to_string(),
                    line: idx + 1,
                    hostname,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod analytics_audit_pins {
    //! Evidence: EV-ANALYTICS-ZERO-DEFAULT (T1) — static-analysis
    //! harness behaviour pins. Validates the source-tree walker
    //! handles empty trees, planted blocklist entries, and the
    //! "comments are still findings" rule. The full live-tree
    //! sweep lives in the sibling integration test
    //! `engine/observability/tests/analytics_zero_default.rs` so
    //! the T1 pins here stay isolated from the production tree.

    use super::{
        AnalyticsAuditReport, AnalyticsFinding, audit_source_tree, known_analytics_hostnames,
    };
    use std::fs;
    use std::path::PathBuf;

    /// Spin up an empty scratch directory inside the cargo target
    /// dir so the pins do not race with sibling tests over a
    /// shared `tempdir`. Path is deterministic per pin name.
    fn scratch(pin: &str) -> PathBuf {
        let base = std::env::temp_dir().join("chromaticity-analytics-audit-pins");
        let path = base.join(pin);
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn empty_tree_produces_empty_report() {
        let root = scratch("empty_tree");
        let report = audit_source_tree(&root).unwrap();
        assert!(
            report.is_clean(),
            "empty tree must produce a clean audit, got {:?}",
            report.findings
        );
    }

    #[test]
    fn missing_audit_root_does_not_error() {
        // Passing a real-but-empty repo root with none of the
        // expected AUDIT_ROOTS subdirs present must not error;
        // the walker treats missing roots as best-effort.
        let root = scratch("missing_root");
        let report = audit_source_tree(&root).unwrap();
        assert!(report.is_clean());
    }

    #[test]
    fn planted_hostname_in_engine_rs_file_surfaces_as_finding() {
        let root = scratch("planted_engine");
        let engine_dir = root.join("engine").join("scratch").join("src");
        fs::create_dir_all(&engine_dir).unwrap();
        let target = engine_dir.join("planted.rs");
        // Construct the offending content from concatenation so
        // the literal blocklist hostname does not appear in the
        // pin source itself (which would route around the
        // self-exclusion list — the pins do not live on the
        // exclusion list, only the production module + its
        // sibling integration test do).
        let body = format!(
            "// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary\n\
             pub const TRACKER: &str = \"https://www.{}/collect\";\n",
            // First entry is google-analytics.com per the canonical
            // ordering — the pin reads the runtime list to avoid
            // hardcoding the literal here.
            known_analytics_hostnames()[0],
        );
        fs::write(&target, body).unwrap();

        let report = audit_source_tree(&root).unwrap();
        assert_eq!(
            report.findings.len(),
            1,
            "expected one finding, got {:?}",
            report.findings
        );
        let finding = &report.findings[0];
        assert_eq!(finding.path, "engine/scratch/src/planted.rs");
        assert_eq!(finding.line, 2);
        assert_eq!(finding.hostname, known_analytics_hostnames()[0]);
    }

    #[test]
    fn planted_hostname_in_comment_still_surfaces() {
        // Per the module-level rationale, comments are still
        // findings: an analytics URL in a comment documents intent
        // to talk to it.
        let root = scratch("planted_comment");
        let engine_dir = root.join("engine").join("scratch").join("src");
        fs::create_dir_all(&engine_dir).unwrap();
        let target = engine_dir.join("commented.rs");
        let body = format!(
            "// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary\n\
             // TODO: post to {} for marketing analytics later\n\
             pub const NOOP: () = ();\n",
            known_analytics_hostnames()[1], // googletagmanager.com
        );
        fs::write(&target, body).unwrap();

        let report = audit_source_tree(&root).unwrap();
        assert_eq!(
            report.findings.len(),
            1,
            "comment-only finding must surface, got {:?}",
            report.findings
        );
        let finding = &report.findings[0];
        assert_eq!(finding.line, 2);
        assert_eq!(finding.hostname, known_analytics_hostnames()[1]);
    }

    #[test]
    fn planted_hostname_in_typescript_and_svelte_files_surfaces() {
        let root = scratch("planted_frontend");
        let lib_dir = root.join("src").join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        let ts_path = lib_dir.join("tracker.ts");
        let svelte_path = lib_dir.join("Banner.svelte");
        let svelte_ts_path = lib_dir.join("store.svelte.ts");

        let host_a = known_analytics_hostnames()[2]; // facebook.net
        let host_b = known_analytics_hostnames()[3]; // segment.io
        let host_c = known_analytics_hostnames()[4]; // mixpanel.com

        fs::write(
            &ts_path,
            format!("export const URL = 'https://{host_a}/p';\n"),
        )
        .unwrap();
        fs::write(
            &svelte_path,
            format!("<script>const u = 'https://{host_b}/v1/t';</script>\n"),
        )
        .unwrap();
        fs::write(
            &svelte_ts_path,
            format!("export const ID = 'https://{host_c}/lib';\n"),
        )
        .unwrap();

        let report = audit_source_tree(&root).unwrap();
        // Stable-sorted by path then line — verify all three landed.
        let hostnames: Vec<&str> = report.findings.iter().map(|f| f.hostname).collect();
        assert!(
            hostnames.contains(&host_a),
            "ts hit missing: {:?}",
            report.findings
        );
        assert!(
            hostnames.contains(&host_b),
            "svelte hit missing: {:?}",
            report.findings
        );
        assert!(
            hostnames.contains(&host_c),
            "svelte.ts hit missing: {:?}",
            report.findings
        );
        assert_eq!(report.findings.len(), 3);
    }

    #[test]
    fn non_audited_extensions_are_ignored() {
        // Markdown, json, toml, lock files etc. live in production
        // paths but are NOT audit-relevant — a hostname appearing
        // in an SBOM or a third-party manifest is not a leak.
        let root = scratch("non_audited_ext");
        let engine_dir = root.join("engine").join("scratch");
        fs::create_dir_all(&engine_dir).unwrap();
        let md_path = engine_dir.join("notes.md");
        let toml_path = engine_dir.join("Cargo.toml");
        let host = known_analytics_hostnames()[0];
        fs::write(&md_path, format!("see {host}\n")).unwrap();
        fs::write(&toml_path, format!("# {host}\n")).unwrap();
        let report = audit_source_tree(&root).unwrap();
        assert!(
            report.is_clean(),
            "non-audited extensions must not surface findings, got {:?}",
            report.findings
        );
    }

    #[test]
    fn audit_self_exclusion_skips_the_audit_module_file() {
        // Plant a blocklisted hostname at exactly the self-
        // exclusion path; verify the walker skips it. This is the
        // mechanism that prevents the audit module's own blocklist
        // text from self-tripping the live-tree sweep.
        let root = scratch("self_exclusion");
        let module_dir = root.join("engine").join("observability").join("src");
        fs::create_dir_all(&module_dir).unwrap();
        let target = module_dir.join("analytics_audit.rs");
        let host = known_analytics_hostnames()[0];
        fs::write(&target, format!("// {host}\n")).unwrap();
        let report = audit_source_tree(&root).unwrap();
        assert!(
            report.is_clean(),
            "self-exclusion must skip the audit module file itself, got {:?}",
            report.findings
        );
    }

    #[test]
    fn finding_struct_is_clone_eq_and_useful() {
        // Cheap pin to keep the public AnalyticsFinding shape
        // honest under refactor.
        let f = AnalyticsFinding {
            path: "engine/x.rs".to_string(),
            line: 7,
            hostname: known_analytics_hostnames()[0],
        };
        let g = f.clone();
        assert_eq!(f, g);
        let report = AnalyticsAuditReport { findings: vec![f] };
        assert!(!report.is_clean());
        assert_eq!(report.findings.len(), 1);
    }

    #[test]
    fn blocklist_is_non_empty_and_lowercased() {
        let list = known_analytics_hostnames();
        assert!(
            list.len() >= 8,
            "blocklist too narrow: {} entries",
            list.len()
        );
        for host in list {
            assert_eq!(
                *host,
                host.to_lowercase(),
                "blocklist entries must be lowercase"
            );
            assert!(
                host.contains('.'),
                "entry {host:?} should look like a hostname"
            );
        }
    }
}
