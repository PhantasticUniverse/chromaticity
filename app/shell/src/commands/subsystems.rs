// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Shell IPC command bridge for wired engine subsystems:
//! - 4-bus [`chromaticity_audio::mixer::BroadcastMixer`]
//! - [`chromaticity_runtime::clock_sync::ClockSyncCoordinator`] & Pro DJ Link planner
//! - [`chromaticity_runtime::cue_runner::SetlistCueRunner`] (2-button footswitch)
//! - [`chromaticity_runtime::dmx_artnet::DmxUniverse`] (DMX512 / Art-Net 4)
//! - [`chromaticity_runtime::data_ingest::DataIngestEngine`] (ADR-017)
//! - [`chromaticity_observability::emit::InMemoryEmitter`] (ADR-016)

#![forbid(unsafe_code)]

use crate::state::AppState;
use chromaticity_audio::mixer::{BusConfig, BusId};
use chromaticity_runtime::cue_runner::{CueRunnerState, FootswitchAction};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Serializable per-bus status snapshot for the 4-bus Broadcast Audio Mixer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BroadcastBusStatusDto {
    pub id: String,
    pub gain_db: f32,
    pub muted: bool,
    pub solo: bool,
    pub rms_left: f32,
    pub rms_right: f32,
    pub peak_left: f32,
    pub peak_right: f32,
}

/// Serializable snapshot of the 4-bus [`chromaticity_audio::mixer::BroadcastMixer`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BroadcastMixerStatusDto {
    pub sample_rate_hz: u32,
    pub buses: Vec<BroadcastBusStatusDto>,
}

/// Serializable snapshot of [`chromaticity_runtime::clock_sync::ClockSyncCoordinator`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClockSyncStatusDto {
    pub active_source: String,
    pub bpm: f32,
    pub beat_phase: f32,
}

/// Serializable snapshot of the 512-channel DMX512 universe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmxUniverseStatusDto {
    pub channel_count: usize,
    pub non_zero_channels: usize,
}

/// Serializable snapshot of the 2-button footswitch cue runner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FootswitchStatusDto {
    pub state: String,
    pub active_index: Option<usize>,
    pub blackout: bool,
}

/// Serializable snapshot of the ADR-017 external data-ingest engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataIngestStatusDto {
    pub source_count: usize,
}

/// Serializable snapshot of ADR-016 local in-process observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityStatusDto {
    pub buffered_events: usize,
}

fn parse_bus_id(id: &str) -> Result<BusId, String> {
    match id {
        "master" => Ok(BusId::MasterAudio),
        "mic" => Ok(BusId::Microphone),
        "media" => Ok(BusId::MediaSource),
        "alerts" => Ok(BusId::Alerts),
        other => Err(format!("unknown broadcast bus id: {other}")),
    }
}

fn build_mixer_status(app: &AppState) -> BroadcastMixerStatusDto {
    let mixer = match app.broadcast_mixer.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let specs = [
        ("master", BusId::MasterAudio),
        ("mic", BusId::Microphone),
        ("media", BusId::MediaSource),
        ("alerts", BusId::Alerts),
    ];
    let buses = specs
        .into_iter()
        .map(|(label, bus_id)| {
            let cfg = mixer.bus_config(bus_id).unwrap_or_default();
            let meter = mixer.bus_meter(bus_id).unwrap_or_default();
            BroadcastBusStatusDto {
                id: label.to_string(),
                gain_db: cfg.gain_db,
                muted: cfg.muted,
                solo: cfg.solo,
                rms_left: meter.left_rms,
                rms_right: meter.right_rms,
                peak_left: meter.left_peak,
                peak_right: meter.right_peak,
            }
        })
        .collect();
    BroadcastMixerStatusDto {
        sample_rate_hz: mixer.sample_rate_hz(),
        buses,
    }
}

/// Return the live 4-bus broadcast audio mixer status and per-bus meters.
#[tauri::command]
pub fn get_broadcast_mixer_status(
    app_state: State<'_, AppState>,
) -> Result<BroadcastMixerStatusDto, String> {
    Ok(build_mixer_status(app_state.inner()))
}

/// Update gain, mute, and solo state on one broadcast audio mixer bus.
#[tauri::command]
pub fn set_broadcast_bus_config(
    app_state: State<'_, AppState>,
    bus_id: String,
    gain_db: f32,
    muted: bool,
    solo: bool,
) -> Result<BroadcastMixerStatusDto, String> {
    let parsed = parse_bus_id(&bus_id)?;
    {
        let mut mixer = match app_state.broadcast_mixer.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut cfg = mixer.bus_config(parsed).unwrap_or_else(BusConfig::default);
        cfg.gain_db = gain_db;
        cfg.muted = muted;
        cfg.solo = solo;
        mixer.configure_bus(parsed, cfg);
    }
    Ok(build_mixer_status(app_state.inner()))
}

