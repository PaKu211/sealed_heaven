//! Descriptive kernel benchmark; no render, save/load or wall-clock gameplay input.
use sealed_sim::living::{Intent, World, production::Job};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn run(production: bool) -> (f64, u64) {
    let mut w = if production {
        World::settlement_production_campaign(7)
    } else {
        World::exact_lod_campaign(7)
    };
    let start = Instant::now();
    for _ in 0..10000 {
        black_box(w.step(w.tick(), Intent::Wait).unwrap());
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let jobs = w.production().map_or(0, |p| {
        Job::ALL.iter().map(|&j| p.record_for(j).executions).sum()
    });
    black_box(w.state_digest());
    (elapsed, jobs)
}
fn median(values: &[f64]) -> f64 {
    let mut a = values.to_vec();
    a.sort_by(f64::total_cmp);
    a[a.len() / 2]
}
fn main() {
    let mut old = Vec::new();
    let mut new = Vec::new();
    let mut expected = None;
    for i in 0..6 {
        for production in if i % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let (ms, jobs) = run(production);
            if production {
                if let Some(j) = expected {
                    assert_eq!(j, jobs);
                }
                expected = Some(jobs);
                if i > 0 {
                    new.push(ms);
                }
            } else if i > 0 {
                old.push(ms);
            }
        }
    }
    let a = median(&old);
    let b = median(&new);
    println!(
        "{}",
        json!({"scope":"pure domain step, seed 7, 12 actual NPCs, three growth sites and one order flow; alternating order, one warmup discarded","seed":7,"turns_per_run":10000,"measured_runs_per_mode":5,"reference_ms":old,"production_ms":new,"reference_median_ms":a,"production_median_ms":b,"ratio":b/a,"due_jobs":expected.unwrap()})
    );
}
