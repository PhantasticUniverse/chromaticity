// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! OSC route bind-target parsing and inbound dispatch.
//!
//! The operator-facing route registry persists the target as a plain
//! `String` on the IPC wire. This module is the semantic seam that
//! validates that string at bind time and turns an inbound OSC message
//! on a bound route into a command-shaped action. Host code can attach
//! an [`OscDispatchSink`] to submit those actions to the runtime command
//! bus or a sibling callback channel.

use std::collections::BTreeMap;

use rosc::{OscMessage, OscPacket, OscType};

use crate::routes::{RouteAddr, RouteParseError, parse_diagnostic};

/// Parsed operator target for a bound OSC route.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindTarget {
    /// `control:<bank>:<name>` — dispatches a control value write.
    Control {
        bank: String,
        name: String,
        control_id: u32,
    },
    /// `scene:<scene_id>` — host callback requests a scene load.
    Scene { scene_id: String },
    /// `snap:<slot>` — dispatches a Snap recall.
    Snap { slot: u8 },
    /// `preset:<id>` — dispatches a preset recall.
    Preset { id: u32 },
    /// `playlist:next` / `playlist:prev` — dispatches playlist +/- 1.
    Playlist { step: i32 },
}

/// Errors returned when validating an operator bind target.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BindError {
    EmptyTarget,
    UnknownFamily(String),
    InvalidControlTarget,
    EmptyControlPart,
    InvalidSceneTarget,
    EmptySceneId,
    InvalidSnapTarget,
    InvalidSnapSlot(String),
    InvalidPresetTarget,
    InvalidPresetId(String),
    InvalidPlaylistTarget,
    InvalidRoutePattern(String),
}

impl core::fmt::Display for BindError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyTarget => write!(f, "bind target must not be empty"),
            Self::UnknownFamily(family) => write!(
                f,
                "unknown osc bind target family: {family} (expected control, scene, snap, preset, or playlist)",
            ),
            Self::InvalidControlTarget => write!(f, "control target must be control:<bank>:<name>"),
            Self::EmptyControlPart => {
                write!(f, "control target bank and name must not be empty")
            }
            Self::InvalidSceneTarget => write!(f, "scene target must be scene:<scene_id>"),
            Self::EmptySceneId => write!(f, "scene target id must not be empty"),
            Self::InvalidSnapTarget => write!(f, "snap target must be snap:<slot>"),
            Self::InvalidSnapSlot(slot) => write!(f, "snap slot {slot} out of range (max 15)"),
            Self::InvalidPresetTarget => write!(f, "preset target must be preset:<id>"),
            Self::InvalidPresetId(id) => write!(f, "preset id {id} must be a u32"),
            Self::InvalidPlaylistTarget => {
                write!(f, "playlist target must be playlist:next or playlist:prev")
            }
            Self::InvalidRoutePattern(route) => {
                write!(f, "osc route pattern cannot be bound: {route}")
            }
        }
    }
}

impl std::error::Error for BindError {}

/// Parse and validate a route bind target.
pub fn parse_bind_target(target: &str) -> Result<BindTarget, BindError> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(BindError::EmptyTarget);
    }

    let parts: Vec<&str> = trimmed.split(':').collect();
    let family = parts
        .first()
        .copied()
        .ok_or(BindError::EmptyTarget)?
        .to_ascii_lowercase();
    match family.as_str() {
        "control" => parse_control_target(&parts),
        "scene" => parse_scene_target(&parts),
        "snap" => parse_snap_target(&parts),
        "preset" => parse_preset_target(&parts),
        "playlist" => parse_playlist_target(&parts),
        _ => Err(BindError::UnknownFamily(family)),
    }
}

fn parse_control_target(parts: &[&str]) -> Result<BindTarget, BindError> {
    if parts.len() != 3 {
        return Err(BindError::InvalidControlTarget);
    }
    let bank = parts[1].trim();
    let name = parts[2].trim();
    if bank.is_empty() || name.is_empty() {
        return Err(BindError::EmptyControlPart);
    }
    Ok(BindTarget::Control {
        bank: bank.to_string(),
        name: name.to_string(),
        control_id: control_target_id(bank, name),
    })
}

fn parse_scene_target(parts: &[&str]) -> Result<BindTarget, BindError> {
    if parts.len() != 2 {
        return Err(BindError::InvalidSceneTarget);
    }
    let scene_id = parts[1].trim();
    if scene_id.is_empty() {
        return Err(BindError::EmptySceneId);
    }
    Ok(BindTarget::Scene {
        scene_id: scene_id.to_string(),
    })
}

