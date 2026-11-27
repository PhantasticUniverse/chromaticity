// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shadertoy GLSL header comment license and attribution auto-parser.
//!
//! Shadertoy shaders default to Creative Commons Attribution-NonCommercial-ShareAlike
//! 3.0 Unported (`CC BY-NC-SA 3.0`) under Shadertoy's Terms of Service unless the
//! author explicitly states a different license (such as `MIT`, `CC0`, or `Apache-2.0`)
//! in the shader comments.
//!
//! This module inspects the leading comment header (first 60 lines) of an imported
//! Shadertoy GLSL source and produces a conservative [`ShadertoyLicenseReport`] that:
//! - Recognizes explicit SPDX identifiers and common Shadertoy comment conventions
//!   (CC0 / Public Domain, MIT, Apache-2.0, CC BY, CC BY-SA, CC BY-NC, CC BY-NC-SA,
//!   and All Rights Reserved).
//! - Always checks `-NC` (Non-Commercial) qualifiers before generic `CC BY` / `CC BY-SA`
//!   so non-commercial restrictions are never stripped.
//! - Defaults conservatively to [`LicenseKind::ShadertoyDefaultNc`] (`is_commercial_safe = false`)
//!   with an explanatory warning whenever no explicit license is found.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Maximum number of leading source lines scanned for license and attribution headers.
const HEADER_SCAN_LINE_LIMIT: usize = 60;

/// Classified license of an imported Shadertoy shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LicenseKind {
    /// CC0 1.0 Universal / Public Domain dedication.
    Cc0,
    /// MIT License.
    Mit,
    /// Apache License 2.0.
    Apache2,
    /// Creative Commons Attribution (`CC BY`, commercial use permitted with attribution).
    CcBy,
    /// Creative Commons Attribution-ShareAlike (`CC BY-SA`, commercial use permitted with attribution & share-alike).
    CcBySa,
    /// Creative Commons Attribution-NonCommercial (`CC BY-NC`, non-commercial only).
    CcByNc,
    /// Creative Commons Attribution-NonCommercial-ShareAlike (`CC BY-NC-SA`, non-commercial only).
    CcByNcSa,
    /// Explicit "All Rights Reserved" proprietary notice.
    AllRightsReserved,
    /// Shadertoy default terms (`CC BY-NC-SA 3.0`, non-commercial) applied when no explicit license is declared.
    ShadertoyDefaultNc,
    /// Unclassified license state.
    Unknown,
}

impl LicenseKind {
    /// Returns the canonical SPDX identifier or human-readable license label.
    ///
    /// Non-commercial (`-NC`) qualifiers are always preserved in the returned label.
    #[must_use]
    pub fn spdx_or_label(self) -> &'static str {
        match self {
            Self::Cc0 => "CC0-1.0",
            Self::Mit => "MIT",
            Self::Apache2 => "Apache-2.0",
            Self::CcBy => "CC-BY",
            Self::CcBySa => "CC-BY-SA",
            Self::CcByNc => "CC-BY-NC",
            Self::CcByNcSa => "CC-BY-NC-SA",
            Self::AllRightsReserved => "All Rights Reserved",
            Self::ShadertoyDefaultNc => "CC-BY-NC-SA-3.0 (Shadertoy Default)",
            Self::Unknown => "Unknown",
        }
    }

    /// Returns `true` only when the license permits commercial use.
    ///
    /// Any `-NC` license, `AllRightsReserved`, `ShadertoyDefaultNc`, or `Unknown`
    /// conservatively returns `false`.
    #[must_use]
    pub fn is_commercial_safe(self) -> bool {
        matches!(
            self,
            Self::Cc0 | Self::Mit | Self::Apache2 | Self::CcBy | Self::CcBySa
        )
    }

    /// Returns `true` when attribution is required (all licenses except [`LicenseKind::Cc0`]).
    #[must_use]
    pub fn requires_attribution(self) -> bool {
        !matches!(self, Self::Cc0)
    }
}

