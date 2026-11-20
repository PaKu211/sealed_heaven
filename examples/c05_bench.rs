//! Dependency-free benchmark driver, NOT a simulation implementation.
//! Run a release build; see C0_5_PERFORMANCE_BASELINE.md for fixture/clock limits.
use sealed_sim::{Simulation, player5::World};
use sha2::{Digest, Sha256};
use std::{fmt::Write, hint::black_box, time::Instant};

const DEMO: &[u8] = include_bytes!("../apps/sealed_playable/assets/demo.c03");
const HELP: &str = "c05_bench [--mode npc|c05] [--actors 20|100|1000] [--ticks 100] [--warmup 10] [--iterations 1]\n\
    Each iteration starts fresh. Warmup advances that world, but is not timed.\n\
    npc = frozen Simulation::drive; c05 = complete World::drive with no player input.\n\
    C05 supports only its authored 20-NPC settlement; other sizes exit 3 (unavailable).";

#[derive(Clone, Debug)]
struct Options {
    mode: String,
    actors: u32,
    ticks: usize,
    warmup: usize,
    iterations: usize,
}

impl Options {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let mut options = Self {
            mode: "npc".into(),
            actors: 20,
            ticks: 100,
            warmup: 10,
            iterations: 1,
        };
        let mut args = args;
        while let Some(key) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value for {key}"))?;
            match key.as_str() {
                "--mode" if value == "npc" || value == "c05" => options.mode = value,
                "--actors" => options.actors = value.parse().map_err(|_| "invalid actors")?,
                "--ticks" => options.ticks = value.parse().map_err(|_| "invalid ticks")?,
                "--warmup" => options.warmup = value.parse().map_err(|_| "invalid warmup")?,
                "--iterations" => {
                    options.iterations = value.parse().map_err(|_| "invalid iterations")?
                }
                _ => return Err(format!("unknown option or value: {key} {value}")),
            }
        }
        if !matches!(options.actors, 20 | 100 | 1000) {
            return Err("actors must be 20, 100, or 1000".into());
        }
        if options.ticks == 0 || options.iterations == 0 {
            return Err("ticks and iterations must be positive".into());
        }
        options
            .ticks
            .checked_add(options.warmup)
            .ok_or("tick count overflow")?;
        options
            .ticks
            .checked_mul(options.iterations)
            .ok_or("sample count overflow")?;
        Ok(options)
    }
}

fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn u64s(out: &mut Vec<u8>, values: &[u64]) {
    for value in values {
        out.extend(value.to_le_bytes());
    }
}