fn parse_snap_target(parts: &[&str]) -> Result<BindTarget, BindError> {
    if parts.len() != 2 {
        return Err(BindError::InvalidSnapTarget);
    }
    let raw = parts[1].trim();
    let slot = raw
        .parse::<u8>()
        .map_err(|_| BindError::InvalidSnapSlot(raw.to_string()))?;
    if slot > 15 {
        return Err(BindError::InvalidSnapSlot(raw.to_string()));
    }
    Ok(BindTarget::Snap { slot })
}

fn parse_preset_target(parts: &[&str]) -> Result<BindTarget, BindError> {
    if parts.len() != 2 {
        return Err(BindError::InvalidPresetTarget);
    }
    let raw = parts[1].trim();
    let id = raw
        .parse::<u32>()
        .map_err(|_| BindError::InvalidPresetId(raw.to_string()))?;
    Ok(BindTarget::Preset { id })
}

fn parse_playlist_target(parts: &[&str]) -> Result<BindTarget, BindError> {
    if parts.len() != 2 {
        return Err(BindError::InvalidPlaylistTarget);
    }
    match parts[1].trim().to_ascii_lowercase().as_str() {
        "next" => Ok(BindTarget::Playlist { step: 1 }),
        "prev" => Ok(BindTarget::Playlist { step: -1 }),
        _ => Err(BindError::InvalidPlaylistTarget),
    }
}

/// Stable numeric id used while the runtime command surface still
/// exposes `ControlId(u32)` and the OSC target names controls by
/// `bank:name`.
#[must_use]
pub fn control_target_id(bank: &str, name: &str) -> u32 {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;
    let mut hash = FNV_OFFSET;
    for byte in bank.bytes().chain([b':']).chain(name.bytes()) {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Runtime-command-shaped action produced by inbound OSC dispatch.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OscRuntimeCommand {
    ControlSet { id: u32, value: f32 },
    PresetRecall { id: u32 },
    SnapRecall { slot: u8 },
    PlaylistAdvance { step: i32 },
}

/// Dispatch action emitted to a host-provided sink.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OscDispatchAction {
    RuntimeCommand(OscRuntimeCommand),
    SceneLoad { scene_id: String },
}

/// Host-provided receiver for dispatched OSC actions.
pub trait OscDispatchSink: Send + Sync {
    fn dispatch(&self, action: OscDispatchAction) -> Result<(), String>;
}

/// Dispatch errors surfaced from route matching, payload extraction,
/// or the host sink.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OscDispatchError {
    RouteRejected(RouteParseError),
    MissingDispatchSink(String),
    MissingFloatArg(String),
    SinkRejected(String),
}

impl core::fmt::Display for OscDispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RouteRejected(err) => write!(f, "{err}"),
            Self::MissingDispatchSink(addr) => {
                write!(
                    f,
                    "bound OSC route {addr} matched but no dispatch sink is installed"
                )
            }
            Self::MissingFloatArg(addr) => {
                write!(
                    f,
                    "bound control route {addr} requires an OSC float argument"
                )
            }
            Self::SinkRejected(message) => {
                write!(f, "osc dispatch sink rejected action: {message}")
            }
        }
    }
}

impl std::error::Error for OscDispatchError {}

/// Bind table used by [`crate::adapter::UdpOscAdapter`] during poll.
#[derive(Debug, Clone, Default)]
pub struct OscRouteBindings {
    entries: BTreeMap<String, BindTarget>,
}

