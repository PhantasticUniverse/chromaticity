// SPDX-License-Identifier: LicenseRef-Chromaticity-Proprietary
// SPDX-FileCopyrightText: 2026 Chromaticity

//! Phase-3 wave PR 6 EV-PERF-BUDGETS criterion harness (Path B,
//! nightly-only proxy benches).
//!
//! Produces evidence for `EV-PERF-BUDGETS` row 36 of
//! `docs/26-evidence-artifact-registry.md`. Wired into the existing
//! `.github/workflows/macos-nightly.yml` workflow so the harness runs
//! once per day on Apple Silicon Metal alongside the cross-backend
//! nextest sweep — no per-PR matrix expansion (the AO posture rejects
//! the ~$70-80/month per-PR macOS surcharge in favour of nightly-only
//! cadence at ~$20/month, with at most a 24h discovery delay; same
//! framing the macos-nightly workflow already records for the Metal-
//! backend coverage rationale).
//!
//! # Why observable-subsystem proxies, not a frame-loop bench
//!
//! `docs/10-performance-latency-budgets.md` declares the 60 fps
//! frame-time hard budget at 16.67 ms with a 14.00 ms subsystem
//! allocation across command + state, script, audio snapshot read,
//! render graph, and present + overhead. The canonical scheduler-tick
//! frame loop that would let a single-bench cover that allocation top-
//! to-bottom is NOT yet in tree — `engine/runtime/src/state.rs` line
//! ~31 carries the `pub(crate) frame: u64` counter under the in-source
//! comment that the "scheduler-tick path lands in a later slice"
//! (ADR-001 / ADR-006 slice-3 scope, deferred). Phase-3 row 36
//! therefore measures **observable-subsystem proxies** — public-API
//! latency around the command bus, the canonical-state replay + hash
//! pipeline, and the preset / playlist apply path — without
//! introducing instrumentation hooks or widening crate-private
//! surface. Once the scheduler-tick path lands, a follow-up wave can
//! add a true frame-loop bench against the hard budget; until then,
//! the proxies are the most useful regression signal available
//! through the public surface.
//!
//! # Bench groups
//!
//! - `command_bus_dispatch` — Lcg-seeded command streams of 32, 256,
//!   and 1024 commands dispatched on a fresh [`InMemoryBus`]. Stresses
//!   the slice-2 dispatch / channel-send / per-`CommandKind` counter
//!   path. Maps to the docs/10 "command + state updates: <= 1.25 ms"
//!   subsystem allocation as a dispatch-side proxy.
//! - `state_replay_canonical_hash` — Lcg-seeded 256- and 1024-command
//!   streams committed via [`InMemoryBus::commit_all_into`] and then
//!   re-applied through
//!   [`chromaticity_runtime::state_replay::replay_canonical_hash`].
//!   Stresses the slice-3a apply_event + canonical-binary-encode +
//!   FNV-1a-64 path — the same code-path EV-STATE-REPLAY's T2 corpus
//!   exercises for correctness, here measured for wall-clock cost
//!   rather than hash drift.
//! - `preset_recall_apply` — single + multi recall throughput against
//!   the `PresetStore.active` recall semantics the wave-PR-3 corpus
//!   pins.
//! - `playlist_advance_apply` — single + multi advance throughput
//!   against the `PlaylistStore.position` advance semantics.
//!
//! # Implementation notes
//!
//! Setup uses [`InMemoryBus::in_memory()`] and [`State::default()`]
//! per the same pattern as `engine/runtime/tests/state_replay_corpus/
//! main.rs`. The Lcg + `sample_command` shape is copied verbatim from
//! that corpus (Knuth MMIX constants) so the bench input distribution
//! matches the corpus distribution; this keeps regressions traceable
//! across both surfaces.
//!
//! `default harness = false` is set in `Cargo.toml`; criterion ships
//! its own [`criterion_main!`] entry point. The bench file is opted
//! out of the workspace `clippy::unwrap_used` / `clippy::expect_used`
//! denials at file scope — criterion benches aren't `#[test]`
//! functions, so the standard `cfg_attr(test, allow(...))` form
//! doesn't apply; a plain file-scope `#![allow(...)]` is the
//! equivalent for benches.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromaticity_runtime::state::State;
use chromaticity_runtime::state_replay::replay_canonical_hash;
use chromaticity_runtime::{
    Command, CommandBus, ControlId, ControlValue, InMemoryBus, MappingId, MappingSource, PresetId,
    SnapSlotId,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Seeded deterministic command sampler (Knuth MMIX LCG). Mirrors the
/// `engine/runtime/tests/state_replay_corpus/main.rs` `Lcg` and
/// `sample_command` shapes verbatim so the bench's input distribution
/// matches the corpus distribution.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }

    fn next_f32_unit(&mut self) -> f32 {
        let n = self.next_u32();
        (n as f32) / (u32::MAX as f32)
    }
}

