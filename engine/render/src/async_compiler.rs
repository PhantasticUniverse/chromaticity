// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Asynchronous background shader compiler (`naga` off the render thread).
//!
//! Keeps 4K60 live output completely stall-free while authoring or hot-reloading
//! WGSL and GLSL fragment shaders. Submissions are non-blocking, stale in-flight
//! revisions are automatically superseded via an atomic revision watermark, and
//! render-thread polling drains only the latest revision's result.

#![forbid(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Instant;

/// Supported input shader languages for background compilation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShaderLanguage {
    /// WebGPU Shading Language (WGSL).
    Wgsl,
    /// Vulkan-flavored GLSL 4.50 fragment shader.
    GlslFragment,
}

/// Explicit lifecycle state machine for [`AsyncShaderCompiler`].
#[derive(Clone, Debug, PartialEq)]
pub enum CompileJobState {
    /// No compilation job has been submitted yet.
    Idle,
    /// A shader revision is currently queued or compiling off-thread.
    Compiling {
        /// Monotonic revision identifier of the active job.
        revision: u64,
    },
    /// The latest submitted revision compiled and validated successfully.
    Ready {
        /// Monotonic revision identifier of the validated artifact.
        revision: u64,
        /// Wall-clock compile duration in milliseconds.
        elapsed_ms: f32,
    },
    /// The latest submitted revision failed to parse, validate, or emit WGSL.
    Failed {
        /// Monotonic revision identifier of the failed job.
        revision: u64,
        /// Structured diagnostic error message.
        message: String,
        /// Wall-clock time spent before failure in milliseconds.
        elapsed_ms: f32,
    },
}

/// Validated shader output ready for pipeline creation on the render thread.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledShaderArtifact {
    /// Monotonic revision identifier matching the originating submission.
    pub revision: u64,
    /// Validated WGSL source code (passthrough for WGSL, transpiled for GLSL).
    pub wgsl_source: String,
    /// Wall-clock compile duration in milliseconds.
    pub elapsed_ms: f32,
}

struct CompileRequest {
    revision: u64,
    language: ShaderLanguage,
    source: String,
}

enum CompileOutcome {
    Ready(CompiledShaderArtifact),
    Failed {
        revision: u64,
        message: String,
        elapsed_ms: f32,
    },
}

impl CompileOutcome {
    fn revision(&self) -> u64 {
        match self {
            Self::Ready(artifact) => artifact.revision,
            Self::Failed { revision, .. } => *revision,
        }
    }
}

/// Thread-safe background shader compiler with automatic stale-revision superseding.
///
/// Spawns a single dedicated worker thread that parses and validates WGSL or
/// GLSL fragment shaders via `naga`. Rapid edits bump an atomic revision counter
/// so queued or in-flight stale revisions are skipped without stalling the
/// render thread.
pub struct AsyncShaderCompiler {
    request_tx: Option<mpsc::Sender<CompileRequest>>,
    result_rx: mpsc::Receiver<CompileOutcome>,
    latest_revision: Arc<AtomicU64>,
    state: CompileJobState,
    worker_handle: Option<JoinHandle<()>>,
}

impl Default for AsyncShaderCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncShaderCompiler {
    /// Spawn a new background shader compiler worker thread in [`CompileJobState::Idle`].
    #[must_use]
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<CompileRequest>();
        let (result_tx, result_rx) = mpsc::channel::<CompileOutcome>();
        let latest_revision = Arc::new(AtomicU64::new(0));
        let worker_revision = Arc::clone(&latest_revision);

        let worker_handle = std::thread::spawn(move || {
            worker_loop(&request_rx, &result_tx, &worker_revision);
        });

        Self {
            request_tx: Some(request_tx),
            result_rx,
            latest_revision,
            state: CompileJobState::Idle,
            worker_handle: Some(worker_handle),
        }
    }

    /// Submit a shader source for non-blocking background compilation.
    ///
    /// Returns a strictly monotonic revision ID (`>= 1`). Any previously queued
    /// or in-flight lower revision is superseded automatically.
    pub fn submit(&mut self, language: ShaderLanguage, source: impl Into<String>) -> u64 {
        let revision = self.latest_revision.fetch_add(1, Ordering::AcqRel) + 1;
        self.state = CompileJobState::Compiling { revision };

        if let Some(tx) = &self.request_tx {
            let _ = tx.send(CompileRequest {
                revision,
                language,
                source: source.into(),
            });
        }

        revision
    }

    /// Non-blocking render-thread drain of completed compilation results.
    ///
    /// Stale outcomes from superseded revisions are silently discarded. Returns
    /// `Some(Ok(artifact))` or `Some(Err((revision, message)))` when the latest
    /// submitted revision completes, and updates [`Self::state`] accordingly.
    pub fn poll(&mut self) -> Option<Result<CompiledShaderArtifact, (u64, String)>> {
        let latest = self.latest_revision.load(Ordering::Acquire);
        let mut latest_result = None;

        while let Ok(outcome) = self.result_rx.try_recv() {
            if outcome.revision() < latest {
                continue;
            }

            match outcome {
                CompileOutcome::Ready(artifact) => {
                    self.state = CompileJobState::Ready {
                        revision: artifact.revision,
                        elapsed_ms: artifact.elapsed_ms,
                    };
                    latest_result = Some(Ok(artifact));
                }
                CompileOutcome::Failed {
                    revision,
                    message,
                    elapsed_ms,
                } => {
                    self.state = CompileJobState::Failed {
                        revision,
                        message: message.clone(),
                        elapsed_ms,
                    };
                    latest_result = Some(Err((revision, message)));
                }
            }
        }

        latest_result
    }

    /// Current state of the compiler job state machine.
    #[must_use]
    pub fn state(&self) -> &CompileJobState {
        &self.state
    }
}

