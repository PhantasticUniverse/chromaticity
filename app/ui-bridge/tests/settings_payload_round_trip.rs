// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Evidence: EV-INTEGRATION-MATRIX (T2) — Wave 2.1 Lane C.3 settings
//! persistence IPC payload round-trip conformance.
//!
//! T2 per `docs/43 §15.5`: this corpus drives the
//! [`chromaticity_ui_bridge::SettingsPayload`] across the ts-rs newtype
//! cluster + bidirectional `From<chromaticity_runtime::settings::Settings>`
//! conversion + serde JSON wire shape per the Wave 2.1 Lane C.3 plan
//! (`/Users/xaviermac/.claude/plans/inherited-percolating-quill.md`).
//! Field-by-field equality across all four sub-payloads
//! (general / audio / graphics / storage) is the load-bearing
//! assertion: any future payload-shape divergence between the engine
//! type and the bridge mirror surfaces here before the committed
//! ts-rs bindings under `src/lib/ipc/bindings/SettingsPayload.ts`
//! drift.
//!
//! Lives under `app/ui-bridge/tests/` (outside the
//! `engine/**`/`integrations/**` evidence-tag-lint coverage scope per
//! `scripts/evidence-tag-lint.sh`); the in-source T1 drift pins under
//! `engine/runtime/src/settings.rs` carry the load/save round-trip
//! invariants that this test layer composes on top of.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use chromaticity_runtime::settings::{
    AudioSettings, GeneralSettings, GraphicsSettings, LiveSourcesSettings, OscSettings, Settings,
    StorageSettings, TempoSettings, Theme,
};
use chromaticity_ui_bridge::{
    AudioSettingsPayload, GeneralSettingsPayload, GraphicsSettingsPayload,
    LiveSourcesSettingsPayload, OscSettingsPayload, SettingsPayload, StorageSettingsPayload,
    TempoSettingsPayload, ThemePayload,
};
use ts_rs::TS;

/// Helper: a non-default settings document exercising distinct
/// values across every field so a copy-paste field-mapping bug
/// surfaces (e.g. swapping `custom_width` and `custom_height` in a
/// hand-rolled `From` impl).
fn nonzero_settings() -> Settings {
    Settings {
        general: GeneralSettings {
            theme: Theme::Atelier,
        },
        audio: AudioSettings {
            input_device: "BlackHole 16ch".to_string(),
            output_device: "External Audio Interface".to_string(),
            sample_rate_hz: 96_000,
            latency_hint: "low".to_string(),
            channel_layout: "stereo-3-4".to_string(),
            channel_left: "L3".to_string(),
            channel_right: "R4".to_string(),
            input_gain: 0.75,
        },
        graphics: GraphicsSettings {
            resolution_preset: "4k".to_string(),
            custom_resolution_enabled: true,
            custom_width: 3840,
            custom_height: 2160,
            fps_target: "120".to_string(),
            vsync: false,
            hdr_target: "hdr10".to_string(),
            display: "display-2".to_string(),
            color_space: "p3".to_string(),
            multi_display_mode: "span2".to_string(),
            render_scale: 1.5,
            screen_fitting: "fill".to_string(),
            fps_slider_enabled: true,
        },
        storage: StorageSettings {
            media_folders: vec![
                "/Users/operator/Movies/A".to_string(),
                "/Users/operator/Movies/B".to_string(),
            ],
            scene_folders: vec!["/Users/operator/Scenes".to_string()],
            backup_path: "/Users/operator/Backups".to_string(),
            backup_retention: "retain-25".to_string(),
        },
        live_sources: LiveSourcesSettings {
            syphon_input_enabled: false,
            syphon_output_enabled: false,
            ndi_input_enabled: true,
            ndi_output_enabled: false,
            constant_output_resolution: true,
            sender_name: "studio-A".to_string(),
        },
        osc: OscSettings {
            local_ip: "192.168.1.42".to_string(),
            port: 9100,
            out_port: 9101,
            tx_enabled: false,
            rx_enabled: true,
        },
        tempo: TempoSettings {
            ableton_link_enabled: false,
            start_stop_sync: false,
            midi_clock_in_enabled: true,
            midi_clock_in_device: "midi-port-2".to_string(),
            midi_clock_out_enabled: false,
            midi_clock_out_device: "midi-port-3".to_string(),
            quantize: "phrase".to_string(),
            manual_bpm: 128.5,
            link_offset_ms: -25.0,
            audio_bpm_sensitivity: 80.0,
            bpm_source_priority: vec![
                "manual".to_string(),
                "midiclock".to_string(),
                "audio".to_string(),
                "link".to_string(),
            ],
        },
    }
}