/// Structured license and attribution report produced on Shadertoy import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadertoyLicenseReport {
    /// Classified license kind.
    pub license: LicenseKind,
    /// Whether the classified license permits commercial use.
    pub is_commercial_safe: bool,
    /// Whether the classified license requires author attribution.
    pub requires_attribution: bool,
    /// Extracted author name from header comments, or the caller-supplied fallback author.
    pub author: Option<String>,
    /// Source URL (caller-supplied or extracted from header comments).
    pub source_url: Option<String>,
    /// Formatted attribution notice suitable for display or export metadata.
    pub attribution_notice: String,
    /// Conservative warning when commercial use is restricted or license terms are implicit.
    pub warning: Option<String>,
}

/// Parses GLSL header comments (first 60 lines) of a Shadertoy shader and returns a
/// conservative [`ShadertoyLicenseReport`].
///
/// When no explicit license is detected in the header comments, the report defaults to
/// [`LicenseKind::ShadertoyDefaultNc`] (`is_commercial_safe = false`) with a warning
/// explaining Shadertoy's default non-commercial terms.
#[must_use]
pub fn parse_shadertoy_license(
    glsl_source: &str,
    fallback_author: Option<&str>,
    source_url: Option<&str>,
) -> ShadertoyLicenseReport {
    let comment_lines = extract_header_comment_lines(glsl_source, HEADER_SCAN_LINE_LIMIT);
    let license = detect_license(&comment_lines);

    let extracted_author = extract_author(&comment_lines);
    let author = extracted_author.or_else(|| clean_non_empty(fallback_author));

    let resolved_url =
        clean_non_empty(source_url).or_else(|| extract_shadertoy_url(&comment_lines));

    let attribution_notice =
        format_attribution_notice(license, author.as_deref(), resolved_url.as_deref());
    let warning = build_warning(license);

    ShadertoyLicenseReport {
        license,
        is_commercial_safe: license.is_commercial_safe(),
        requires_attribution: license.requires_attribution(),
        author,
        source_url: resolved_url,
        attribution_notice,
        warning,
    }
}

/// Extracts comment text lines from the first `max_lines` of GLSL source.
///
/// Only text inside `// ...` and `/* ... */` comments is returned so GLSL code tokens
/// cannot trigger false-positive license matches.
fn extract_header_comment_lines(source: &str, max_lines: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_block_comment = false;

    for raw_line in source.lines().take(max_lines) {
        let mut rest = raw_line;
        while !rest.is_empty() {
            if in_block_comment {
                if let Some(end_idx) = rest.find("*/") {
                    let segment = clean_block_comment_line(&rest[..end_idx]);
                    if !segment.is_empty() {
                        result.push(segment);
                    }
                    in_block_comment = false;
                    rest = &rest[end_idx + 2..];
                } else {
                    let segment = clean_block_comment_line(rest);
                    if !segment.is_empty() {
                        result.push(segment);
                    }
                    break;
                }
            } else {
                let line_idx = rest.find("//");
                let block_idx = rest.find("/*");

                match (line_idx, block_idx) {
                    (Some(li), Some(bi)) if li < bi => {
                        let segment = rest[li + 2..].trim();
                        if !segment.is_empty() {
                            result.push(segment.to_string());
                        }
                        break;
                    }
                    (Some(li), None) => {
                        let segment = rest[li + 2..].trim();
                        if !segment.is_empty() {
                            result.push(segment.to_string());
                        }
                        break;
                    }
                    (_, Some(bi)) => {
                        in_block_comment = true;
                        rest = &rest[bi + 2..];
                    }
                    (None, None) => break,
                }
            }
        }
    }

    result
}

/// Strips leading block-comment decoration (`*`) and surrounding whitespace.
fn clean_block_comment_line(segment: &str) -> String {
    segment.trim().trim_start_matches('*').trim().to_string()
}

