//! Native restore measurement for the Qinghe living world.

use std::time::Instant;

use sealed_sim::living::{Intent, MAX_TICKS, World};

fn measure(label: &str, bytes: &[u8]) {
    let baseline = World::restore(bytes).expect("valid baseline checkpoint");
    assert_eq!(baseline.checkpoint().as_slice(), bytes);
    let mut samples = Vec::new();
    for _ in 0..20 {
        let start = Instant::now();
        let restored = World::restore(std::hint::black_box(bytes)).expect("replay succeeded");
        samples.push(start.elapsed());
        assert_eq!(restored, baseline);
    }
    samples.sort_unstable();
    println!(
        "{label}: turns={} incidents={} pending={} bytes={} median={:?} p95={:?}",
        baseline.tick(),
        baseline.social().incidents().len(),
        baseline.social().pending_reports().len(),
        bytes.len(),
        samples[10],
        samples[18],
    );
}

fn main() {
    let mut world = World::qinghe();
    for ticks in [4_096, 16_384, MAX_TICKS] {
        while world.tick() < ticks {
            world.step(world.tick(), Intent::Wait).expect("valid wait");
        }
        measure("Qinghe routines", &world.checkpoint());
    }
    for path in std::env::args_os().skip(1) {
        let bytes = std::fs::read(&path).expect("read checkpoint");
        measure(&path.to_string_lossy(), &bytes);
    }
}