fn nonzero_payload() -> SettingsPayload {
    SettingsPayload {
        general: GeneralSettingsPayload {
            theme: ThemePayload::Atelier,
        },
        audio: AudioSettingsPayload {
            input_device: "BlackHole 16ch".to_string(),
            output_device: "External Audio Interface".to_string(),
            sample_rate_hz: 96_000,
            latency_hint: "low".to_string(),
            channel_layout: "stereo-3-4".to_string(),
            channel_left: "L3".to_string(),
            channel_right: "R4".to_string(),
            input_gain: 0.75,
        },
        graphics: GraphicsSettingsPayload {
            resolution_preset: "4k".to_string(),
            custom_resolution_enabled: true,
            custom_width: 3840,
            custom_height: 2160,
            fps_target: "120".to_string(),
            vsync: false,
            hdr_target: "hdr10".to_string(),
            display: "display-2".to_string(),
            color_space: "p3".to_string(),
            multi_display_mode: "span2".to_string(),
            render_scale: 1.5,
            screen_fitting: "fill".to_string(),
            fps_slider_enabled: true,
        },
        storage: StorageSettingsPayload {
            media_folders: vec![
                "/Users/operator/Movies/A".to_string(),
                "/Users/operator/Movies/B".to_string(),
            ],
            scene_folders: vec!["/Users/operator/Scenes".to_string()],
            backup_path: "/Users/operator/Backups".to_string(),
            backup_retention: "retain-25".to_string(),
        },
        live_sources: LiveSourcesSettingsPayload {
            syphon_input_enabled: false,
            syphon_output_enabled: false,
            ndi_input_enabled: true,
            ndi_output_enabled: false,
            constant_output_resolution: true,
            sender_name: "studio-A".to_string(),
        },
        osc: OscSettingsPayload {
            local_ip: "192.168.1.42".to_string(),
            port: 9100,
            out_port: 9101,
            tx_enabled: false,
            rx_enabled: true,
        },
        tempo: TempoSettingsPayload {
            ableton_link_enabled: false,
            start_stop_sync: false,
            midi_clock_in_enabled: true,
            midi_clock_in_device: "midi-port-2".to_string(),
            midi_clock_out_enabled: false,
            midi_clock_out_device: "midi-port-3".to_string(),
            quantize: "phrase".to_string(),
            manual_bpm: 128.5,
            link_offset_ms: -25.0,
            audio_bpm_sensitivity: 80.0,
            bpm_source_priority: vec![
                "manual".to_string(),
                "midiclock".to_string(),
                "audio".to_string(),
                "link".to_string(),
            ],
        },
    }
}

#[test]
fn engine_to_payload_field_by_field_equality() {
    // Producer-side conversion: the engine type's value flows through
    // the `From<Settings>` impl. The structural assertion pins the
    // cross-section field set so a future field-mapping bug surfaces
    // here.
    let settings = nonzero_settings();
    let payload: SettingsPayload = settings.clone().into();

    assert_eq!(payload.general.theme, ThemePayload::Atelier);
    assert_eq!(payload.audio.input_device, settings.audio.input_device);
    assert_eq!(payload.audio.sample_rate_hz, settings.audio.sample_rate_hz);
    assert_eq!(payload.audio.input_gain, settings.audio.input_gain);
    assert_eq!(
        payload.graphics.resolution_preset,
        settings.graphics.resolution_preset
    );
    assert_eq!(
        payload.graphics.custom_width,
        settings.graphics.custom_width
    );
    assert_eq!(
        payload.graphics.custom_height,
        settings.graphics.custom_height
    );
    assert_eq!(payload.graphics.fps_target, settings.graphics.fps_target);
    assert_eq!(payload.graphics.vsync, settings.graphics.vsync);
    assert_eq!(
        payload.graphics.render_scale,
        settings.graphics.render_scale
    );
    assert_eq!(
        payload.storage.media_folders,
        settings.storage.media_folders
    );
    assert_eq!(
        payload.storage.scene_folders,
        settings.storage.scene_folders
    );
    assert_eq!(payload.storage.backup_path, settings.storage.backup_path);
}