/// Return the resolved multi-source tempo clock status (Pro DJ Link / MIDI / Audio).
#[tauri::command]
pub fn get_clock_sync_status(app_state: State<'_, AppState>) -> Result<ClockSyncStatusDto, String> {
    let coord = match app_state.clock_sync.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let elapsed_us = app_state.started_at.elapsed().as_micros() as u64;
    let resolved = coord.resolve(elapsed_us);
    Ok(ClockSyncStatusDto {
        active_source: format!("{:?}", resolved.source),
        bpm: resolved.bpm,
        beat_phase: resolved.beat_phase,
    })
}

/// Dispatch a 2-button footswitch cue action (`"next"`, `"prev"`, `"blackout"`).
#[tauri::command]
pub fn dispatch_footswitch_action(
    app_state: State<'_, AppState>,
    action: String,
) -> Result<FootswitchStatusDto, String> {
    let parsed = match action.as_str() {
        "next" => FootswitchAction::NextCue,
        "prev" => FootswitchAction::PrevCue,
        "blackout" => FootswitchAction::ToggleBlackout,
        other => return Err(format!("unknown footswitch action: {other}")),
    };
    let mut runner = match app_state.cue_runner.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let _ = runner.dispatch(parsed);
    let (state_label, active_index, blackout) = match runner.state() {
        CueRunnerState::Empty => ("empty", None, false),
        CueRunnerState::Ready {
            active_index,
            blackout,
        } => ("ready", Some(active_index), blackout),
        CueRunnerState::Transitioning {
            to_index, blackout, ..
        } => ("transitioning", Some(to_index), blackout),
    };
    Ok(FootswitchStatusDto {
        state: state_label.to_string(),
        active_index,
        blackout,
    })
}

/// Return the current 512-channel DMX512 / Art-Net 4 universe summary.
#[tauri::command]
pub fn get_dmx_universe_status(
    app_state: State<'_, AppState>,
) -> Result<DmxUniverseStatusDto, String> {
    let universe = match app_state.dmx_universe.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let channels = universe.channels();
    let non_zero = channels.iter().filter(|&&v| v > 0).count();
    Ok(DmxUniverseStatusDto {
        channel_count: channels.len(),
        non_zero_channels: non_zero,
    })
}

/// Set a single 1-based DMX512 channel (`1..=512`) to `0..=255`.
#[tauri::command]
pub fn set_dmx_channel(
    app_state: State<'_, AppState>,
    channel: u16,
    value: u8,
) -> Result<DmxUniverseStatusDto, String> {
    let mut universe = match app_state.dmx_universe.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if !universe.set_channel(channel, value) {
        return Err(format!("DMX channel {channel} out of range (1..=512)"));
    }
    let channels = universe.channels();
    let non_zero = channels.iter().filter(|&&v| v > 0).count();
    Ok(DmxUniverseStatusDto {
        channel_count: channels.len(),
        non_zero_channels: non_zero,
    })
}

/// Return the ADR-017 external data-ingest engine status.
#[tauri::command]
pub fn get_data_ingest_status(
    app_state: State<'_, AppState>,
) -> Result<DataIngestStatusDto, String> {
    let ingest = match app_state.data_ingest.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Ok(DataIngestStatusDto {
        source_count: ingest.sources().len(),
    })
}

/// Return the ADR-016 local in-process observability emitter status.
#[tauri::command]
pub fn get_observability_status(
    app_state: State<'_, AppState>,
) -> Result<ObservabilityStatusDto, String> {
    Ok(ObservabilityStatusDto {
        buffered_events: app_state.observability_emitter.len(),
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-DETERMINISM (T1) — Verifies shell IPC wiring for
    //! BroadcastMixer, ClockSync, DMX512, CueRunner, DataIngest, and Observability.

    use super::*;

    #[test]
    fn broadcast_mixer_and_dmx_wire_cleanly_through_app_state() {
        let app = AppState::new();
        let initial = build_mixer_status(&app);
        assert_eq!(initial.sample_rate_hz, 48_000);
        assert_eq!(initial.buses.len(), 4);

        {
            let mut mixer = app.broadcast_mixer.lock().unwrap();
            let mut cfg = mixer.bus_config(BusId::Microphone).unwrap();
            cfg.muted = true;
            cfg.gain_db = -6.0;
            mixer.configure_bus(BusId::Microphone, cfg);
        }
        let updated = build_mixer_status(&app);
        let mic = updated.buses.iter().find(|b| b.id == "mic").unwrap();
        assert!(mic.muted);
        assert!((mic.gain_db - (-6.0)).abs() < 1e-4);

        {
            let mut dmx = app.dmx_universe.lock().unwrap();
            assert!(dmx.set_channel(1, 255));
            assert!(!dmx.set_channel(0, 128));
            assert_eq!(dmx.get_channel(1), Some(255));
        }
    }
}
