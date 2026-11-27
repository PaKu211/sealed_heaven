//! Descriptive kernel benchmark; never influences canonical simulation.
use sealed_sim::living::{Intent, World};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn run(seed: u32, turns: u64, lod: bool) -> (f64, (u64, u64)) {
    let mut world = if lod {
        World::exact_lod_campaign(seed)
    } else {
        World::regional_knowledge_campaign(seed)
    };
    let start = Instant::now();
    for _ in 0..turns {
        black_box(world.step(world.tick(), Intent::Wait).unwrap());
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let count = world
        .scheduler()
        .map_or((turns * 12, 0), |s| s.evaluation_totals());
    assert_eq!(count.0 + count.1, turns * 12);
    black_box(world.state_digest());
    (elapsed, count)
}
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[sorted.len() / 2]
}
fn main() {
    const TURNS: u64 = 10000;
    const SEED: u32 = 7;
    let mut reference = Vec::new();
    let mut indexed = Vec::new();
    let mut totals = None;
    for i in 0..6 {
        for lod in if i % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let (ms, count) = run(SEED, TURNS, lod);
            if i > 0 {
                if lod {
                    if let Some(old) = totals {
                        assert_eq!(old, count);
                    }
                    totals = Some(count);
                    indexed.push(ms);
                } else {
                    reference.push(ms);
                }
            }
        }
    }
    let (full, routine) = totals.unwrap();
    let old = median(&reference);
    let new = median(&indexed);
    println!(
        "{}",
        json!({"scope":"pure domain step, seed 7, no renderer, no save/load; alternating order, one warmup discarded; 12 actual named NPCs","seed":SEED,"turns_per_run":TURNS,"measured_runs_per_mode":5,"reference_ms":reference,"lod_ms":indexed,"reference_median_ms":old,"lod_median_ms":new,"ratio":new/old,"full_evaluations":full,"routine_evaluations":routine})
    );
}
