use tri_game_spikes::energy::{Actor, EnergyScheduler};
use tri_game_spikes::kernel::{self, read_ledger};
use tri_game_spikes::logistics::{self, Dispatch};
use tri_game_spikes::path::{self, Map};
use tri_game_spikes::schedule::{self, Activity, Village, minute_of_day, scheduled};

#[test]
fn authoring_matches_frozen_demo_and_ledger_reader() {
    assert!(kernel::demo_matches_authoring());
    let sim = kernel::load_population(20, 1).unwrap();
    let ledger = read_ledger(&sim.domain_checkpoint_bytes()).unwrap();
    assert_eq!(ledger.actors.len(), 20);
    assert_eq!(ledger.stocks.len(), 10);
    assert_eq!(ledger.stocks[0].id, 1);
    assert_eq!(ledger.stocks[0].quantity, 1);
    assert_eq!(ledger.stocks.last().unwrap().quantity, 4);
    let mut dormant = kernel::load_population(20, 10_000).unwrap();
    let output = dormant.drive(0, &[]).unwrap();
    assert!(output.resource_events().is_empty());
}

#[test]
fn population_replay_is_stable_for_one_worker_schedule() {
    let report = kernel::measure_population(20, 1, 4, 0, 2).unwrap();
    assert!(report.decisions >= 20);
    assert_eq!(report.final_hash, {
        let again = kernel::measure_population(20, 1, 4, 0, 1).unwrap();
        again.final_hash
    });
}

#[test]
fn daily_clock_replays_across_restore_without_touching_kernel() {
    let sim = kernel::load_population(20, 1).unwrap();
    let before = sim.domain_checkpoint_bytes();
    let mut village = Village::settlement(20);
    let mut trace = Vec::new();
    for _ in 0..10_000 {
        trace.extend(village.observations().into_iter().map(|item| item as u8));
        village.advance(1);
    }
    let digest = tri_game_spikes::sha256(&trace);
    let saved = Village::from_bytes(&{
        let mut partial = Village::settlement(20);
        partial.advance(4_096);
        partial.to_bytes()
    })
    .unwrap();
    assert_eq!(saved.tick, 4_096);
    let mut resumed = saved;
    let mut other = Vec::new();
    let mut prefix = Village::settlement(20);
    for _ in 0..4_096 {
        other.extend(prefix.observations().into_iter().map(|item| item as u8));
        prefix.advance(1);
    }
    for _ in 4_096..10_000 {
        other.extend(resumed.observations().into_iter().map(|item| item as u8));
        resumed.advance(1);
    }
    assert_eq!(tri_game_spikes::sha256(&other), digest);
    assert_eq!(minute_of_day(0), 0);
    assert_eq!(scheduled(0), Activity::Sleep);
    assert_eq!(scheduled(360), Activity::Eat);
    assert_eq!(scheduled(420), Activity::Work);
    assert_eq!(scheduled(1020), Activity::Socialize);
    assert_eq!(scheduled(1260), Activity::Sleep);
    assert!(schedule::transition_count(20) > 20);
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    let farmer = village.residents.iter().find(|resident| resident.id == 1);
    assert!(farmer.is_some());
}

#[test]
fn energy_queue_skips_zero_speed_and_is_deterministic() {
    let mut scheduler = EnergyScheduler::new(1000);
    scheduler.insert(Actor {
        entity_id: 2,
        speed: 0,
        energy: 0,
    });
    assert_eq!(scheduler.advance_to_ready(), None);
    scheduler.insert(Actor {
        entity_id: 7,
        speed: 100,
        energy: 0,
    });
    scheduler.insert(Actor {
        entity_id: 3,
        speed: 100,
        energy: 0,
    });
    assert_eq!(scheduler.advance_to_ready(), Some(3));
    let first = scheduler.actors().to_vec();
    scheduler.consume_action(3, 1000);
    assert_eq!(scheduler.advance_to_ready(), Some(7));
    let mut again = EnergyScheduler::new(1000);
    again.insert(Actor {
        entity_id: 7,
        speed: 100,
        energy: 0,
    });
    again.insert(Actor {
        entity_id: 3,
        speed: 100,
        energy: 0,
    });
    again.insert(Actor {
        entity_id: 2,
        speed: 0,
        energy: 50,
    });
    assert_eq!(again.advance_to_ready(), Some(3));
    assert_eq!(again.actors()[0].energy, 50);
    assert_eq!(again.actors()[2].energy, first[1].energy);
}

#[test]
fn hierarchical_search_agrees_and_rejects_disconnected_cells() {
    let mut map = Map::from_open(8, 8, vec![true; 64]);
    map.open_fill_split();
    let (missing, stats) = map.hierarchical((1, 1), (6, 6));
    assert!(missing.is_none());
    assert_eq!(stats.tile_visits, 0);
    let open = Map::from_open(16, 16, vec![true; 256]);
    let (tile, _) = open.tile_bfs((1, 1), (14, 14));
    let (regional, _) = open.hierarchical((1, 1), (14, 14));
    assert!(path::paths_agree(
        &open,
        (1, 1),
        (14, 14),
        tile.as_deref(),
        regional.as_deref()
    ));
    let sample = path::demo_wall(32, 7);
    let queries = path::queries(&sample, 20, 11);
    assert_eq!(
        path::parallel_digest(&sample, &queries, 1),
        path::parallel_digest(&sample, &queries, 4)
    );
}

#[test]
fn shrine_haul_conserves_herbs_and_deduplicates_jobs() {
    let report = logistics::run_transfer().unwrap();
    assert!(report.rejected_drive_preserved);
    assert!(report.checkpoint_equal);
    assert_eq!(report.conserved, 12);
    assert!(report.shrine_quantity >= 5);
    assert!(report.dispatches >= 1);
    assert!(report.duplicates >= 1);
    let desk = logistics::Desk::new(5);
    assert_eq!(desk.demand, 5);
    let _ = Dispatch::Watching;
}

trait SplitMap {
    fn open_fill_split(&mut self);
}

impl SplitMap for Map {
    fn open_fill_split(&mut self) {
        let width = self.width;
        let height = self.height;
        let mut open = vec![true; (width * height) as usize];
        for y in 0..height {
            open[(y * width + width / 2) as usize] = false;
        }
        *self = Map::from_open(width, height, open);
    }
}
