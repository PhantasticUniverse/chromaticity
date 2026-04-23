# Supportability, Privacy, and Operator Runbook Policy

## Purpose
Define minimum operational support controls for commercial release readiness.

## Support Bundle Policy
- Support bundle export is opt-in per session unless explicit enterprise policy overrides it.
- Bundle minimum contents:
 - runtime version + profile metadata,
 - recent diagnostics and subsystem health events,
 - bounded performance/SLO snapshots,
 - adapter connectivity timeline,
 - anonymized crash identifiers (if present).
- Bundle exclusions by default:
 - raw camera frames,
 - raw audio streams,
 - full scene asset binaries unless explicitly selected.

## Telemetry and Log Privacy Policy
- Collect only telemetry needed for reliability/performance diagnosis.
- Redact or hash user-identifying fields in logs and telemetry.
- Retention policy:
 - local logs default retention: 30 days,
 - exported support bundles: user-controlled retention,
 - crash report payload retention follows legal/privacy policy baseline.
- Any telemetry schema change requires privacy impact checklist update.

## Crash Reporting Policy
- Crash reporting is opt-in by default for unmanaged installs.
- Report payload must exclude scene source and media payload content by default.
- Include reproducibility metadata only (build version, subsystem, stack, profile class).

## Operator Runbook Minimum
- Required runbooks:
 - startup and device preflight,
 - reconnect recovery procedure,
 - degraded-mode operation,
 - emergency fallback/black-frame policy,
 - show-state restore and rollback.
- Every runbook must include trigger conditions, step-by-step recovery actions, and abort/escalation rules.

## Evidence and Gate Binding
- `EV-SUPPORT-OPS-READINESS` validates:
 - support bundle policy conformance,
 - telemetry/log redaction checks,
 - crash-report policy verification,
 - runbook completeness checklist.
- `EV-SUPPORT-OPS-READINESS` is required for Phase 5/release gate.