#[test]
fn payload_to_engine_round_trip_field_by_field_equality() {
    // Reverse direction: a payload constructed on the consumer side
    // (e.g. by deserialising the get_settings IPC response) converts
    // back into the engine type with field-identity preserved.
    let payload = nonzero_payload();
    let settings: Settings = payload.clone().into();

    assert_eq!(settings.general.theme, Theme::Atelier);
    assert_eq!(settings.audio.input_device, payload.audio.input_device);
    assert_eq!(settings.audio.sample_rate_hz, payload.audio.sample_rate_hz);
    assert_eq!(settings.audio.input_gain, payload.audio.input_gain);
    assert_eq!(
        settings.graphics.resolution_preset,
        payload.graphics.resolution_preset
    );
    assert_eq!(settings.graphics.fps_target, payload.graphics.fps_target);
    assert_eq!(settings.graphics.vsync, payload.graphics.vsync);
    assert_eq!(
        settings.graphics.render_scale,
        payload.graphics.render_scale
    );
    assert_eq!(
        settings.storage.media_folders,
        payload.storage.media_folders
    );
    assert_eq!(
        settings.storage.scene_folders,
        payload.storage.scene_folders
    );
    assert_eq!(settings.storage.backup_path, payload.storage.backup_path);
    assert_eq!(
        settings.storage.backup_retention,
        payload.storage.backup_retention
    );
}

#[test]
fn payload_json_round_trip_field_by_field_equality() {
    // The `set_settings` Tauri command carries `SettingsPayload` as
    // its argument. Tauri serialises the argument to JSON over the
    // IPC seam; pin serde JSON round-trip identity so a future
    // serde-tagged attribute or rename surfaces here before the
    // wire shape drifts.
    let payload = nonzero_payload();
    let json = serde_json::to_string(&payload).expect("serialize payload");
    let decoded: SettingsPayload = serde_json::from_str(&json).expect("deserialize payload");
    assert_eq!(decoded, payload);
}

#[test]
fn default_engine_to_payload_round_trip_preserves_defaults() {
    // The initial-launch case: the shell loads the default
    // settings document (no file yet) and emits the payload back
    // through `get_settings`. The bridge MUST preserve the
    // bundle-faithful defaults so the frontend `settingsStore`
    // initialises with the same shape the existing `$state`
    // fixtures show today.
    let settings = Settings::default();
    let payload: SettingsPayload = settings.clone().into();
    let back: Settings = payload.into();
    assert_eq!(back, settings);
}

#[test]
fn theme_payload_variants_round_trip_through_engine() {
    // Pin all three discriminants so a future bridge-side
    // ThemePayload variant addition forces a matching engine-side
    // Theme variant addition (and vice versa).
    for theme in [Theme::Pulse, Theme::Atelier, Theme::Console] {
        let payload: ThemePayload = theme.into();
        let back: Theme = payload.into();
        assert_eq!(back, theme);
    }
}

#[test]
fn ts_rs_exported_names_match_committed_bindings() {
    // The committed bindings live at
    // `src/lib/ipc/bindings/SettingsPayload.ts` (+ the sibling
    // section payloads). ts-rs's `name()` is the source of truth for
    // the export filename; an accidental rename of a Rust newtype
    // (or a `#[ts(rename = ...)]` attribute) surfaces here before
    // the frontend index drifts.
    assert_eq!(<SettingsPayload as TS>::name(), "SettingsPayload");
    assert_eq!(
        <GeneralSettingsPayload as TS>::name(),
        "GeneralSettingsPayload"
    );
    assert_eq!(<AudioSettingsPayload as TS>::name(), "AudioSettingsPayload");
    assert_eq!(
        <GraphicsSettingsPayload as TS>::name(),
        "GraphicsSettingsPayload"
    );
    assert_eq!(
        <StorageSettingsPayload as TS>::name(),
        "StorageSettingsPayload"
    );
    assert_eq!(<ThemePayload as TS>::name(), "ThemePayload");
    assert_eq!(<OscSettingsPayload as TS>::name(), "OscSettingsPayload");
    assert_eq!(<TempoSettingsPayload as TS>::name(), "TempoSettingsPayload");
    assert_eq!(
        <LiveSourcesSettingsPayload as TS>::name(),
        "LiveSourcesSettingsPayload"
    );
}

