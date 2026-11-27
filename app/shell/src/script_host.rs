// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side live `script.js` host backed by `chromaticity-script`
//! (`rquickjs` QuickJS sandbox with ADR-012 watchdog + 16 MiB memory cap).
//!
//! Because `rquickjs::Runtime` is `!Send`, the host isolates QuickJS on a
//! dedicated lazily-spawned worker thread (`chromaticity-script-host`) and
//! communicates via bounded synchronous request/reply channels. [`AppState`]
//! remains `Send + Sync` with zero `unsafe` code.

use chromaticity_audio::AudioUniformSnapshot;
use chromaticity_script::{
    ScriptCommand, ScriptCommandError, ScriptCommandSender, ScriptRuntime, ScriptValue,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Explicit lifecycle state of the shell `script.js` host.
#[derive(Debug, Clone)]
pub enum ScriptHostState {
    /// No scene loaded, or the loaded scene has no `script.js`.
    Idle,
    /// A scene's `script.js` is loaded inside QuickJS and executing per tick.
    Active {
        scene_id: String,
        ticks_executed: u64,
        last_emitted_count: usize,
    },
    /// The scene's `script.js` failed to parse, timed out, or threw at runtime.
    Error { scene_id: String, message: String },
}

/// Serializable status snapshot surfaced to `inspect_state_machines` and UI.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptHostStatus {
    pub state: &'static str,
    pub scene_id: Option<String>,
    pub ticks_executed: u64,
    pub last_emitted_count: usize,
    pub error: Option<String>,
}

/// Reactive scalar writes emitted by `script.js` via `__chromaticityEmit`.
#[derive(Debug, Default, Clone)]
pub struct ScriptTickOutput {
    pub emitted: HashMap<String, f32>,
}

enum WorkerRequest {
    Load {
        source: String,
        reply: Sender<Result<ScriptTickOutput, String>>,
    },
    Tick {
        elapsed_secs: f32,
        dt: f32,
        frame: u64,
        rms: f32,
        peak: f32,
        bpm: f32,
        reply: Sender<Result<ScriptTickOutput, String>>,
    },
    Clear,
    Shutdown,
}

struct CollectingSender {
    writes: Arc<Mutex<HashMap<String, f32>>>,
}

