use std::env;
use std::time::Instant;

use tri_game_spikes::kernel::{self, TickSummary};
use tri_game_spikes::logistics;
use tri_game_spikes::path;
use tri_game_spikes::schedule::{self, Village};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("baseline") => baseline(args),
        Some("experiments") => experiments(),
        Some("memory") => memory_probe(),
        Some("help") | None => {
            eprintln!(
                "tri_game baseline|experiments|memory\n\
                 baseline writes one JSON object for --actors 20|100|1000 [--dormant] [--ticks 100] [--warmup 10] [--iterations 3]\n\
                 player baseline: --actors 20 --player"
            );
        }
        Some(other) => {
            eprintln!("unknown command {other}");
            std::process::exit(2);
        }
    }
}

fn baseline(args: impl Iterator<Item = String>) {
    let mut actors = 20u32;
    let mut ticks = 100usize;
    let mut warmup = 10usize;
    let mut iterations = 3usize;
    let mut dormant = false;
    let mut player = false;
    let mut values = args;
    while let Some(key) = values.next() {
        match key.as_str() {
            "--dormant" => dormant = true,
            "--player" => player = true,
            "--actors" | "--ticks" | "--warmup" | "--iterations" => {
                let value = values.next().unwrap_or_else(|| exit_arg());
                match key.as_str() {
                    "--actors" => actors = value.parse().unwrap_or_else(|_| exit_arg()),
                    "--ticks" => ticks = value.parse().unwrap_or_else(|_| exit_arg()),
                    "--warmup" => warmup = value.parse().unwrap_or_else(|_| exit_arg()),
                    "--iterations" => iterations = value.parse().unwrap_or_else(|_| exit_arg()),
                    _ => exit_arg(),
                }
            }
            _ => exit_arg(),
        }
    }
    let report = if player {
        if actors != 20 {
            eprintln!("status: unavailable");
            eprintln!(
                "C05 aggregate populations other than the authored 20 NPCs are not supported"
            );
            std::process::exit(3);
        }
        kernel::measure_player(ticks, warmup, iterations).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        })
    } else {
        let wake = if dormant { 100_000 } else { 1 };
        kernel::measure_population(actors, wake, ticks, warmup, iterations).unwrap_or_else(
            |error| {
                eprintln!("{error}");
                std::process::exit(1);
            },
        )
    };
    println!(
        "{},\"peak_working_set\":{}}}",
        summary_json(&report.summary()).trim_end_matches('}'),
        peak_working_set()
    );
}

fn experiments() {
    let mut village = Village::settlement(20);
    let started = Instant::now();
    village.advance(10_000);
    let schedule_ns = started.elapsed().as_nanos();
    let transitions = schedule::transition_count(20);
    let transfer = logistics::run_transfer().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    println!("{{");
    println!("  \"schedule_20_advance_10000_ticks_ns\": {schedule_ns},");
    println!("  \"schedule_20_transitions_per_day\": {transitions},");
    println!("  \"ticks_per_minute\": {},", schedule::TICKS_PER_MINUTE);
    println!("  \"logistics_ticks\": {},", transfer.ticks);
    println!("  \"logistics_shrine\": {},", transfer.shrine_quantity);
    println!("  \"logistics_conserved\": {},", transfer.conserved);
    println!("  \"logistics_dispatches\": {},", transfer.dispatches);
    println!("  \"logistics_duplicates\": {},", transfer.duplicates);
    println!(
        "  \"logistics_checkpoint_equal\": {},",
        transfer.checkpoint_equal
    );
    println!(
        "  \"logistics_rejection_preserved\": {},",
        transfer.rejected_drive_preserved
    );
    let sizes = [8u32, 64, 128, 256];
    for (index, size) in sizes.into_iter().enumerate() {
        let queries = if size == 8 { 50 } else { 500 };
        let result = path::benchmark(size, queries, 0x005E_A1ED);
        let comma = if index + 1 == sizes.len() { "" } else { "," };
        println!("  \"path_{size}\": {{");
        println!("    \"queries\": {},", result.queries);
        println!("    \"tile_mean_ns\": {},", result.tile_mean_ns);
        println!("    \"tile_p95_ns\": {},", result.tile_p95_ns);
        println!(
            "    \"hierarchical_mean_ns\": {},",
            result.hierarchical_mean_ns
        );
        println!(
            "    \"hierarchical_p95_ns\": {},",
            result.hierarchical_p95_ns
        );
        println!("    \"tile_mean_visits\": {:.2},", result.tile_mean_visits);
        println!(
            "    \"hierarchical_mean_visits\": {:.2},",
            result.hierarchical_mean_visits
        );
        println!("    \"agreement\": {},", result.agreement);
        println!("    \"fallbacks\": {},", result.hierarchical_fallbacks);
        println!(
            "    \"component_rejections\": {},",
            result.component_rejections
        );
        println!("    \"invalidation_ns\": {},", result.invalidation_ns);
        println!("    \"invalidated_cells\": {},", result.invalidated_cells);
        println!("    \"cache_drop_ns\": {},", result.cache_drop_ns);
        println!("    \"cached_paths_remaining\": {}", result.cached_paths);
        println!("  }}{comma}");
    }
    println!("}}");
}

fn memory_probe() {
    let before = peak_working_set();
    let _ = kernel::measure_population(1000, 1, 20, 1, 1);
    let after = peak_working_set();
    println!("{{\"before_peak_working_set\": {before}, \"after_peak_working_set\": {after}}}");
}

fn summary_json(summary: &TickSummary) -> String {
    format!(
        "{{\"label\":\"{}\",\"actors\":{},\"samples\":{},\"mean_ns\":{},\"p50_ns\":{},\"p95_ns\":{},\"max_ns\":{},\"total_ns\":{},\"unpaced_tps\":{:.6},\"decisions\":{},\"resource_events\":{},\"final_checkpoint_bytes\":{},\"final_hash\":\"{}\"}}",
        summary.label,
        summary.actors,
        summary.samples,
        summary.mean_ns,
        summary.p50_ns,
        summary.p95_ns,
        summary.max_ns,
        summary.total_ns,
        summary.unpaced_tps,
        summary.decisions,
        summary.resource_events,
        summary.final_checkpoint_bytes,
        summary
            .final_hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn peak_working_set() -> usize {
    #[repr(C)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut core::ffi::c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut core::ffi::c_void,
            counters: *mut Counters,
            cb: u32,
        ) -> i32;
    }
    unsafe {
        let mut counters = Counters {
            cb: size_of::<Counters>() as u32,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };
        let ok = K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb);
        if ok == 0 {
            0
        } else {
            counters.peak_working_set_size
        }
    }
}

fn exit_arg() -> ! {
    eprintln!("invalid baseline arguments");
    std::process::exit(2);
}