fn stock(
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

/// Offline initial-state authoring only. Transcribes pack_demo.py and the C03
/// contract packer (docs/tools/c03_vectors.py); it never generates future states.
/// Actors repeat five mixed goals in 20-actor cohorts in ONE shared world.
/// Shared tool/station capacity is 4 per cohort, not extra independent worlds.
fn authored_fixture(actors: u32) -> Vec<u8> {
    assert!(matches!(actors, 20 | 100 | 1000));
    let n = u64::from(actors);
    let mut scenario = Vec::new();
    u64s(&mut scenario, &[4]); // frozen S4 wake interval
    for row in 0..5 {
        for col in 0..5 {
            scenario.extend((if row == col { 1000i64 } else { 0i64 }).to_le_bytes());
        }
    }
    for _ in 0..5 {
        scenario.extend(i64::MIN.to_le_bytes());
    }
    u64s(&mut scenario, &[1, 1, 1, 1, 2, 1]); // one frozen recipe
    scenario.push(3);
    u64s(&mut scenario, &[2]);

    let mut out = b"SH:C03:STATE".to_vec();
    out.extend(1u16.to_le_bytes());
    out.extend(3u32.to_le_bytes());
    out.extend(sha(&scenario));
    // tick, seed, next actor/record/transaction/stock identities. Offsetting the
    // shared stock IDs by 80 retains demo.c03's IDs 101/102 at n=20.
    u64s(&mut out, &[0, 100, n + 1, 1, 1, n + 83]);
    out.extend(scenario);
    u64s(&mut out, &[n]);
    for id in 1..=actors {
        let goal = (id - 1) % 5;
        out.extend(id.to_le_bytes());
        out.extend([1, 1]); // enabled, active processing
        u64s(&mut out, &[1]); // first wake
        for channel in 0..5 {
            out.extend((if channel == goal { 1000u32 } else { 0u32 }).to_le_bytes());
        }
        u64s(&mut out, &[0; 6]); // roles, relationship
        u64s(
            &mut out,
            &[if matches!(goal, 1 | 3 | 4) { 2 } else { 1 }, 0],
        );
        out.push(1); // site locus
        u64s(&mut out, &[1]);
        out.extend(0u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        u64s(&mut out, &[0, 0]); // edge, traversal
    }
    u64s(&mut out, &[3]);
    for (id, grid) in [(1, 1), (2, 0), (9, 0)] {
        u64s(&mut out, &[id]);
        out.push(grid);
    }
    u64s(&mut out, &[2]);
    for (id, kind) in [(20, 2), (30, 4)] {
        u64s(&mut out, &[id]);
        out.push(kind);
    }
    u64s(&mut out, &[2, 1, 2, 1, 1, 2, 1, 2, 1]); // two directed edges
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
    // reservations, tasks, transactions, registry, pending, traces, next event.
    u64s(&mut out, &[0, 0, 0, 0, 0, 0, 1]);
    out.extend(sha(b"SH:C03:EVENT-INIT:v1"));
    let checksum = sha(&out);
    out.extend(checksum);
    out
}

/// Rejects a fixture unless the kernel accepts and canonically roundtrips it.
/// The independent FIRST-TICK oracle requires one desired-goal decision per
/// actual actor, each score 1,000,000 and wake 5; it does not synthesize AI.
fn validated_fixture(actors: u32) -> Result<(Simulation, String), String> {
    if authored_fixture(20) != DEMO {
        return Err("fixture authoring no longer matches immutable demo.c03".into());
    }
    let bytes = if actors == 20 {
        DEMO.to_vec()
    } else {
        authored_fixture(actors)
    };
    let sim = Simulation::restore_domain_checkpoint(&bytes)
        .map_err(|e| format!("fixture restore: {e:?}"))?;
    if sim.tick() != 0 || sim.domain_checkpoint_bytes() != bytes {
        return Err("initial fixture canonical roundtrip failed".into());
    }
    for id in 1..=actors {
        let expected = match (id - 1) % 5 {
            0 | 3 => Some(1),
            _ => None,
        };
        if sim.resource_stock_quantity(u64::from(id)) != expected {
            return Err(format!("initial stock oracle failed for actor {id}"));
        }
    }
    for id in [u64::from(actors) + 81, u64::from(actors) + 82] {
        if sim.resource_stock_quantity(id) != Some(u64::from(actors) / 5) {
            return Err("shared resource capacity oracle failed".into());
        }
    }
    let mut probe = sim.clone();
    let output = probe
        .drive(0, &[])
        .map_err(|e| format!("first-tick probe: {e:?}"))?;
    let decisions: Vec<_> = output
        .resource_events()
        .iter()
        .filter(|e| e.kind == 6)
        .collect();
    if decisions.len() != actors as usize {
        return Err("first-tick actor count mismatch".into());
    }
    for (index, event) in decisions.iter().enumerate() {
        let goal = index % 5;
        let mut expected = vec![goal as u8 + 1];
        // Each prior actor allocates one task and 1, 0, 2, 1, 0 reservations.
        let prior_reservations = (index / 5 * 4) + [0, 1, 1, 3, 4][goal];
        u64s(&mut expected, &[(1 + index + prior_reservations) as u64, 5]);
        for score in 0..5 {
            expected.extend((if score == goal { 1_000_000i64 } else { 0i64 }).to_le_bytes());
        }
        if event.actor_id != index as u32 + 1 || event.tick != 1 || event.payload != expected {
            return Err(format!(
                "first-tick mixed-goal oracle failed at actor {}",
                index + 1
            ));
        }
    }
    let probe_bytes = probe.domain_checkpoint_bytes();
    let restored = Simulation::restore_domain_checkpoint(&probe_bytes)
        .map_err(|e| format!("first-tick checkpoint invalid: {e:?}"))?;
    if restored.domain_checkpoint_bytes() != probe_bytes {
        return Err("first-tick checkpoint roundtrip mismatch".into());
    }
    Ok((sim, hex(&sha(&bytes))))
}

#[allow(clippy::large_enum_variant)]
enum Subject {
    Npc(Simulation),
    C05(World),
}

impl Subject {
    fn new(options: &Options) -> Result<(Self, String), String> {
        let (npc, hash) = validated_fixture(options.actors)?;
        if options.mode == "npc" {
            return Ok((Self::Npc(npc), hash));
        }
        let world = World::settlement();
        if world.npcs().domain_checkpoint_bytes() != DEMO {
            return Err("C05 settlement does not embed canonical demo.c03".into());
        }
        Ok((Self::C05(world), hash))
    }
    fn tick(&self) -> u64 {
        match self {
            Self::Npc(s) => s.tick(),
            Self::C05(w) => w.tick(),
        }
    }
    fn npcs(&self) -> &Simulation {
        match self {
            Self::Npc(s) => s,
            Self::C05(w) => w.npcs(),
        }
    }
    fn checkpoint(&self) -> Vec<u8> {
        match self {
            Self::Npc(s) => s.domain_checkpoint_bytes(),
            Self::C05(w) => w.checkpoint(),
        }
    }
    fn step(&mut self) -> Result<u64, String> {
        let boundary = self.tick();
        // Timing is confined to the public call, including its internal cloning,
        // validation and serialization. Returned event destruction is excluded.
        match self {
            Self::Npc(sim) => {
                let start = Instant::now();
                let result = black_box(sim).drive(boundary, black_box(&[]));
                let ns = start.elapsed().as_nanos() as u64;
                black_box(result.map_err(|e| format!("NPC drive: {e:?}"))?);
                Ok(ns)
            }
            Self::C05(world) => {
                let start = Instant::now();
                let result = black_box(world).drive(boundary, black_box(&[]));
                let ns = start.elapsed().as_nanos() as u64;
                black_box(result.map_err(|e| format!("C05 drive: {e:?}"))?);
                Ok(ns)
            }
        }
    }
    fn validate_final(&self, bytes: &[u8]) -> Result<(), String> {
        let roundtrip = match self {
            Self::Npc(_) => Simulation::restore_domain_checkpoint(bytes)
                .map_err(|e| format!("final NPC restore: {e:?}"))?
                .domain_checkpoint_bytes(),
            Self::C05(_) => World::restore(bytes)
                .map_err(|e| format!("final C05 restore: {e:?}"))?
                .checkpoint(),
        };
        if roundtrip != bytes {
            return Err("final checkpoint roundtrip mismatch".into());
        }
        Ok(())
    }
}

fn percentile(sorted: &[u64], percent: usize) -> u64 {
    sorted[(sorted.len() * percent).div_ceil(100).saturating_sub(1)]
}

fn stats(samples: &[u64]) -> String {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let total: u128 = sorted.iter().map(|n| u128::from(*n)).sum();
    let tps = if total == 0 {
        "null".into()
    } else {
        format!("{:.6}", sorted.len() as f64 * 1e9 / total as f64)
    };
    format!(
        "{{\"samples\":{},\"total_drive_ns\":{total},\"mean_ns\":{:.3},\"p50_ns\":{},\"p95_ns\":{},\"p99_ns\":{},\"max_ns\":{},\"unpaced_drive_tps\":{tps}}}",
        sorted.len(),
        total as f64 / sorted.len() as f64,
        percentile(&sorted, 50),
        percentile(&sorted, 95),
        percentile(&sorted, 99),
        sorted.last().unwrap()
    )
}

fn benchmark(options: &Options) -> Result<String, String> {
    let mut runs = Vec::new();
    let mut all_samples = Vec::new();
    for iteration in 0..options.iterations {
        let (mut subject, fixture_sha256) = Subject::new(options)?;
        let initial = subject.checkpoint();
        let initial_sha256 = hex(&sha(&initial));
        let initial_len = initial.len();
        drop(initial);
        for _ in 0..options.warmup {
            subject.step()?;
        }
        let before = subject.checkpoint().len();
        let first_tick = subject.tick();
        let first_event = subject.npcs().next_event_id();
        let mut samples = Vec::with_capacity(options.ticks);
        let loop_start = Instant::now();
        for _ in 0..options.ticks {
            samples.push(subject.step()?);
        }
        let loop_wall_ns = loop_start.elapsed().as_nanos();
        if subject.tick() != (options.warmup + options.ticks) as u64 {
            return Err("unexpected number of committed ticks".into());
        }
        let final_checkpoint = subject.checkpoint();
        subject.validate_final(&final_checkpoint)?;
        let raw = samples
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let mut run = String::new();
        write!(run, "{{\"iteration\":{},\"fixture_sha256\":\"{fixture_sha256}\",\"initial_sha256\":\"{initial_sha256}\",\"final_sha256\":\"{}\",\"npc_final_sha256\":\"{}\",\"npc_event_digest\":\"{}\",\"first_measured_boundary\":{first_tick},\"final_tick\":{},\"measured_npc_events\":{},\"checkpoint_initial_bytes\":{initial_len},\"checkpoint_post_warmup_bytes\":{before},\"checkpoint_final_bytes\":{},\"checkpoint_measured_growth_bytes\":{},\"checkpoint_total_growth_bytes\":{},\"loop_wall_ns\":{loop_wall_ns},\"loop_wall_tps\":{:.6},\"timing\":{},\"drive_ns\":[{raw}]}}",
            iteration + 1, hex(&sha(&final_checkpoint)), hex(&subject.npcs().domain_checkpoint_sha256()),
            hex(&subject.npcs().event_digest()), subject.tick(), subject.npcs().next_event_id() - first_event,
            final_checkpoint.len(), final_checkpoint.len() as i128 - before as i128,
            final_checkpoint.len() as i128 - initial_len as i128,
            options.ticks as f64 * 1e9 / loop_wall_ns.max(1) as f64, stats(&samples)).unwrap();
        runs.push(run);
        all_samples.extend(samples);
    }
    Ok(format!(
        "{{\"schema\":\"c05-benchmark-v1\",\"status\":\"validated\",\"profile\":\"{}\",\"mode\":\"{}\",\"actors\":{},\"ticks_per_iteration\":{},\"warmup_per_iteration\":{},\"iterations\":{},\"commands_per_tick\":0,\"fixture\":\"s4-mixed-goals-shared-topology-capacity-scaled\",\"fixture_oracle\":\"demo20-byte-equality+kernel-restore+first-tick-all-actors+canonical-roundtrip\",\"percentiles\":\"nearest-rank\",\"aggregate\":{},\"runs\":[{}]}}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        options.mode,
        options.actors,
        options.ticks,
        options.warmup,
        options.iterations,
        stats(&all_samples),
        runs.join(",")
    ))
}

fn main() {
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("{HELP}");
        return;
    }
    let result = Options::parse(std::env::args().skip(1)).and_then(|options| {
        if options.mode == "c05" && options.actors != 20 {
            println!("{{\"schema\":\"c05-benchmark-v1\",\"status\":\"unavailable\",\"mode\":\"c05\",\"actors\":{},\"reason\":\"C05 World::settlement is the authored 20-NPC world; no validated scaled aggregate constructor\"}}", options.actors);
            std::process::exit(3);
        }
        benchmark(&options)
    });
    match result {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("{error}\n{HELP}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_twenty_is_the_immutable_c04_fixture() {
        assert_eq!(authored_fixture(20), DEMO);
    }

    #[test]
    fn genuine_scaled_fixtures_pass_initial_and_first_tick_oracles() {
        for actors in [20, 100, 1000] {
            validated_fixture(actors).unwrap();
        }
    }

    #[test]
    fn scaled_worlds_continue_and_restore_without_synthetic_steps() {
        for actors in [100, 1000] {
            let (mut sim, _) = validated_fixture(actors).unwrap();
            for boundary in 0..12 {
                sim.drive(boundary, &[]).unwrap();
            }
            let bytes = sim.domain_checkpoint_bytes();
            let mut restored = Simulation::restore_domain_checkpoint(&bytes).unwrap();
            assert_eq!(restored.domain_checkpoint_bytes(), bytes);
            assert_eq!(sim.drive(12, &[]), restored.drive(12, &[]));
            assert_eq!(
                sim.domain_checkpoint_bytes(),
                restored.domain_checkpoint_bytes()
            );
        }
    }

    #[test]
    fn percentiles_are_nearest_rank_not_interpolated() {
        let samples: Vec<u64> = (1..=100).collect();
        assert_eq!(
            (
                percentile(&samples, 50),
                percentile(&samples, 95),
                percentile(&samples, 99)
            ),
            (50, 95, 99)
        );
        assert_eq!(percentile(&[42], 99), 42);
    }

    #[test]
    fn invalid_and_zero_counts_fail_before_benchmarking() {
        for args in [
            vec!["--ticks", "0"],
            vec!["--iterations", "0"],
            vec!["--actors", "21"],
            vec!["--mode", "fake"],
        ] {
            assert!(Options::parse(args.into_iter().map(String::from)).is_err());
        }
    }
}