/// Detects the license kind from extracted header comment lines.
///
/// Priority strictly checks `-NC` variants before generic `CC BY` / `CC BY-SA` so
/// non-commercial qualifiers are never stripped.
fn detect_license(comment_lines: &[String]) -> LicenseKind {
    // 1. Check explicit SPDX-License-Identifier headers first.
    for line in comment_lines {
        if let Some(kind) = parse_spdx_line(line) {
            return kind;
        }
    }

    let joined_lower = comment_lines
        .iter()
        .map(|line| line.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n");

    let has_noncommercial_qualifier = joined_lower.contains("non-commercial")
        || joined_lower.contains("noncommercial")
        || joined_lower.contains("by-nc")
        || joined_lower.contains("by nc");

    // 2. CC BY-NC-SA (checked BEFORE CC BY-NC, CC BY-SA, and CC BY).
    if contains_cc_by_nc_sa(&joined_lower) {
        return LicenseKind::CcByNcSa;
    }

    // 3. CC BY-NC (checked BEFORE CC BY).
    if contains_cc_by_nc(&joined_lower) {
        return LicenseKind::CcByNc;
    }

    // 4. Explicit "All Rights Reserved".
    if joined_lower.contains("all rights reserved") {
        return LicenseKind::AllRightsReserved;
    }

    // 5. CC0 / Public Domain.
    if contains_cc0(&joined_lower) {
        return LicenseKind::Cc0;
    }

    // 6. MIT License.
    if contains_mit(&joined_lower, comment_lines) {
        return LicenseKind::Mit;
    }

    // 7. Apache-2.0.
    if contains_apache2(&joined_lower) {
        return LicenseKind::Apache2;
    }

    // 8. CC BY-SA (checked BEFORE generic CC BY; if a non-commercial qualifier is also
    //    present anywhere in the header, conservatively preserve `-NC`).
    if contains_cc_by_sa(&joined_lower) {
        return if has_noncommercial_qualifier {
            LicenseKind::CcByNcSa
        } else {
            LicenseKind::CcBySa
        };
    }

    // 9. Generic CC BY (checked LAST among CC licenses; preserve `-NC` if present).
    if contains_cc_by(&joined_lower) {
        return if has_noncommercial_qualifier {
            LicenseKind::CcByNc
        } else {
            LicenseKind::CcBy
        };
    }

    // 10. Conservative Shadertoy default when no explicit license is declared.
    LicenseKind::ShadertoyDefaultNc
}

/// Parses an explicit `SPDX-License-Identifier:` line, preserving `-NC` qualifiers.
fn parse_spdx_line(line: &str) -> Option<LicenseKind> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find("spdx-license-identifier:")?;
    let expr = lower[idx + "spdx-license-identifier:".len()..].trim();
    if expr.is_empty() {
        return None;
    }

    // Always check `-nc` variants before generic `cc-by` / `cc-by-sa`.
    if expr.contains("cc-by-nc-sa") {
        return Some(LicenseKind::CcByNcSa);
    }
    if expr.contains("cc-by-nc") {
        return Some(LicenseKind::CcByNc);
    }
    if expr.contains("cc0") || expr.contains("unlicense") {
        return Some(LicenseKind::Cc0);
    }
    if expr.contains("apache-2") {
        return Some(LicenseKind::Apache2);
    }
    if contains_word(expr, "mit") {
        return Some(LicenseKind::Mit);
    }
    if expr.contains("cc-by-sa") {
        return Some(LicenseKind::CcBySa);
    }
    if expr.contains("cc-by") {
        return Some(LicenseKind::CcBy);
    }

    Some(LicenseKind::Unknown)
}