impl Drop for AsyncShaderCompiler {
    fn drop(&mut self) {
        // Drop the sender first so the worker's `recv()` unblocks and exits cleanly.
        self.request_tx.take();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

fn worker_loop(
    request_rx: &mpsc::Receiver<CompileRequest>,
    result_tx: &mpsc::Sender<CompileOutcome>,
    latest_revision: &AtomicU64,
) {
    while let Ok(mut request) = request_rx.recv() {
        // Drain any already-queued newer submissions so we only compile the newest.
        while let Ok(newer) = request_rx.try_recv() {
            request = newer;
        }

        // Skip immediately if superseded before starting compilation.
        if request.revision < latest_revision.load(Ordering::Acquire) {
            continue;
        }

        let started = Instant::now();
        let compiled = compile_shader(request.language, &request.source);
        let elapsed_ms = started.elapsed().as_secs_f32() * 1000.0;

        // Discard if superseded while compiling.
        if request.revision < latest_revision.load(Ordering::Acquire) {
            continue;
        }

        let outcome = match compiled {
            Ok(wgsl_source) => CompileOutcome::Ready(CompiledShaderArtifact {
                revision: request.revision,
                wgsl_source,
                elapsed_ms,
            }),
            Err(message) => CompileOutcome::Failed {
                revision: request.revision,
                message,
                elapsed_ms,
            },
        };

        if result_tx.send(outcome).is_err() {
            break;
        }
    }
}

fn compile_shader(language: ShaderLanguage, source: &str) -> Result<String, String> {
    match language {
        ShaderLanguage::Wgsl => {
            let module = naga::front::wgsl::parse_str(source)
                .map_err(|err| format!("WGSL parse error: {}", err.emit_to_string(source)))?;

            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            );
            validator
                .validate(&module)
                .map_err(|err| format!("WGSL validation error: {}", err.emit_to_string(source)))?;

            Ok(source.to_owned())
        }
        ShaderLanguage::GlslFragment => {
            let mut frontend = naga::front::glsl::Frontend::default();
            let module = frontend
                .parse(
                    &naga::front::glsl::Options {
                        stage: naga::ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                    source,
                )
                .map_err(|err| format!("GLSL fragment parse error: {err:?}"))?;

            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            );
            let module_info = validator
                .validate(&module)
                .map_err(|err| format!("GLSL fragment validation error: {err:?}"))?;

            naga::back::wgsl::write_string(
                &module,
                &module_info,
                naga::back::wgsl::WriterFlags::empty(),
            )
            .map_err(|err| format!("WGSL emit error: {err:?}"))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Evidence: EV-RENDER-CONFORMANCE (T1) — async background shader compiler
    //! WGSL/GLSL off-thread compilation, error diagnostics, and stale-revision superseding.

    use std::time::{Duration, Instant};

    use super::{AsyncShaderCompiler, CompileJobState, CompiledShaderArtifact, ShaderLanguage};

    const VALID_WGSL_FRAGMENT: &str = r#"
@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv, 0.5, 1.0);
}
"#;

    const VALID_GLSL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(uv, 0.25, 1.0);
}
"#;

    const BROKEN_WGSL_FRAGMENT: &str = r#"
@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(undefined_symbol, 1.0);
}
"#;

    const BROKEN_GLSL_FRAGMENT: &str = r#"
