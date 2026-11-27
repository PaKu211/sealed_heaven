//! Descriptive kernel benchmark; C4.5 has different gameplay, not a speed-only change.
use sealed_sim::living::{Intent, World};
use serde_json::json;
use std::{hint::black_box, time::Instant};
fn run(history: bool) -> (f64, usize) {
    let mut w = if history {
        World::road_history_campaign(7)
    } else {
        World::settlement_production_campaign(7)
    };
    let start = Instant::now();
    for _ in 0..10000 {
        black_box(w.step(w.tick(), Intent::Wait).unwrap());
    }
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let resolved = w.road_history().map_or(0, |h| {
        h.caches().iter().filter(|c| c.resolution.is_some()).count()
    });
    black_box(w.state_digest());
    (elapsed, resolved)
}
fn median(values: &[f64]) -> f64 {
    let mut a = values.to_vec();
    a.sort_by(f64::total_cmp);
    a[a.len() / 2]
}
fn main() {
    let mut old = Vec::new();
    let mut new = Vec::new();
    let mut resolved = None;
    for i in 0..6 {
        for history in if i % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let (ms, n) = run(history);
            if history {
                if let Some(r) = resolved {
                    assert_eq!(r, n);
                }
                resolved = Some(n);
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
        json!({"scope":"domain step only, seed 7, 12 NPCs, no renderer/save/load; alternating order and one warmup discarded; gameplay differs across revisions","seed":7,"turns_per_run":10000,"measured_runs_per_mode":5,"reference_ms":old,"history_ms":new,"reference_median_ms":a,"history_median_ms":b,"ratio":b/a,"resolved_caches":resolved.unwrap()})
    );
}