fn contains_cc_by_nc_sa(lower: &str) -> bool {
    lower.contains("cc by-nc-sa")
        || lower.contains("cc-by-nc-sa")
        || lower.contains("cc by nc sa")
        || lower.contains("by-nc-sa")
        || lower.contains("attribution-noncommercial-sharealike")
        || lower.contains("attribution noncommercial sharealike")
        || lower.contains("attribution-non-commercial-sharealike")
        || lower.contains("creativecommons.org/licenses/by-nc-sa")
}

fn contains_cc_by_nc(lower: &str) -> bool {
    lower.contains("cc by-nc")
        || lower.contains("cc-by-nc")
        || lower.contains("cc by nc")
        || lower.contains("by-nc")
        || lower.contains("attribution-noncommercial")
        || lower.contains("attribution noncommercial")
        || lower.contains("attribution-non-commercial")
        || lower.contains("creativecommons.org/licenses/by-nc")
}

fn contains_cc0(lower: &str) -> bool {
    contains_word(lower, "cc0")
        || lower.contains("cc0-1.0")
        || lower.contains("cc-0")
        || lower.contains("creative commons zero")
        || lower.contains("public domain")
        || lower.contains("creativecommons.org/publicdomain/zero")
        || contains_word(lower, "unlicense")
}

fn contains_mit(lower: &str, comment_lines: &[String]) -> bool {
    if lower.contains("mit license")
        || lower.contains("mit licence")
        || lower.contains("licensed under mit")
        || lower.contains("licensed under the mit")
        || lower.contains("license: mit")
        || lower.contains("licence: mit")
        || lower.contains("license - mit")
        || lower.contains("permission is hereby granted, free of charge")
        || lower.contains("opensource.org/licenses/mit")
    {
        return true;
    }

    comment_lines.iter().any(|line| {
        let l = line.to_ascii_lowercase();
        (l.contains("license") || l.contains("licence")) && contains_word(&l, "mit")
    })
}

fn contains_apache2(lower: &str) -> bool {
    lower.contains("apache-2.0")
        || lower.contains("apache 2.0")
        || lower.contains("apache license, version 2.0")
        || lower.contains("apache license 2.0")
        || lower.contains("apache.org/licenses/license-2.0")
        || lower.contains("licensed under the apache license")
}

fn contains_cc_by_sa(lower: &str) -> bool {
    lower.contains("cc by-sa")
        || lower.contains("cc-by-sa")
        || lower.contains("cc by sa")
        || lower.contains("attribution-sharealike")
        || lower.contains("attribution sharealike")
        || lower.contains("creativecommons.org/licenses/by-sa")
}

fn contains_cc_by(lower: &str) -> bool {
    lower.contains("cc by")
        || lower.contains("cc-by")
        || lower.contains("creative commons attribution")
        || lower.contains("creativecommons.org/licenses/by/")
}

/// Checks whether `word` appears as a standalone ASCII-alphanumeric-bounded token in `haystack`.
fn contains_word(haystack: &str, word: &str) -> bool {
    let mut search_from = 0;
    while let Some(rel_idx) = haystack[search_from..].find(word) {
        let start = search_from + rel_idx;
        let end = start + word.len();

        let left_ok = start == 0
            || haystack[..start]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphanumeric());
        let right_ok = end == haystack.len()
            || haystack[end..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_alphanumeric());

        if left_ok && right_ok {
            return true;
        }
        search_from = end;
    }
    false
}

/// Attempts to extract an author name from header comment lines.
fn extract_author(comment_lines: &[String]) -> Option<String> {
    // 1. Explicit `Author:`, `Created by`, `Written by`, `Shader by` markers.
    for line in comment_lines {
        if let Some(name) = parse_labeled_author_line(line) {
            return Some(name);
        }
    }

    // 2. `Copyright (c) <year> <author>` lines.
    for line in comment_lines {
        if let Some(name) = parse_copyright_author_line(line) {
            return Some(name);
        }
    }

    // 3. Leading `by <author>` / `by: <author>` lines (avoiding `CC BY`, `licensed by`, etc.).
    for line in comment_lines {
        if let Some(name) = parse_leading_by_line(line) {
            return Some(name);
        }
    }

    None
}