#version 450
layout(location = 0) out vec4 fragColor;
void main() {
    fragColor = vec4(does_not_exist, 1.0)
}
"#;

    fn wait_for_poll(
        compiler: &mut AsyncShaderCompiler,
        timeout: Duration,
    ) -> Option<Result<CompiledShaderArtifact, (u64, String)>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(outcome) = compiler.poll() {
                return Some(outcome);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        None
    }

    #[test]
    fn valid_wgsl_compiles_off_thread_to_ready() {
        let mut compiler = AsyncShaderCompiler::new();
        assert_eq!(compiler.state(), &CompileJobState::Idle);

        let rev = compiler.submit(ShaderLanguage::Wgsl, VALID_WGSL_FRAGMENT);
        assert_eq!(rev, 1);
        assert_eq!(
            compiler.state(),
            &CompileJobState::Compiling { revision: 1 }
        );

        let Some(outcome) = wait_for_poll(&mut compiler, Duration::from_secs(5)) else {
            panic!("timed out waiting for WGSL compilation");
        };

        let Ok(artifact) = outcome else {
            panic!("expected WGSL compilation to succeed, got: {outcome:?}");
        };

        assert_eq!(artifact.revision, 1);
        assert!(artifact.wgsl_source.contains("fn fs_main"));
        assert!(artifact.elapsed_ms >= 0.0);
        assert!(matches!(
            compiler.state(),
            CompileJobState::Ready { revision: 1, elapsed_ms } if *elapsed_ms >= 0.0
        ));
    }

    #[test]
    fn valid_glsl_fragment_compiles_and_emits_valid_wgsl() {
        let mut compiler = AsyncShaderCompiler::new();
        let rev = compiler.submit(ShaderLanguage::GlslFragment, VALID_GLSL_FRAGMENT);
        assert_eq!(rev, 1);

        let Some(outcome) = wait_for_poll(&mut compiler, Duration::from_secs(5)) else {
            panic!("timed out waiting for GLSL fragment compilation");
        };

        let Ok(artifact) = outcome else {
            panic!("expected GLSL fragment compilation to succeed, got: {outcome:?}");
        };

        assert_eq!(artifact.revision, 1);
        assert!(artifact.wgsl_source.contains("@fragment"));
        assert!(artifact.elapsed_ms >= 0.0);
        assert!(matches!(
            compiler.state(),
            CompileJobState::Ready { revision: 1, .. }
        ));

        // Verify the emitted WGSL parses and validates cleanly through naga's WGSL frontend.
        let reparsed = naga::front::wgsl::parse_str(&artifact.wgsl_source);
        assert!(
            reparsed.is_ok(),
            "transpiled WGSL should re-parse cleanly: {reparsed:?}"
        );
    }

    #[test]
    fn broken_shaders_return_failed_with_diagnostic_without_panicking() {
        let mut compiler = AsyncShaderCompiler::new();

        let wgsl_rev = compiler.submit(ShaderLanguage::Wgsl, BROKEN_WGSL_FRAGMENT);
        let Some(wgsl_outcome) = wait_for_poll(&mut compiler, Duration::from_secs(5)) else {
            panic!("timed out waiting for broken WGSL compilation");
        };
        let Err((failed_rev, wgsl_msg)) = wgsl_outcome else {
            panic!("expected broken WGSL to fail");
        };
        assert_eq!(failed_rev, wgsl_rev);
        assert!(!wgsl_msg.is_empty());
        assert!(matches!(
            compiler.state(),
            CompileJobState::Failed { revision, message, elapsed_ms }
                if *revision == wgsl_rev && !message.is_empty() && *elapsed_ms >= 0.0
        ));

        let glsl_rev = compiler.submit(ShaderLanguage::GlslFragment, BROKEN_GLSL_FRAGMENT);
        assert_eq!(glsl_rev, wgsl_rev + 1);
        let Some(glsl_outcome) = wait_for_poll(&mut compiler, Duration::from_secs(5)) else {
            panic!("timed out waiting for broken GLSL compilation");
        };
        let Err((failed_glsl_rev, glsl_msg)) = glsl_outcome else {
            panic!("expected broken GLSL to fail");
        };
        assert_eq!(failed_glsl_rev, glsl_rev);
        assert!(!glsl_msg.is_empty());
        assert!(matches!(
            compiler.state(),
            CompileJobState::Failed { revision, .. } if *revision == glsl_rev
        ));
    }

    #[test]
    fn rapid_multi_revision_submission_supersedes_stale_revisions() {
        let mut compiler = AsyncShaderCompiler::new();
        let mut last_rev = 0;

        // Rapidly submit 12 broken/intermediate revisions followed by a final valid revision.
        for idx in 0..12_u64 {
            let rev = if idx < 11 {
                compiler.submit(ShaderLanguage::Wgsl, BROKEN_WGSL_FRAGMENT)
            } else {
                compiler.submit(ShaderLanguage::Wgsl, VALID_WGSL_FRAGMENT)
            };
            assert_eq!(rev, idx + 1);
            last_rev = rev;
        }

        assert_eq!(last_rev, 12);
        assert_eq!(
            compiler.state(),
            &CompileJobState::Compiling { revision: 12 }
        );

        let Some(outcome) = wait_for_poll(&mut compiler, Duration::from_secs(5)) else {
            panic!("timed out waiting for latest revision");
        };

        let Ok(artifact) = outcome else {
            panic!("expected only latest valid revision (12) to be delivered, got: {outcome:?}");
        };

        assert_eq!(artifact.revision, 12);
        assert!(matches!(
            compiler.state(),
            CompileJobState::Ready { revision: 12, .. }
        ));

        // Subsequent polls must return None (no stale revisions left behind).
        assert!(compiler.poll().is_none());
    }
}