impl ScriptCommandSender for CollectingSender {
    fn send(&self, cmd: ScriptCommand) -> Result<(), ScriptCommandError> {
        let (name, value) = match cmd {
            ScriptCommand::ReactiveWrite { name, value } => (name, value),
            ScriptCommand::TypedReactiveWrite { symbol, value } => {
                (symbol.name().to_string(), value)
            }
            _ => return Ok(()),
        };
        let scalar = match value {
            ScriptValue::Number(v) => v as f32,
            ScriptValue::Bool(b) => {
                if b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => return Ok(()),
        };
        if let Ok(mut map) = self.writes.lock() {
            map.insert(name, scalar);
        }
        Ok(())
    }
}

/// Long-lived shell `script.js` host owned by [`crate::state::AppState`].
pub struct ScriptHost {
    state: ScriptHostState,
    worker_tx: Option<Sender<WorkerRequest>>,
    worker_join: Option<JoinHandle<()>>,
    last_emitted: HashMap<String, f32>,
}

impl Default for ScriptHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptHost {
    /// Construct an idle script host (no worker thread spawned until a
    /// scene with `script.js` is loaded).
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: ScriptHostState::Idle,
            worker_tx: None,
            worker_join: None,
            last_emitted: HashMap::new(),
        }
    }

    /// Arm the script host from a `.chroma` scene directory.
    ///
    /// If `<scene_dir>/script.js` exists, spawns the QuickJS worker thread
    /// (if needed), evaluates the script, and calls `setup()` if defined.
    pub fn set_scene(&mut self, scene_id: String, scene_dir: &Path) {
        let script_path = scene_dir.join("script.js");
        if !script_path.is_file() {
            self.clear_scene();
            return;
        }
        match std::fs::read_to_string(&script_path) {
            Ok(source) => self.set_inline_script(scene_id, source),
            Err(err) => {
                self.last_emitted.clear();
                self.state = ScriptHostState::Error {
                    scene_id,
                    message: format!("failed to read {}: {err}", script_path.display()),
                };
            }
        }
    }

    /// Arm an in-memory `script.js` source directly (used by tests and live Author).
    pub fn set_inline_script(&mut self, scene_id: String, source: String) {
        self.last_emitted.clear();
        if let Err(err) = self.ensure_worker() {
            self.state = ScriptHostState::Error {
                scene_id,
                message: err,
            };
            return;
        }
        let Some(tx) = self.worker_tx.as_ref() else {
            self.state = ScriptHostState::Error {
                scene_id,
                message: "script host worker channel unavailable".into(),
            };
            return;
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx
            .send(WorkerRequest::Load {
                source,
                reply: reply_tx,
            })
            .is_err()
        {
            self.state = ScriptHostState::Error {
                scene_id,
                message: "script host worker disconnected".into(),
            };
            return;
        }
        match reply_rx.recv() {
            Ok(Ok(out)) => {
                let count = out.emitted.len();
                self.last_emitted = out.emitted;
                self.state = ScriptHostState::Active {
                    scene_id,
                    ticks_executed: 0,
                    last_emitted_count: count,
                };
            }
            Ok(Err(err)) => {
                self.state = ScriptHostState::Error {
                    scene_id,
                    message: err,
                };
            }
            Err(_) => {
                self.state = ScriptHostState::Error {
                    scene_id,
                    message: "script host worker reply channel closed".into(),
                };
            }
        }
    }

    /// Disarm the script host back to [`ScriptHostState::Idle`].
    pub fn clear_scene(&mut self) {
        self.last_emitted.clear();
        if let Some(tx) = self.worker_tx.as_ref() {
            let _ = tx.send(WorkerRequest::Clear);
        }
        self.state = ScriptHostState::Idle;
    }

    /// Execute one frame tick of the active `script.js` (`update(dt)` if defined).
    ///
    /// Returns the map of reactive scalar values emitted during this tick.
    /// When idle or errored, returns an empty map without erroring.
    pub fn tick(
        &mut self,
        elapsed_secs: f32,
        dt: f32,
        frame: u64,
        audio: &AudioUniformSnapshot,
    ) -> HashMap<String, f32> {
        let scene_id = match &self.state {
            ScriptHostState::Active { scene_id, .. } => scene_id.clone(),
            ScriptHostState::Idle | ScriptHostState::Error { .. } => return HashMap::new(),
        };
        let Some(tx) = self.worker_tx.as_ref() else {
            return HashMap::new();
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx
            .send(WorkerRequest::Tick {
                elapsed_secs,
                dt,
                frame,
                rms: audio.rms,
                peak: audio.peak,
                bpm: audio.bpm,
                reply: reply_tx,
            })
            .is_err()
        {
            self.state = ScriptHostState::Error {
                scene_id,
                message: "script host worker disconnected".into(),
            };
            return HashMap::new();
        }
        match reply_rx.recv() {
            Ok(Ok(out)) => {
                let count = out.emitted.len();
                self.last_emitted.extend(out.emitted.clone());
                if let ScriptHostState::Active {
                    ticks_executed,
                    last_emitted_count,
                    ..
                } = &mut self.state
                {
                    *ticks_executed = ticks_executed.saturating_add(1);
                    *last_emitted_count = count;
                }
                out.emitted
            }
            Ok(Err(err)) => {
                self.state = ScriptHostState::Error {
                    scene_id,
                    message: err,
                };
                HashMap::new()
            }
            Err(_) => {
                self.state = ScriptHostState::Error {
                    scene_id,
                    message: "script host worker reply channel closed".into(),
                };
                HashMap::new()
            }
        }
    }

    /// Read-only status snapshot for `inspect_state_machines`.
    #[must_use]
    pub fn status(&self) -> ScriptHostStatus {
        match &self.state {
            ScriptHostState::Idle => ScriptHostStatus {
                state: "idle",
                scene_id: None,
                ticks_executed: 0,
                last_emitted_count: 0,
                error: None,
            },
            ScriptHostState::Active {
                scene_id,
                ticks_executed,
                last_emitted_count,
            } => ScriptHostStatus {
                state: "active",
                scene_id: Some(scene_id.clone()),
                ticks_executed: *ticks_executed,
                last_emitted_count: *last_emitted_count,
                error: None,
            },
            ScriptHostState::Error { scene_id, message } => ScriptHostStatus {
                state: "error",
                scene_id: Some(scene_id.clone()),
                ticks_executed: 0,
                last_emitted_count: 0,
                error: Some(message.clone()),
            },
        }
    }

    /// Short state tag for `inspect_state_machines`.
    #[must_use]
    pub fn state_name(&self) -> &'static str {
        match &self.state {
            ScriptHostState::Idle => "idle",
            ScriptHostState::Active { .. } => "active",
            ScriptHostState::Error { .. } => "error",
        }
    }

    /// Last accumulated emitted values from `script.js`.
    #[must_use]
    pub fn last_emitted(&self) -> &HashMap<String, f32> {
        &self.last_emitted
    }

    fn ensure_worker(&mut self) -> Result<(), String> {
        if self.worker_tx.is_some() {
            return Ok(());
        }
        let (tx, rx) = mpsc::channel();
        let join = std::thread::Builder::new()
            .name("chromaticity-script-host".into())
            .spawn(move || run_script_worker(rx))
            .map_err(|err| format!("failed to spawn script host worker: {err}"))?;
        self.worker_tx = Some(tx);
        self.worker_join = Some(join);
        Ok(())
    }
}