fn parse_labeled_author_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    for prefix in [
        "author:",
        "created by:",
        "created by ",
        "written by:",
        "written by ",
        "shader by:",
        "shader by ",
    ] {
        if let Some(idx) = lower.find(prefix) {
            let candidate = &line[idx + prefix.len()..];
            if let Some(cleaned) = sanitize_author(candidate) {
                return Some(cleaned);
            }
        }
    }
    None
}

fn parse_copyright_author_line(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let idx = lower.find("copyright")?;
    let mut rest = line[idx + "copyright".len()..].trim_start();

    // Strip optional `(c)`, `(C)`, `©`, `:` tokens.
    loop {
        let trimmed = rest.trim_start();
        if let Some(after) = trimmed
            .strip_prefix("(c)")
            .or_else(|| trimmed.strip_prefix("(C)"))
            .or_else(|| trimmed.strip_prefix('©'))
            .or_else(|| trimmed.strip_prefix(':'))
        {
            rest = after;
        } else {
            rest = trimmed;
            break;
        }
    }

    // Strip leading year tokens such as `2013`, `2013-2024`, `2013, 2024`.
    loop {
        let trimmed = rest.trim_start_matches(|c: char| {
            c.is_whitespace() || c == ',' || c == '-' || c == '–' || c == '—'
        });
        let digit_count = trimmed.chars().take_while(char::is_ascii_digit).count();
        if digit_count == 4 {
            rest = &trimmed[4..];
        } else {
            rest = trimmed;
            break;
        }
    }

    // Strip optional `by ` immediately after the year.
    let lower_rest = rest.to_ascii_lowercase();
    if lower_rest.starts_with("by ") {
        rest = &rest[3..];
    }

    sanitize_author(rest)
}

fn parse_leading_by_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();

    let candidate = if lower.starts_with("by:") || lower.starts_with("by ") {
        &trimmed[3..]
    } else {
        return None;
    };

    sanitize_author(candidate)
}

