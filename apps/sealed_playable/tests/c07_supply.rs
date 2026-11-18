use sealed_playable::agency::{Action, Session};
use sealed_playable::locale::Locale;
use sealed_playable::supply::{self, Phase, SCENARIO_DEMAND, SCENARIO_HERBS, Supply};
use sealed_sim::Simulation;

#[test]
fn five_deliveries_conserve_twelve_herbs() {
    let mut supply = Supply::shrine();
    let status = supply.run_until_met().unwrap();
    assert_eq!(status.demand, SCENARIO_DEMAND);
    assert_eq!(status.shrine, 5);
    assert_eq!(status.carrier, 7);
    assert_eq!(status.total, SCENARIO_HERBS);
    assert_eq!(status.deliveries, 5);
    assert!(status.duplicate_observations >= 1);
    assert_eq!(status.phase, Phase::Completed);
    assert_eq!(status.task_id, 0);
    let again = supply.checkpoint();
    for _ in 0..10 {
        supply.step().unwrap();
    }
    let held = supply.status().unwrap();
    assert_eq!(held.shrine, 5);
    assert_eq!(held.total, 12);
    let loaded = Supply::restore(&supply.checkpoint(), 5).unwrap();
    assert_eq!(loaded.status().unwrap().deliveries, 5);
    assert_eq!(loaded.checkpoint(), supply.checkpoint());
}

#[test]
fn a_second_demand_does_not_change_the_first() {
    let mut supply = Supply::shrine();
    assert!(!supply.request(9));
    assert_eq!(supply.demand(), 5);
    assert_eq!(supply.status().unwrap().total, 12);
}

#[test]
fn the_same_open_task_is_not_a_second_job() {
    let mut supply = Supply::shrine();
    let mut saw_duplicate = false;
    for _ in 0..40 {
        let before = supply.status().unwrap().duplicate_observations;
        supply.step().unwrap();
        if supply.status().unwrap().duplicate_observations > before {
            saw_duplicate = true;
            break;
        }
    }
    assert!(saw_duplicate);
    assert_eq!(supply.status().unwrap().total, 12);
}

#[test]
fn two_herbs_cannot_satisfy_a_demand_of_five() {
    let mut supply = Supply::open(2, 5);
    for _ in 0..40 {
        supply.step().unwrap();
    }
    let status = supply.status().unwrap();
    assert_eq!(status.total, 2);
    assert!(status.shrine <= 2);
    assert_ne!(status.phase, Phase::Completed);
}

#[test]
fn an_unknown_worker_rolls_back() {
    let mut supply = Supply::shrine();
    let before = supply.checkpoint();
    supply.reject_unknown().unwrap();
    assert_eq!(supply.checkpoint(), before);
    assert_eq!(supply.status().unwrap().phase, Phase::Rejected);
    assert_eq!(supply.status().unwrap().total, 12);
}

#[test]
fn cancelling_the_open_haul_keeps_the_herb_total() {
    let mut supply = Supply::shrine();
    for _ in 0..30 {
        if supply.status().unwrap().task_id != 0 {
            break;
        }
        supply.step().unwrap();
    }
    let before = supply.status().unwrap();
    assert_ne!(before.task_id, 0);
    supply.cancel_open().unwrap();
    let after = supply.status().unwrap();
    assert_eq!(after.task_id, 0);
    assert_eq!(after.total, before.total);
    assert_eq!(after.phase, Phase::Cancelled);
}

#[test]
fn save_load_and_replay_match() {
    let mut supply = Supply::shrine();
    for _ in 0..8 {
        supply.step().unwrap();
    }
    let saved = supply.checkpoint();
    let mut restored = Supply::restore(&saved, supply.demand()).unwrap();
    let mut replay = Supply::restore(
        &Simulation::restore_domain_checkpoint(&saved)
            .unwrap()
            .domain_checkpoint_bytes(),
        5,
    )
    .unwrap();
    for _ in 0..6 {
        supply.step().unwrap();
        restored.step().unwrap();
        replay.step().unwrap();
    }
    assert_eq!(restored.checkpoint(), supply.checkpoint());
    assert_eq!(replay.checkpoint(), supply.checkpoint());
    assert_eq!(supply.status().unwrap().total, 12);
}

#[test]
fn the_panel_reads_the_checkpoint_not_a_private_balance() {
    let supply = Supply::shrine();
    let status = supply.status().unwrap();
    assert_eq!(status.carrier, 12);
    assert_eq!(status.shrine, 0);
    assert_eq!(status.worker, supply::HAULER);
    assert_eq!(status.phase, Phase::Requested);
    let _ = supply::HERB;
}

#[test]
fn the_session_panel_follows_the_checkpoint_through_save_and_language() {
    let mut session = Session::default();
    let dir = tempfile::tempdir().unwrap();
    session.save_path = dir.path().join("checkpoint.c03");
    let untouched = session.supply().checkpoint();
    session.act(Action::Language(Locale::Indonesian));
    assert_eq!(session.supply().checkpoint(), untouched);
    for _ in 0..40 {
        session.act(Action::Step);
        if session.supply().status().unwrap().phase == Phase::Completed {
            break;
        }
    }
    let status = session.supply().status().unwrap();
    assert_eq!(status.shrine, 5);
    assert_eq!(status.total, 12);
    assert_eq!(status.deliveries, 5);
    let saved = session.supply().checkpoint();
    session.act(Action::Save);
    session.act(Action::Reset);
    assert_eq!(session.supply().status().unwrap().shrine, 0);
    session.act(Action::Load);
    assert_eq!(session.supply().checkpoint(), saved);
    assert_eq!(session.supply().status().unwrap().deliveries, 5);
    assert_eq!(session.locale, Locale::Indonesian);
}