impl Drop for ScriptHost {
    fn drop(&mut self) {
        if let Some(tx) = self.worker_tx.take() {
            let _ = tx.send(WorkerRequest::Shutdown);
        }
        if let Some(join) = self.worker_join.take() {
            let _ = join.join();
        }
    }
}

type SharedWriteMap = Arc<Mutex<HashMap<String, f32>>>;

fn run_script_worker(rx: Receiver<WorkerRequest>) {
    let mut active_runtime: Option<(ScriptRuntime, SharedWriteMap)> = None;

    while let Ok(req) = rx.recv() {
        match req {
            WorkerRequest::Load { source, reply } => {
                let writes = Arc::new(Mutex::new(HashMap::new()));
                let sender = Arc::new(CollectingSender {
                    writes: Arc::clone(&writes),
                });
                let runtime_res = ScriptRuntime::new()
                    .and_then(|rt| rt.with_watchdog(1, 4))
                    .and_then(|rt| rt.with_memory_limit(16 * 1024 * 1024))
                    .and_then(|rt| rt.with_command_sender(sender));
                let runtime = match runtime_res {
                    Ok(rt) => rt,
                    Err(err) => {
                        active_runtime = None;
                        let _ = reply.send(Err(format!("script runtime init failed: {err}")));
                        continue;
                    }
                };

                let init_source =
                    format!("{source}\n;if (typeof setup === 'function') {{ setup(); }}\n");
                match runtime.execute(&init_source) {
                    Ok(_) => {
                        let emitted = drain_writes(&writes);
                        active_runtime = Some((runtime, writes));
                        let _ = reply.send(Ok(ScriptTickOutput { emitted }));
                    }
                    Err(err) => {
                        active_runtime = None;
                        let _ = reply.send(Err(format!("script load failed: {err}")));
                    }
                }
            }
            WorkerRequest::Tick {
                elapsed_secs,
                dt,
                frame,
                rms,
                peak,
                bpm,
                reply,
            } => {
                let Some((runtime, writes)) = active_runtime.as_ref() else {
                    let _ = reply.send(Ok(ScriptTickOutput::default()));
                    continue;
                };
                let tick_script = format!(
                    "globalThis.chroma_Time = {elapsed_secs};\
                     globalThis.chroma_Frame = {frame};\
                     globalThis.chroma_AudioRMS = {rms};\
                     globalThis.chroma_AudioPeak = {peak};\
                     globalThis.syn_BPM = {bpm};\
                     if (typeof update === 'function') {{ update({dt}); }}"
                );
                match runtime.execute(&tick_script) {
                    Ok(_) => {
                        let emitted = drain_writes(writes);
                        let _ = reply.send(Ok(ScriptTickOutput { emitted }));
                    }
                    Err(err) => {
                        active_runtime = None;
                        let _ = reply.send(Err(format!("script update failed: {err}")));
                    }
                }
            }
            WorkerRequest::Clear => {
                active_runtime = None;
            }
            WorkerRequest::Shutdown => {
                break;
            }
        }
    }
}

fn drain_writes(writes: &Arc<Mutex<HashMap<String, f32>>>) -> HashMap<String, f32> {
    match writes.lock() {
        Ok(mut map) => std::mem::take(&mut *map),
        Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
    }
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod script_host_pins {
    use super::*;

    #[test]
    fn script_host_executes_setup_and_update_and_collects_emits() {
        let mut host = ScriptHost::new();
        assert_eq!(host.state_name(), "idle");

        host.set_inline_script(
            "test-script".into(),
            r#"
            let phase = 0;
            function setup() {
                __chromaticityEmit("ctrl_0", 0.25);
            }
            function update(dt) {
                phase += dt;
                __chromaticityEmit("ctrl_0", Math.min(1.0, 0.25 + phase));
            }
            "#
            .into(),
        );
        assert_eq!(host.state_name(), "active");
        assert!((host.last_emitted().get("ctrl_0").copied().unwrap_or(0.0) - 0.25).abs() < 1e-4);

        let out = host.tick(0.5, 0.5, 1, &AudioUniformSnapshot::default());
        assert!((out.get("ctrl_0").copied().unwrap_or(0.0) - 0.75).abs() < 1e-4);
        assert_eq!(host.status().ticks_executed, 1);
    }

    #[test]
    fn script_host_syntax_error_transitions_to_error_without_panicking() {
        let mut host = ScriptHost::new();
        host.set_inline_script("broken".into(), "function (( {".into());
        assert_eq!(host.state_name(), "error");
        assert!(host.status().error.is_some());
    }
}
