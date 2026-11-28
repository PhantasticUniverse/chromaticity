// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell-side live `script.js` host backed by `chromaticity-script`
//! (`rquickjs` QuickJS sandbox with ADR-012 watchdog + 16 MiB memory cap).
//!
//! Because `rquickjs::Runtime` is `!Send`, the host isolates QuickJS on a
//! dedicated lazily-spawned worker thread (`chromaticity-script-host`) and
//! communicates via bounded synchronous request/reply channels. [`AppState`]
//! remains `Send + Sync` with zero `unsafe` code.

use chromaticity_audio::{AudioUniformSnapshot, SynAudioEnvelopeBank, SynAudioEnvelopes};
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
        envelopes: SynAudioEnvelopes,
        on_beat: f32,
        beat_time: f32,
        reply: Sender<Result<ScriptTickOutput, String>>,
    },
    ControlChange {
        name: String,
        prev: f32,
        next: f32,
        reply: Sender<Result<ScriptTickOutput, String>>,
    },
    Clear,
    Shutdown,
}

/// Sanitize a floating-point scalar before interpolating into QuickJS source.
#[inline]
fn sanitize_js_f32(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Native scripting API (`inputs.*` / `uniforms.*` proxies plus
/// `setUniform(name, val)` and `chroma_*` / `TIME` globals) injected before
/// evaluating a scene's `script.js`.
const SCRIPT_API_PREAMBLE: &str = r#"
(function __chromaticityInstallScriptApi() {
  const __inputsBacking = Object.create(null);
  const __uniformsBacking = Object.create(null);

  function __safeEmit(name, val) {
    if (typeof name !== 'string' || name.length === 0) {
      return;
    }
    if (typeof val === 'number') {
      if (Number.isFinite(val)) {
        __uniformsBacking[name] = val;
        if (typeof __chromaticityEmit === 'function') {
          __chromaticityEmit(name, val);
        }
      }
      return;
    }
    if (typeof val === 'boolean') {
      const numeric = val ? 1.0 : 0.0;
      __uniformsBacking[name] = numeric;
      if (typeof __chromaticityEmit === 'function') {
        __chromaticityEmit(name, numeric);
      }
      return;
    }
    if (Array.isArray(val)) {
      const suffixes = ['_x', '_y', '_z', '_w'];
      for (let i = 0; i < val.length && i < suffixes.length; i++) {
        const elem = val[i];
        if (typeof elem === 'number' && Number.isFinite(elem)) {
          const key = name + suffixes[i];
          __uniformsBacking[key] = elem;
          if (typeof __chromaticityEmit === 'function') {
            __chromaticityEmit(key, elem);
          }
        }
      }
    }
  }

  const __handlers = {
    onChange: Object.create(null),
    onOffToOn: Object.create(null),
    onOnToOff: Object.create(null),
    whileOn: Object.create(null),
    whileOff: Object.create(null),
  };
  function __pushHandler(bucket, name, fn) {
    if (typeof name !== 'string' || typeof fn !== 'function') return;
    if (!bucket[name]) bucket[name] = [];
    bucket[name].push(fn);
  }
  globalThis.onChange = function onChange(name, fn) { __pushHandler(__handlers.onChange, name, fn); };
  globalThis.onOffToOn = function onOffToOn(name, fn) { __pushHandler(__handlers.onOffToOn, name, fn); };
  globalThis.onOnToOff = function onOnToOff(name, fn) { __pushHandler(__handlers.onOnToOff, name, fn); };
  globalThis.whileOn = function whileOn(name, fn) { __pushHandler(__handlers.whileOn, name, fn); };
  globalThis.whileOff = function whileOff(name, fn) { __pushHandler(__handlers.whileOff, name, fn); };
  globalThis.randomizeControl = function randomizeControl(name) {
    if (typeof name === 'string' && name.length > 0) {
      const val = Math.random();
      __inputsBacking[name] = val;
      __safeEmit('__ctrl_random_' + name, val);
    }
  };
  globalThis.defaultControl = function defaultControl(name) {
    if (typeof name === 'string' && name.length > 0) {
      __safeEmit('__ctrl_default_' + name, 1.0);
    }
  };
  globalThis.__chromaticityNotifyControl = function __chromaticityNotifyControl(name, prev, next) {
    __inputsBacking[name] = next;
    const list = __handlers.onChange[name];
    if (list) { for (let i = 0; i < list.length; i++) list[i](next, prev); }
    if (prev <= 0.5 && next > 0.5 && __handlers.onOffToOn[name]) {
      const a = __handlers.onOffToOn[name];
      for (let i = 0; i < a.length; i++) a[i](next);
    }
    if (prev > 0.5 && next <= 0.5 && __handlers.onOnToOff[name]) {
      const a = __handlers.onOnToOff[name];
      for (let i = 0; i < a.length; i++) a[i](next);
    }
  };
  globalThis.__chromaticityTickControlHandlers = function __chromaticityTickControlHandlers(dt) {
    for (const name in __handlers.whileOn) {
      if ((__inputsBacking[name] || 0) > 0.5) {
        const a = __handlers.whileOn[name];
        for (let i = 0; i < a.length; i++) a[i](dt);
      }
    }
    for (const name in __handlers.whileOff) {
      if ((__inputsBacking[name] || 0) <= 0.5) {
        const a = __handlers.whileOff[name];
        for (let i = 0; i < a.length; i++) a[i](dt);
      }
    }
  };

  globalThis.__chromaticitySyncGlobals = function __chromaticitySyncGlobals(
    elapsedSecs,
    frame,
    rms,
    peak,
    bpm,
    level,
    bassLevel,
    midLevel,
    midHighLevel,
    highLevel,
    hits,
    bassHits,
    midHits,
    midHighHits,
    highHits,
    presence,
    bassPresence,
    midPresence,
    midHighPresence,
    highPresence,
    audioTime,
    bassTime,
    midTime,
    midHighTime,
    highTime,
    fadeInOut,
    intensity,
    bpmConfidence,
    randomOnBeat,
    onBeat,
    beatTime
  ) {
    const map = {
      chroma_Time: elapsedSecs,
      chroma_Frame: frame,
      chroma_AudioRMS: rms,
      chroma_AudioPeak: peak,
      TIME: elapsedSecs,
      chroma_AudioTime: audioTime,
      chroma_BassTime: bassTime,
      chroma_MidTime: midTime,
      chroma_MidHighTime: midHighTime,
      chroma_HighTime: highTime,
      chroma_BPM: bpm,
      chroma_BPMConfidence: bpmConfidence,
      chroma_RandomOnBeat: randomOnBeat,
      chroma_FadeInOut: fadeInOut,
      chroma_Intensity: intensity,
      chroma_Level: level,
      chroma_Hits: hits,
      chroma_Presence: presence,
      chroma_BassLevel: bassLevel,
      chroma_BassHits: bassHits,
      chroma_BassPresence: bassPresence,
      chroma_MidLevel: midLevel,
      chroma_MidHits: midHits,
      chroma_MidPresence: midPresence,
      chroma_MidHighLevel: midHighLevel,
      chroma_MidHighHits: midHighHits,
      chroma_MidHighPresence: midHighPresence,
      chroma_HighLevel: highLevel,
      chroma_HighHits: highHits,
      chroma_HighPresence: highPresence,
      chroma_OnBeat: onBeat,
      chroma_BeatTime: beatTime,
    };
    for (const k in map) {
      globalThis[k] = map[k];
      __inputsBacking[k] = map[k];
    }
  };

  globalThis.__chromaticitySyncGlobals(
    0, 0, 0, 0, 120,
    0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
    0, 0, 0, 0, 0,
    1, 0.5, 0, 0.5, 0, 0
  );

  globalThis.setUniform = function setUniform(name, val) {
    __safeEmit(name, val);
  };

  if (typeof globalThis.print !== 'function') {
    globalThis.print = function print() {};
  }
  if (typeof globalThis.log !== 'function') {
    globalThis.log = function log() {};
  }

  globalThis.inputs = new Proxy(__inputsBacking, {
    get(target, prop) {
      if (typeof prop === 'symbol') {
        return target[prop];
      }
      if (Object.prototype.hasOwnProperty.call(target, prop)) {
        return target[prop];
      }
      const globalVal = globalThis[prop];
      if (typeof globalVal === 'number' && Number.isFinite(globalVal)) {
        return globalVal;
      }
      return 0.0;
    },
    set(target, prop, value) {
      if (typeof prop === 'string') {
        target[prop] = value;
      }
      return true;
    },
    has() {
      return true;
    }
  });

  globalThis.uniforms = new Proxy(__uniformsBacking, {
    get(target, prop) {
      if (typeof prop === 'symbol') {
        return target[prop];
      }
      if (Object.prototype.hasOwnProperty.call(target, prop)) {
        return target[prop];
      }
      return 0.0;
    },
    set(target, prop, value) {
      if (typeof prop === 'string') {
        target[prop] = value;
        __safeEmit(prop, value);
      }
      return true;
    }
  });
})();
"#;

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
        if !scalar.is_finite() {
            return Ok(());
        }
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
    envelopes: SynAudioEnvelopeBank,
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
            envelopes: SynAudioEnvelopeBank::new(),
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
        // Imported `.synScene` scripts reference reference-format names;
        // translate them to the native API (no-op for native scripts).
        let source = crate::syn_compat::adapt_syn_script(&source);
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
        let elapsed_secs = sanitize_js_f32(elapsed_secs);
        let dt = sanitize_js_f32(dt);
        let rms = sanitize_js_f32(audio.rms).clamp(0.0, 1.0);
        let peak = sanitize_js_f32(audio.peak).clamp(0.0, 1.0);
        let bpm = sanitize_js_f32(audio.bpm).max(0.0);
        let beat_time = if bpm > 0.0 {
            sanitize_js_f32(elapsed_secs * (bpm / 60.0))
        } else {
            0.0
        };
        let on_beat = if bpm > 0.0 && beat_time.fract() < 0.12 {
            1.0
        } else {
            0.0
        };
        let envelopes = self.envelopes.advance(audio, dt, beat_time);

        if tx
            .send(WorkerRequest::Tick {
                elapsed_secs,
                dt,
                frame,
                rms,
                peak,
                bpm,
                envelopes,
                on_beat,
                beat_time,
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

    /// Notify the active `script.js` of a control value transition so
    /// `onChange`, `onOffToOn`, and `onOnToOff` handlers execute.
    pub fn notify_control_change(
        &mut self,
        name: &str,
        prev: f32,
        next: f32,
    ) -> HashMap<String, f32> {
        if !matches!(self.state, ScriptHostState::Active { .. }) {
            return HashMap::new();
        }
        let Some(tx) = self.worker_tx.as_ref() else {
            return HashMap::new();
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if tx
            .send(WorkerRequest::ControlChange {
                name: name.to_string(),
                prev: sanitize_js_f32(prev),
                next: sanitize_js_f32(next),
                reply: reply_tx,
            })
            .is_err()
        {
            return HashMap::new();
        }
        match reply_rx.recv() {
            Ok(Ok(out)) => {
                self.last_emitted.extend(out.emitted.clone());
                out.emitted
            }
            _ => HashMap::new(),
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

                let init_source = format!(
                    "{SCRIPT_API_PREAMBLE}\n{source}\n;if (typeof setup === 'function') {{ setup(); }}\n"
                );
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
                envelopes,
                on_beat,
                beat_time,
                reply,
            } => {
                let Some((runtime, writes)) = active_runtime.as_ref() else {
                    let _ = reply.send(Ok(ScriptTickOutput::default()));
                    continue;
                };
                let tick_script = format!(
                    "globalThis.__chromaticitySyncGlobals(\
                        {elapsed_secs}, {frame}, {rms}, {peak}, {bpm}, \
                        {}, {}, {}, {}, {}, \
                        {}, {}, {}, {}, {}, \
                        {}, {}, {}, {}, {}, \
                        {}, {}, {}, {}, {}, \
                        {}, {}, {}, {}, \
                        {on_beat}, {beat_time}\
                     );\
                     globalThis.__chromaticityTickControlHandlers({dt});\
                     if (typeof update === 'function') {{ update({dt}); }}",
                    envelopes.level,
                    envelopes.bass_level,
                    envelopes.mid_level,
                    envelopes.mid_high_level,
                    envelopes.high_level,
                    envelopes.hits,
                    envelopes.bass_hits,
                    envelopes.mid_hits,
                    envelopes.mid_high_hits,
                    envelopes.high_hits,
                    envelopes.presence,
                    envelopes.bass_presence,
                    envelopes.mid_presence,
                    envelopes.mid_high_presence,
                    envelopes.high_presence,
                    envelopes.time,
                    envelopes.bass_time,
                    envelopes.mid_time,
                    envelopes.mid_high_time,
                    envelopes.high_time,
                    envelopes.fade_in_out,
                    envelopes.intensity,
                    envelopes.bpm_confidence,
                    envelopes.random_on_beat,
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
            WorkerRequest::ControlChange {
                name,
                prev,
                next,
                reply,
            } => {
                let Some((runtime, writes)) = active_runtime.as_ref() else {
                    let _ = reply.send(Ok(ScriptTickOutput::default()));
                    continue;
                };
                let escaped = serde_json::to_string(&name).unwrap_or_else(|_| "\"\"".into());
                let script =
                    format!("globalThis.__chromaticityNotifyControl({escaped}, {prev}, {next});");
                match runtime.execute(&script) {
                    Ok(_) => {
                        let emitted = drain_writes(writes);
                        let _ = reply.send(Ok(ScriptTickOutput { emitted }));
                    }
                    Err(err) => {
                        let _ = reply.send(Err(format!("script control handler failed: {err}")));
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

    #[test]
    fn script_host_set_uniform_and_globals_update_every_tick() {
        let mut host = ScriptHost::new();
        host.set_inline_script(
            "syn-modern".into(),
            r#"
            function update(dt) {
                setUniform('my_pulse', chroma_BassLevel);
                setUniform('bands_sum', chroma_MidLevel + chroma_MidHighLevel + chroma_HighLevel);
                setUniform('level_val', chroma_Level);
                setUniform('beat_val', chroma_BeatTime);
                setUniform('on_beat', chroma_OnBeat);
                setUniform('time_diff', Math.abs(chroma_AudioTime - TIME));
                setUniform('bpm_val', chroma_BPM);
                setUniform('vec_pair', [chroma_BassLevel, chroma_HighLevel]);
            }
            "#
            .into(),
        );
        assert_eq!(host.state_name(), "active");

        let audio = AudioUniformSnapshot {
            bpm: 120.0,
            rms: 0.55,
            peak: 0.85,
            fft_bands: [0.9, 0.7, 0.6, 0.4, 0.3, 0.2, 0.2, 0.1],
            ..AudioUniformSnapshot::default()
        };
        let out = host.tick(1.0, 1.0 / 60.0, 60, &audio);
        assert_eq!(host.state_name(), "active");
        assert!(host.status().error.is_none());

        assert!((out.get("my_pulse").copied().unwrap() - 0.8).abs() < 1e-4);
        // mid = 0.5, midHigh = 0.25, high = 0.15 => sum = 0.9
        assert!((out.get("bands_sum").copied().unwrap() - 0.9).abs() < 1e-4);
        assert!((out.get("level_val").copied().unwrap() - 0.55).abs() < 1e-4);
        // 1.0s at 120 BPM => 2.0 beats (exact integer => on_beat == 1.0)
        assert!((out.get("beat_val").copied().unwrap() - 2.0).abs() < 1e-4);
        assert!((out.get("on_beat").copied().unwrap() - 1.0).abs() < 1e-4);
        // chroma_AudioTime is energy-integrated (distinct from wall-clock TIME)
        assert!(out.get("time_diff").copied().unwrap() > 0.0);
        assert!((out.get("bpm_val").copied().unwrap() - 120.0).abs() < 1e-4);
        assert!((out.get("vec_pair_x").copied().unwrap() - 0.8).abs() < 1e-4);
        assert!((out.get("vec_pair_y").copied().unwrap() - 0.15).abs() < 1e-4);

        // Every emitted value must be finite.
        assert!(out.values().all(|v| v.is_finite()));
    }

    #[test]
    fn script_host_fires_control_event_handlers_and_real_envelopes() {
        let mut host = ScriptHost::new();
        host.set_inline_script(
            "syn-events".into(),
            r#"
            let changes = 0;
            let rises = 0;
            onChange('intensity', (next, prev) => {
                changes += 1;
                setUniform('delta', next - prev);
            });
            onOffToOn('flash', () => {
                rises += 1;
                setUniform('rises', rises);
            });
            function update(dt) {
                setUniform('bass_hits', chroma_BassHits);
                setUniform('bass_pres', chroma_BassPresence);
                setUniform('audio_t', chroma_AudioTime);
            }
            "#
            .into(),
        );
        let out = host.notify_control_change("intensity", 0.2, 0.75);
        assert!((out.get("delta").copied().unwrap_or(0.0) - 0.55).abs() < 1e-4);
        let out2 = host.notify_control_change("flash", 0.0, 1.0);
        assert!((out2.get("rises").copied().unwrap_or(0.0) - 1.0).abs() < 1e-4);

        let audio = AudioUniformSnapshot {
            bpm: 128.0,
            rms: 0.6,
            peak: 0.9,
            fft_bands: [0.9, 0.8, 0.5, 0.4, 0.3, 0.2, 0.1, 0.1],
            ..AudioUniformSnapshot::default()
        };
        let tick = host.tick(0.5, 1.0 / 60.0, 1, &audio);
        assert!(tick.get("bass_hits").copied().unwrap_or(0.0) > 0.0);
        assert!(tick.get("bass_pres").copied().unwrap_or(0.0) > 0.0);
        assert!(tick.get("audio_t").copied().unwrap_or(0.0) > 0.0);
    }
}