fn sample_command(lcg: &mut Lcg) -> Command {
    let kind = lcg.next_u32() % 7;
    let id_val = lcg.next_u32();
    match kind {
        0 => Command::ControlSet {
            id: ControlId(id_val),
            value: ControlValue(lcg.next_f32_unit()),
        },
        1 => Command::PresetRecall {
            id: PresetId(id_val),
        },
        2 => Command::PlaylistAdvance {
            step: lcg.next_u32() as i32,
        },
        3 => Command::MappingBind {
            id: MappingId(id_val),
            source: MappingSource::Osc,
        },
        4 => Command::SnapRecall {
            slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
        },
        5 => Command::SnapSave {
            slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
        },
        _ => Command::SnapClear {
            slot: SnapSlotId::new((id_val % 16) as u8).expect("modulo keeps slot in range"),
        },
    }
}

/// Pre-generate a `count`-element command vector under the Lcg `seed`.
/// Generated outside the criterion measurement closure so dispatch
/// throughput is what's measured, not the sampler.
fn generate_commands(seed: u64, count: u32) -> Vec<Command> {
    let mut lcg = Lcg::new(seed);
    (0..count).map(|_| sample_command(&mut lcg)).collect()
}

fn bench_command_bus_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_bus_dispatch");
    for &count in &[32_u32, 256, 1024] {
        let commands = generate_commands(0xC1D_5EED_u64, count);
        group.bench_function(format!("dispatch_{count}"), |b| {
            b.iter(|| {
                let bus = InMemoryBus::in_memory();
                for command in &commands {
                    bus.dispatch(black_box(command.clone()))
                        .expect("in-memory bus dispatch must succeed");
                }
                black_box(bus.commit_all());
            });
        });
    }
    group.finish();
}

fn bench_state_replay_canonical_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_replay_canonical_hash");
    for &count in &[256_u32, 1024] {
        let commands = generate_commands(0xDEAD_BEEF_u64, count);
        group.bench_function(format!("replay_{count}"), |b| {
            b.iter(|| {
                let bus = InMemoryBus::in_memory();
                for command in &commands {
                    bus.dispatch(command.clone())
                        .expect("in-memory bus dispatch must succeed");
                }
                let mut state = State::default();
                let events = bus.commit_all_into(&mut state);
                black_box(replay_canonical_hash(&events));
            });
        });
    }
    group.finish();
}

fn bench_preset_recall_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("preset_recall_apply");
    group.bench_function("single_recall", |b| {
        b.iter(|| {
            let bus = InMemoryBus::in_memory();
            bus.dispatch(Command::PresetRecall {
                id: black_box(PresetId(7)),
            })
            .expect("preset command must dispatch");
            let mut state = State::default();
            black_box(bus.commit_all_into(&mut state));
        });
    });

    let multi_ids: Vec<u32> = (0..64).collect();
    group.bench_function("multi_recall_64", |b| {
        b.iter(|| {
            let bus = InMemoryBus::in_memory();
            for &id in &multi_ids {
                bus.dispatch(Command::PresetRecall {
                    id: PresetId(black_box(id)),
                })
                .expect("preset command must dispatch");
            }
            let mut state = State::default();
            black_box(bus.commit_all_into(&mut state));
        });
    });
    group.finish();
}

fn bench_playlist_advance_apply(c: &mut Criterion) {
    let mut group = c.benchmark_group("playlist_advance_apply");
    group.bench_function("single_advance", |b| {
        b.iter(|| {
            let bus = InMemoryBus::in_memory();
            bus.dispatch(Command::PlaylistAdvance { step: black_box(4) })
                .expect("playlist command must dispatch");
            let mut state = State::default();
            black_box(bus.commit_all_into(&mut state));
        });
    });

    let multi_steps: Vec<i32> = (0..64).map(|i| if i % 2 == 0 { 3 } else { -2 }).collect();
    group.bench_function("multi_advance_64", |b| {
        b.iter(|| {
            let bus = InMemoryBus::in_memory();
            for &step in &multi_steps {
                bus.dispatch(Command::PlaylistAdvance {
                    step: black_box(step),
                })
                .expect("playlist command must dispatch");
            }
            let mut state = State::default();
            black_box(bus.commit_all_into(&mut state));
        });
    });
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets =
        bench_command_bus_dispatch,
        bench_state_replay_canonical_hash,
        bench_preset_recall_apply,
        bench_playlist_advance_apply,
);
criterion_main!(benches);