impl OscRouteBindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn bind(&mut self, canonical: &str, target: &str) -> Result<(), BindError> {
        let route = canonical.trim();
        parse_route_pattern(route)
            .map_err(|_| BindError::InvalidRoutePattern(route.to_string()))?;
        let target = parse_bind_target(target)?;
        self.entries.insert(route.to_string(), target);
        Ok(())
    }

    pub fn unbind(&mut self, canonical: &str) {
        self.entries.remove(canonical.trim());
    }

    /// Remove every adapter-local route binding.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn dispatch_packet(
        &self,
        packet: &OscPacket,
        sink: &dyn OscDispatchSink,
    ) -> Result<usize, OscDispatchError> {
        self.dispatch_packet_optional(packet, Some(sink))
    }

    pub(crate) fn dispatch_packet_optional(
        &self,
        packet: &OscPacket,
        sink: Option<&dyn OscDispatchSink>,
    ) -> Result<usize, OscDispatchError> {
        match packet {
            OscPacket::Message(message) => self.dispatch_message(message, sink),
            OscPacket::Bundle(bundle) => {
                let mut dispatched = 0usize;
                for packet in &bundle.content {
                    dispatched =
                        dispatched.saturating_add(self.dispatch_packet_optional(packet, sink)?);
                }
                Ok(dispatched)
            }
        }
    }

    fn dispatch_message(
        &self,
        message: &OscMessage,
        sink: Option<&dyn OscDispatchSink>,
    ) -> Result<usize, OscDispatchError> {
        let message_addr = match parse_diagnostic(&message.addr) {
            Ok(addr) => addr,
            Err(RouteParseError::PrefixReserved(prefix)) => {
                return Err(OscDispatchError::RouteRejected(
                    RouteParseError::PrefixReserved(prefix),
                ));
            }
            Err(_) => return Ok(0),
        };

        for (pattern, target) in &self.entries {
            let pattern_addr =
                parse_route_pattern(pattern).map_err(OscDispatchError::RouteRejected)?;
            if route_matches(&pattern_addr, &message_addr) {
                let action = action_for_target(target, message)?;
                let sink = sink
                    .ok_or_else(|| OscDispatchError::MissingDispatchSink(message.addr.clone()))?;
                sink.dispatch(action)
                    .map_err(OscDispatchError::SinkRejected)?;
                return Ok(1);
            }
        }
        Ok(0)
    }
}

fn parse_route_pattern(pattern: &str) -> Result<RouteAddr, RouteParseError> {
    let route = match pattern.split_whitespace().next() {
        Some(route) if !route.is_empty() => route,
        _ => return Err(RouteParseError::MalformedAddress),
    };
    parse_diagnostic(route)
}

fn route_matches(pattern: &RouteAddr, message: &RouteAddr) -> bool {
    pattern.prefix == message.prefix
        && pattern.segments.len() == message.segments.len()
        && pattern
            .segments
            .iter()
            .zip(&message.segments)
            .all(|(left, right)| is_placeholder(left) || left == right)
}

fn is_placeholder(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}') && segment.len() >= 2
}

fn action_for_target(
    target: &BindTarget,
    message: &OscMessage,
) -> Result<OscDispatchAction, OscDispatchError> {
    match target {
        BindTarget::Control { control_id, .. } => {
            let value = first_float_arg(message)
                .ok_or_else(|| OscDispatchError::MissingFloatArg(message.addr.clone()))?;
            Ok(OscDispatchAction::RuntimeCommand(
                OscRuntimeCommand::ControlSet {
                    id: *control_id,
                    value,
                },
            ))
        }
        BindTarget::Scene { scene_id } => Ok(OscDispatchAction::SceneLoad {
            scene_id: scene_id.clone(),
        }),
        BindTarget::Snap { slot } => Ok(OscDispatchAction::RuntimeCommand(
            OscRuntimeCommand::SnapRecall { slot: *slot },
        )),
        BindTarget::Preset { id } => Ok(OscDispatchAction::RuntimeCommand(
            OscRuntimeCommand::PresetRecall { id: *id },
        )),
        BindTarget::Playlist { step } => Ok(OscDispatchAction::RuntimeCommand(
            OscRuntimeCommand::PlaylistAdvance { step: *step },
        )),
    }
}

fn first_float_arg(message: &OscMessage) -> Option<f32> {
    message.args.first().and_then(|arg| match arg {
        OscType::Float(value) => Some(*value),
        _ => None,
    })
}

#[cfg(test)]
#[cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
mod tests {
    //! Evidence: EV-INTEGRATION-MATRIX (T1) — bind-target grammar
    //! and adapter dispatch action pins.

    use std::sync::Mutex;

    use rosc::{OscMessage, OscPacket, OscType};

    use super::{
        BindError, BindTarget, OscDispatchAction, OscDispatchSink, OscRouteBindings,
        OscRuntimeCommand, control_target_id, parse_bind_target,
    };

    #[test]
    fn parse_bind_target_accepts_supported_target_families() {
        assert_eq!(
            parse_bind_target("control:scene:brightness").unwrap(),
            BindTarget::Control {
                bank: "scene".to_string(),
                name: "brightness".to_string(),
                control_id: control_target_id("scene", "brightness"),
            },
        );
        assert_eq!(
            parse_bind_target("scene:demo-kaleido").unwrap(),
            BindTarget::Scene {
                scene_id: "demo-kaleido".to_string(),
            },
        );
        assert_eq!(
            parse_bind_target("snap:15").unwrap(),
            BindTarget::Snap { slot: 15 },
        );
        assert_eq!(
            parse_bind_target("preset:42").unwrap(),
            BindTarget::Preset { id: 42 },
        );
        assert_eq!(
            parse_bind_target("playlist:next").unwrap(),
            BindTarget::Playlist { step: 1 },
        );
        assert_eq!(
            parse_bind_target("playlist:prev").unwrap(),
            BindTarget::Playlist { step: -1 },
        );
    }