#[test]
fn tempo_settings_engine_to_payload_round_trip() {
    // Wave 2.7 Lane SET.A: pin the TempoSettingsPayload field-by-field
    // mapping so a future shape divergence between
    // `chromaticity_runtime::settings::TempoSettings` and the bridge
    // mirror surfaces here before the wire shape drifts.
    let settings = nonzero_settings();
    let payload: SettingsPayload = settings.clone().into();
    assert_eq!(
        payload.tempo.ableton_link_enabled,
        settings.tempo.ableton_link_enabled
    );
    assert_eq!(
        payload.tempo.start_stop_sync,
        settings.tempo.start_stop_sync
    );
    assert_eq!(
        payload.tempo.midi_clock_in_enabled,
        settings.tempo.midi_clock_in_enabled
    );
    assert_eq!(
        payload.tempo.midi_clock_in_device,
        settings.tempo.midi_clock_in_device
    );
    assert_eq!(
        payload.tempo.midi_clock_out_enabled,
        settings.tempo.midi_clock_out_enabled
    );
    assert_eq!(
        payload.tempo.midi_clock_out_device,
        settings.tempo.midi_clock_out_device
    );
    assert_eq!(payload.tempo.quantize, settings.tempo.quantize);
    assert_eq!(payload.tempo.manual_bpm, settings.tempo.manual_bpm);
    assert_eq!(payload.tempo.link_offset_ms, settings.tempo.link_offset_ms);
    assert_eq!(
        payload.tempo.audio_bpm_sensitivity,
        settings.tempo.audio_bpm_sensitivity
    );
    assert_eq!(
        payload.tempo.bpm_source_priority,
        settings.tempo.bpm_source_priority
    );

    // Reverse direction.
    let back: Settings = payload.into();
    assert_eq!(back.tempo.quantize, settings.tempo.quantize);
    assert_eq!(back.tempo.manual_bpm, settings.tempo.manual_bpm);
    assert_eq!(
        back.tempo.bpm_source_priority,
        settings.tempo.bpm_source_priority
    );
}

#[test]
fn live_sources_engine_to_payload_round_trip() {
    // Wave 2.7 SET.B: pin the LiveSourcesSettingsPayload field-by-
    // field mapping so a future shape divergence between
    // `chromaticity_runtime::settings::LiveSourcesSettings` and the
    // bridge mirror surfaces here before the wire shape drifts.
    let settings = nonzero_settings();
    let payload: SettingsPayload = settings.clone().into();
    assert_eq!(
        payload.live_sources.syphon_input_enabled,
        settings.live_sources.syphon_input_enabled
    );
    assert_eq!(
        payload.live_sources.syphon_output_enabled,
        settings.live_sources.syphon_output_enabled
    );
    assert_eq!(
        payload.live_sources.ndi_input_enabled,
        settings.live_sources.ndi_input_enabled
    );
    assert_eq!(
        payload.live_sources.ndi_output_enabled,
        settings.live_sources.ndi_output_enabled
    );
    assert_eq!(
        payload.live_sources.constant_output_resolution,
        settings.live_sources.constant_output_resolution
    );
    assert_eq!(
        payload.live_sources.sender_name,
        settings.live_sources.sender_name
    );

    // Reverse direction.
    let back: Settings = payload.into();
    assert_eq!(back.live_sources, settings.live_sources);
}

#[test]
fn osc_settings_engine_to_payload_round_trip() {
    // Wave 2.1 Lane D.3: pin the OscSettingsPayload field-by-field
    // mapping so a future shape divergence between
    // `chromaticity_runtime::settings::OscSettings` and the bridge
    // mirror surfaces here before the wire shape drifts.
    let settings = nonzero_settings();
    let payload: SettingsPayload = settings.clone().into();
    assert_eq!(payload.osc.local_ip, settings.osc.local_ip);
    assert_eq!(payload.osc.port, settings.osc.port);
    assert_eq!(payload.osc.out_port, settings.osc.out_port);
    assert_eq!(payload.osc.tx_enabled, settings.osc.tx_enabled);
    assert_eq!(payload.osc.rx_enabled, settings.osc.rx_enabled);

    // Reverse direction.
    let back: Settings = payload.into();
    assert_eq!(back.osc.local_ip, settings.osc.local_ip);
    assert_eq!(back.osc.port, settings.osc.port);
    assert_eq!(back.osc.out_port, settings.osc.out_port);
}
