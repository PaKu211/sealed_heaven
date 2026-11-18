//! Public-kernel measurements. Fixture bytes follow the accepted C0.3 authoring
//! already checked by `examples/c05_bench.rs`. This module does not edit them.

use sealed_sim::{Simulation, player5::World};
use std::time::Instant;

const DEMO: &[u8] = include_bytes!("../../../apps/sealed_playable/assets/demo.c03");

#[derive(Clone, Debug)]
pub struct TickReport {
    pub label: String,
    pub actors: u32,
    pub samples: Vec<u128>,
    pub decisions: u64,
    pub resource_events: u64,
    pub final_checkpoint_bytes: usize,
    pub final_hash: [u8; 32],
}

impl TickReport {
    pub fn summary(&self) -> TickSummary {
        let mut samples = self.samples.clone();
        samples.sort_unstable();
        let total: u128 = samples.iter().sum();
        TickSummary {
            label: self.label.clone(),
            actors: self.actors,
            samples: samples.len(),
            mean_ns: total / samples.len() as u128,
            p50_ns: crate::nearest_rank(&samples, 0.50),
            p95_ns: crate::nearest_rank(&samples, 0.95),
            max_ns: *samples.last().unwrap(),
            total_ns: total,
            unpaced_tps: (samples.len() as f64) * 1_000_000_000.0 / total as f64,
            decisions: self.decisions,
            resource_events: self.resource_events,
            final_checkpoint_bytes: self.final_checkpoint_bytes,
            final_hash: self.final_hash,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickSummary {
    pub label: String,
    pub actors: u32,
    pub samples: usize,
    pub mean_ns: u128,
    pub p50_ns: u128,
    pub p95_ns: u128,
    pub max_ns: u128,
    pub total_ns: u128,
    pub unpaced_tps: f64,
    pub decisions: u64,
    pub resource_events: u64,
    pub final_checkpoint_bytes: usize,
    pub final_hash: [u8; 32],
}

pub fn demo_matches_authoring() -> bool {
    authored_population(20, 1) == DEMO
}

pub fn load_population(actors: u32, first_wake: u64) -> Result<Simulation, String> {
    if !matches!(actors, 20 | 100 | 1000) {
        return Err("actors must be 20, 100, or 1000".into());
    }
    if !demo_matches_authoring() {
        return Err("20-actor authoring drifted from demo.c03".into());
    }
    let bytes = authored_population(actors, first_wake);
    let sim =
        Simulation::restore_domain_checkpoint(&bytes).map_err(|error| format!("{error:?}"))?;
    if sim.domain_checkpoint_bytes() != bytes {
        return Err("canonical roundtrip failed".into());
    }
    Ok(sim)
}

pub fn measure_population(
    actors: u32,
    first_wake: u64,
    ticks: usize,
    warmup: usize,
    iterations: usize,
) -> Result<TickReport, String> {
    let mut samples = Vec::with_capacity(ticks * iterations);
    let mut decisions = 0u64;
    let mut resource_events = 0u64;
    let mut final_hash = [0u8; 32];
    let mut final_checkpoint_bytes = 0usize;
    for _ in 0..iterations {
        let mut sim = load_population(actors, first_wake)?;
        for tick in 0..warmup {
            sim.drive(tick as u64, &[])
                .map_err(|error| format!("warmup {tick}: {error:?}"))?;
        }
        for offset in 0..ticks {
            let tick = (warmup + offset) as u64;
            let started = Instant::now();
            let output = std::hint::black_box(
                sim.drive(std::hint::black_box(tick), std::hint::black_box(&[])),
            );
            let elapsed = started.elapsed().as_nanos();
            let output = output.map_err(|error| format!("tick {tick}: {error:?}"))?;
            samples.push(elapsed);
            resource_events += output.resource_events().len() as u64;
            decisions += output
                .resource_events()
                .iter()
                .filter(|event| event.kind == 6)
                .count() as u64;
        }
        let bytes = sim.domain_checkpoint_bytes();
        final_checkpoint_bytes = bytes.len();
        final_hash = crate::sha256(&bytes);
    }
    let label = if first_wake == 1 {
        format!("npc-active-{actors}")
    } else {
        format!("npc-dormant-{actors}")
    };
    Ok(TickReport {
        label,
        actors,
        samples,
        decisions,
        resource_events,
        final_checkpoint_bytes,
        final_hash,
    })
}

pub fn measure_player(
    ticks: usize,
    warmup: usize,
    iterations: usize,
) -> Result<TickReport, String> {
    let mut samples = Vec::with_capacity(ticks * iterations);
    let mut decisions = 0u64;
    let mut resource_events = 0u64;
    let mut final_hash = [0u8; 32];
    let mut final_checkpoint_bytes = 0usize;
    for _ in 0..iterations {
        let mut world = World::settlement();
        for tick in 0..warmup {
            world
                .drive(tick as u64, &[])
                .map_err(|error| format!("player warmup {tick}: {error:?}"))?;
        }
        for offset in 0..ticks {
            let tick = (warmup + offset) as u64;
            let started = Instant::now();
            let output = std::hint::black_box(
                world.drive(std::hint::black_box(tick), std::hint::black_box(&[])),
            );
            let elapsed = started.elapsed().as_nanos();
            let output = output.map_err(|error| format!("player tick {tick}: {error:?}"))?;
            samples.push(elapsed);
            // `World::drive` returns player events. Embedded NPC decisions stay
            // inside the kernel and are counted on the direct population rows.
            resource_events += output.len() as u64;
            let _ = &mut decisions;
        }
        let bytes = world.checkpoint();
        final_checkpoint_bytes = bytes.len();
        final_hash = world.hash();
    }
    Ok(TickReport {
        label: "c05-player-20".into(),
        actors: 20,
        samples,
        decisions,
        resource_events,
        final_checkpoint_bytes,
        final_hash,
    })
}

fn authored_population(actors: u32, first_wake: u64) -> Vec<u8> {
    let n = u64::from(actors);
    let scenario = frozen_scenario();
    let mut out = b"SH:C03:STATE".to_vec();
    out.extend(1u16.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend(crate::sha256(&scenario));
    u64s(&mut out, &[0, 100, n + 1, 1, 1, n + 83]);
    out.extend(scenario);
    u64s(&mut out, &[n]);
    for id in 1..=actors {
        let goal = (id - 1) % 5;
        out.extend(id.to_le_bytes());
        out.extend([1, 1]);
        u64s(&mut out, &[first_wake]);
        for channel in 0..5 {
            out.extend((if channel == goal { 1000u32 } else { 0 }).to_le_bytes());
        }
        u64s(&mut out, &[0; 6]);
        u64s(
            &mut out,
            &[if matches!(goal, 1 | 3 | 4) { 2 } else { 1 }, 0],
        );
        out.push(1);
        u64s(&mut out, &[1]);
        out.extend(0u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        u64s(&mut out, &[0, 0]);
    }
    u64s(&mut out, &[3]);
    for (id, grid) in [(1u64, 1u8), (2, 0), (9, 0)] {
        u64s(&mut out, &[id]);
        out.push(grid);
    }
    u64s(&mut out, &[2]);
    for (id, kind) in [(20u64, 2u8), (30, 4)] {
        u64s(&mut out, &[id]);
        out.push(kind);
    }
    u64s(&mut out, &[2, 1, 2, 1, 1, 2, 1, 2, 1]);
    u64s(&mut out, &[1]);
    out.extend(3u32.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend([1, 1, 1, 1, 0, 1, 1, 1, 1]);
    u64s(&mut out, &[2 * n / 5 + 2]);
    for id in 1..=actors {
        match (id - 1) % 5 {
            0 => stock(&mut out, u64::from(id), 1, 1, 1, 1, u64::from(id)),
            3 => stock(&mut out, u64::from(id), 7, 1, 1, 1, u64::from(id)),
            _ => {}
        }
    }
    stock(&mut out, n + 81, 4, 2, n / 5, 3, 1);
    stock(&mut out, n + 82, 5, 2, n / 5, 3, 1);
    u64s(&mut out, &[0, 0, 0, 0, 0, 0, 1]);
    out.extend(crate::sha256(b"SH:C03:EVENT-INIT:v1"));
    let checksum = crate::sha256(&out);
    out.extend(checksum);
    out
}

pub fn frozen_scenario() -> Vec<u8> {
    let mut scenario = Vec::new();
    u64s(&mut scenario, &[4]);
    for row in 0..5 {
        for col in 0..5 {
            scenario.extend((if row == col { 1000i64 } else { 0 }).to_le_bytes());
        }
    }
    for _ in 0..5 {
        scenario.extend(i64::MIN.to_le_bytes());
    }
    u64s(&mut scenario, &[1, 1, 1, 1, 2, 1]);
    scenario.push(3);
    u64s(&mut scenario, &[2]);
    scenario
}

pub fn stock(
    out: &mut Vec<u8>,
    id: u64,
    definition: u64,
    class: u8,
    quantity: u64,
    kind: u8,
    owner: u64,
) {
    u64s(out, &[id, definition]);
    out.push(class);
    u64s(out, &[quantity]);
    out.push(kind);
    u64s(out, &[owner]);
}

pub fn u64s(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockView {
    pub id: u64,
    pub definition_id: u64,
    pub class: u8,
    pub quantity: u64,
    pub owner_kind: u8,
    pub owner_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorView {
    pub id: u32,
    pub task_id: u64,
    pub site: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerView {
    pub actors: Vec<ActorView>,
    pub stocks: Vec<StockView>,
    pub active_reservations: u64,
}

pub fn read_ledger(bytes: &[u8]) -> Result<LedgerView, String> {
    let body = bytes
        .get(..bytes.len().saturating_sub(32))
        .ok_or("checkpoint shorter than checksum")?;
    let mut cursor = Cursor {
        bytes: body,
        index: 0,
    };
    let tag = cursor.take(12)?;
    if tag != b"SH:C03:STATE" {
        return Err("not a C0.3 checkpoint".into());
    }
    cursor.skip(2 + 4 + 32 + 8 * 6)?;
    cursor.skip(frozen_scenario().len())?;
    let actor_count = cursor.u64()? as usize;
    let mut actors = Vec::with_capacity(actor_count);
    for _ in 0..actor_count {
        let id = cursor.u32()?;
        cursor.skip(1 + 1 + 8 + 20 + 48)?;
        cursor.u64()?;
        let task_id = cursor.u64()?;
        cursor.u8()?;
        let site = cursor.u64()?;
        cursor.skip(4 + 4 + 8 + 8)?;
        actors.push(ActorView { id, task_id, site });
    }
    let sites = cursor.u64()? as usize;
    for _ in 0..sites {
        cursor.skip(8)?;
        cursor.u8()?;
    }
    let holders = cursor.u64()? as usize;
    for _ in 0..holders {
        cursor.skip(8)?;
        cursor.u8()?;
    }
    let edges = cursor.u64()? as usize;
    cursor.skip(edges * 32)?;
    cursor.u64()?;
    let width = cursor.u32()?;
    let height = cursor.u32()?;
    cursor.skip((width as usize).saturating_mul(height as usize))?;
    let stock_count = cursor.u64()? as usize;
    let mut stocks = Vec::with_capacity(stock_count);
    for _ in 0..stock_count {
        stocks.push(StockView {
            id: cursor.u64()?,
            definition_id: cursor.u64()?,
            class: cursor.u8()?,
            quantity: cursor.u64()?,
            owner_kind: cursor.u8()?,
            owner_id: cursor.u64()?,
        });
    }
    let reservation_count = cursor.u64()? as usize;
    let mut active_reservations = 0u64;
    for _ in 0..reservation_count {
        cursor.skip(8 + 8 + 4 + 8 + 8 + 8 + 8)?;
        if cursor.u8()? == 1 {
            active_reservations += 1;
        }
    }
    Ok(LedgerView {
        actors,
        stocks,
        active_reservations,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .index
            .checked_add(len)
            .ok_or("cursor overflow")?
            .min(self.bytes.len());
        if end - self.index != len {
            return Err("truncated checkpoint".into());
        }
        let slice = &self.bytes[self.index..end];
        self.index = end;
        Ok(slice)
    }

    fn skip(&mut self, len: usize) -> Result<(), String> {
        self.take(len).map(|_| ())
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}