/// Cleans trailing boilerplate (URLs, "All rights reserved", trailing punctuation) from an
/// extracted author string.
fn sanitize_author(raw: &str) -> Option<String> {
    let mut s = raw.trim();

    // Truncate at inline URLs or common trailing clauses.
    let lower = s.to_ascii_lowercase();
    for marker in [
        "all rights reserved",
        "http://",
        "https://",
        " - ",
        " | ",
        " -- ",
    ] {
        if let Some(idx) = lower.find(marker) {
            s = &s[..idx];
            break;
        }
    }

    let cleaned = s
        .trim()
        .trim_end_matches(['.', ',', ';', ':', '-', '—', '–'])
        .trim();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

/// Extracts the first `shadertoy.com/view/...` URL from header comments if present.
fn extract_shadertoy_url(comment_lines: &[String]) -> Option<String> {
    for line in comment_lines {
        for token in line.split_whitespace() {
            let trimmed = token.trim_matches(|c: char| {
                matches!(
                    c,
                    '(' | ')' | '[' | ']' | '<' | '>' | '"' | '\'' | ',' | ';'
                )
            });
            let lower = trimmed.to_ascii_lowercase();
            if (lower.starts_with("https://") || lower.starts_with("http://"))
                && lower.contains("shadertoy.com/view/")
            {
                return Some(trimmed.trim_end_matches('.').to_string());
            }
        }
    }
    None
}

fn clean_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

fn format_attribution_notice(
    license: LicenseKind,
    author: Option<&str>,
    source_url: Option<&str>,
) -> String {
    let author_label = author.unwrap_or("Unknown author");
    let license_label = license.spdx_or_label();
    match source_url {
        Some(url) => format!("Shader by {author_label} — License: {license_label} ({url})"),
        None => format!("Shader by {author_label} — License: {license_label}"),
    }
}

fn build_warning(license: LicenseKind) -> Option<String> {
    match license {
        LicenseKind::ShadertoyDefaultNc => Some(
            "No explicit license found in shader header comments. Shadertoy shaders default to \
             CC BY-NC-SA 3.0 (Non-Commercial, ShareAlike), which prohibits commercial use unless \
             the author explicitly grants a permissive license such as MIT, CC0, or Apache-2.0."
                .to_string(),
        ),
        LicenseKind::CcByNc | LicenseKind::CcByNcSa => Some(format!(
            "Shader is licensed under {} (Non-Commercial). Commercial use is not permitted \
             without separate permission from the author.",
            license.spdx_or_label()
        )),
        LicenseKind::AllRightsReserved => Some(
            "Shader declares All Rights Reserved. Commercial use and redistribution are not \
             permitted without explicit permission from the author."
                .to_string(),
        ),
        LicenseKind::Unknown => Some(
            "Shader license could not be determined. Verify licensing terms with the author \
             before commercial use."
                .to_string(),
        ),
        LicenseKind::Cc0
        | LicenseKind::Mit
        | LicenseKind::Apache2
        | LicenseKind::CcBy
        | LicenseKind::CcBySa => None,
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-IMPORT-SHADERTOY (T1) — Shadertoy header comment license & attribution auto-parser preserves -NC qualifiers and defaults conservatively to ShadertoyDefaultNc.

    use super::{LicenseKind, parse_shadertoy_license};

    #[test]
    fn parses_cc0_mrange_style_header() {
        let glsl = r#"
// CC0: Glowing neon tunnel
// Author: mrange
// https://www.shadertoy.com/view/7sX3zN
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(1.0);
}
"#;
        let report = parse_shadertoy_license(glsl, Some("fallback"), None);
        assert_eq!(report.license, LicenseKind::Cc0);
        assert_eq!(report.license.spdx_or_label(), "CC0-1.0");
        assert!(report.is_commercial_safe);
        assert!(!report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("mrange"));
        assert_eq!(
            report.source_url.as_deref(),
            Some("https://www.shadertoy.com/view/7sX3zN")
        );
        assert!(report.warning.is_none());
        assert!(report.attribution_notice.contains("mrange"));
        assert!(report.attribution_notice.contains("CC0-1.0"));
    }

    #[test]
    fn parses_mit_inigo_quilez_style_header() {
        let glsl = r#"
// The MIT License
// Copyright © 2013 Inigo Quilez
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software...
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.0);
}
"#;
        let url = "https://www.shadertoy.com/view/Xds3zN";
        let report = parse_shadertoy_license(glsl, None, Some(url));
        assert_eq!(report.license, LicenseKind::Mit);
        assert_eq!(report.license.spdx_or_label(), "MIT");
        assert!(report.is_commercial_safe);
        assert!(report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("Inigo Quilez"));
        assert_eq!(report.source_url.as_deref(), Some(url));
        assert!(report.warning.is_none());
        assert!(report.attribution_notice.contains("Inigo Quilez"));
        assert!(report.attribution_notice.contains("MIT"));
    }

    #[test]
    fn preserves_nc_qualifier_for_cc_by_nc_sa() {
        let glsl = r#"
/*
 * Plasma Waves
 * Created by ShaderAuthor
 * Licensed under CC BY-NC-SA 3.0
 */
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.5);
}
"#;
        let report = parse_shadertoy_license(glsl, None, None);
        assert_eq!(report.license, LicenseKind::CcByNcSa);
        assert_eq!(report.license.spdx_or_label(), "CC-BY-NC-SA");
        assert!(!report.is_commercial_safe);
        assert!(report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("ShaderAuthor"));
        let Some(warning) = report.warning.as_deref() else {
            panic!("expected non-commercial warning for CC BY-NC-SA");
        };
        assert!(warning.contains("Non-Commercial"));
        assert!(report.attribution_notice.contains("CC-BY-NC-SA"));
    }

    #[test]
    fn preserves_nc_qualifier_for_cc_by_nc_without_sa() {
        let glsl = r#"
// SPDX-License-Identifier: CC-BY-NC-4.0
// by Alex Rivera
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(0.2);
}
"#;
        let report = parse_shadertoy_license(glsl, None, None);
        assert_eq!(report.license, LicenseKind::CcByNc);
        assert_eq!(report.license.spdx_or_label(), "CC-BY-NC");
        assert!(!report.is_commercial_safe);
        assert!(report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("Alex Rivera"));
        assert!(report.warning.is_some());
    }

    #[test]
    fn parses_all_rights_reserved() {
        let glsl = r#"
// Copyright (c) 2024 Studio Example. All rights reserved.
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    fragColor = vec4(1.0);
}
"#;
        let report = parse_shadertoy_license(glsl, None, None);
        assert_eq!(report.license, LicenseKind::AllRightsReserved);
        assert_eq!(report.license.spdx_or_label(), "All Rights Reserved");
        assert!(!report.is_commercial_safe);
        assert!(report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("Studio Example"));
        let Some(warning) = report.warning.as_deref() else {
            panic!("expected warning for All Rights Reserved");
        };
        assert!(warning.contains("All Rights Reserved"));
    }

    #[test]
    fn defaults_conservatively_to_shadertoy_default_nc_when_unlabeled() {
        let glsl = r#"
// Simple rainbow gradient experiment
// Note: emit and limit variables must not trigger MIT detection
void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    float limit = 1.0;
    float emit = limit * 0.5;
    fragColor = vec4(emit);
}
"#;
        let report = parse_shadertoy_license(
            glsl,
            Some("CommunityArtist"),
            Some("https://www.shadertoy.com/view/abc123"),
        );
        assert_eq!(report.license, LicenseKind::ShadertoyDefaultNc);
        assert_eq!(
            report.license.spdx_or_label(),
            "CC-BY-NC-SA-3.0 (Shadertoy Default)"
        );
        assert!(!report.is_commercial_safe);
        assert!(report.requires_attribution);
        assert_eq!(report.author.as_deref(), Some("CommunityArtist"));
        assert_eq!(
            report.source_url.as_deref(),
            Some("https://www.shadertoy.com/view/abc123")
        );
        let Some(warning) = report.warning.as_deref() else {
            panic!("expected Shadertoy default non-commercial warning");
        };
        assert!(warning.contains("CC BY-NC-SA 3.0"));
        assert!(warning.contains("Non-Commercial"));
    }

    #[test]
    fn parses_apache2_and_commercial_cc_by_variants() {
        let apache_glsl = "// SPDX-License-Identifier: Apache-2.0\n// Author: Dev";
        let apache_report = parse_shadertoy_license(apache_glsl, None, None);
        assert_eq!(apache_report.license, LicenseKind::Apache2);
        assert!(apache_report.is_commercial_safe);
        assert!(apache_report.requires_attribution);
        assert!(apache_report.warning.is_none());

        let cc_by_glsl = "// Licensed under CC BY 4.0\n// Author: Artist";
        let cc_by_report = parse_shadertoy_license(cc_by_glsl, None, None);
        assert_eq!(cc_by_report.license, LicenseKind::CcBy);
        assert!(cc_by_report.is_commercial_safe);
        assert!(cc_by_report.requires_attribution);
        assert!(cc_by_report.warning.is_none());

        let cc_by_sa_glsl = "// Licensed under CC BY-SA 4.0\n// Author: Artist";
        let cc_by_sa_report = parse_shadertoy_license(cc_by_sa_glsl, None, None);
        assert_eq!(cc_by_sa_report.license, LicenseKind::CcBySa);
        assert!(cc_by_sa_report.is_commercial_safe);
        assert!(cc_by_sa_report.requires_attribution);
        assert!(cc_by_sa_report.warning.is_none());
    }
}