    #[test]
    fn parse_bind_target_rejects_invalid_targets() {
        assert_eq!(parse_bind_target("").unwrap_err(), BindError::EmptyTarget);
        assert_eq!(
            parse_bind_target("control:scene").unwrap_err(),
            BindError::InvalidControlTarget,
        );
        assert_eq!(
            parse_bind_target("control::brightness").unwrap_err(),
            BindError::EmptyControlPart,
        );
        assert_eq!(
            parse_bind_target("snap:16").unwrap_err(),
            BindError::InvalidSnapSlot("16".to_string()),
        );
        assert_eq!(
            parse_bind_target("preset:not-a-number").unwrap_err(),
            BindError::InvalidPresetId("not-a-number".to_string()),
        );
        assert_eq!(
            parse_bind_target("playlist:previous").unwrap_err(),
            BindError::InvalidPlaylistTarget,
        );
    }

    #[derive(Default)]
    struct VecSink {
        actions: Mutex<Vec<OscDispatchAction>>,
    }

    impl OscDispatchSink for VecSink {
        fn dispatch(&self, action: OscDispatchAction) -> Result<(), String> {
            let mut guard = self
                .actions
                .lock()
                .map_err(|err| format!("actions mutex poisoned: {err}"))?;
            guard.push(action);
            Ok(())
        }
    }

    #[test]
    fn dispatch_message_emits_control_set_action_for_bound_route() {
        let mut bindings = OscRouteBindings::new();
        bindings
            .bind("/controls/{bank}/{name}", "control:scene:brightness")
            .unwrap();
        let sink = VecSink::default();
        let message = OscMessage {
            addr: "/controls/scene/brightness".to_string(),
            args: vec![OscType::Float(0.75)],
        };

        let dispatched = bindings
            .dispatch_packet(&OscPacket::Message(message), &sink)
            .unwrap();

        assert_eq!(dispatched, 1);
        assert_eq!(
            sink.actions.lock().unwrap().as_slice(),
            &[OscDispatchAction::RuntimeCommand(
                OscRuntimeCommand::ControlSet {
                    id: control_target_id("scene", "brightness"),
                    value: 0.75,
                },
            )],
        );
    }

    #[test]
    fn dispatch_message_emits_runtime_and_scene_actions() {
        let mut bindings = OscRouteBindings::new();
        bindings.bind("/scenes/{sceneName}", "scene:demo").unwrap();
        bindings.bind("/presets/{name}", "preset:7").unwrap();
        bindings.bind("/favslots/{n}", "snap:3").unwrap();
        bindings.bind("/playlists/next", "playlist:next").unwrap();
        let sink = VecSink::default();

        for addr in [
            "/scenes/demo",
            "/presets/calm",
            "/favslots/3",
            "/playlists/next",
        ] {
            let message = OscMessage {
                addr: addr.to_string(),
                args: Vec::new(),
            };
            bindings
                .dispatch_packet(&OscPacket::Message(message), &sink)
                .unwrap();
        }

        assert_eq!(
            sink.actions.lock().unwrap().as_slice(),
            &[
                OscDispatchAction::SceneLoad {
                    scene_id: "demo".to_string(),
                },
                OscDispatchAction::RuntimeCommand(OscRuntimeCommand::PresetRecall { id: 7 }),
                OscDispatchAction::RuntimeCommand(OscRuntimeCommand::SnapRecall { slot: 3 }),
                OscDispatchAction::RuntimeCommand(OscRuntimeCommand::PlaylistAdvance { step: 1 }),
            ],
        );
    }

    #[test]
    fn reserved_data_route_returns_distinct_dispatch_error() {
        let mut bindings = OscRouteBindings::new();
        bindings
            .bind("/controls/{bank}/{name}", "control:a:b")
            .unwrap();
        let sink = VecSink::default();
        let message = OscMessage {
            addr: "/data/subscribe/foo".to_string(),
            args: Vec::new(),
        };

        let err = bindings
            .dispatch_packet(&OscPacket::Message(message), &sink)
            .unwrap_err();

        assert!(
            err.to_string().contains("reserved"),
            "expected reserved diagnostic, got {err}",
        );
    }
}
