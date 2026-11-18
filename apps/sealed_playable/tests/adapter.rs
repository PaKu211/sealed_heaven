use bevy::prelude::*;
use sealed_playable::{
    Action, DEMO, FrameInput, Session, SimulationPlugin, TICK_NS, TickEvidence, locale::Locale,
    view::View,
};
use sealed_sim::{ActorId, Command, CommandKey, Payload, ProcessingMode, Simulation};
use std::collections::BTreeMap;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(SimulationPlugin);
    app
}
fn frame(app: &mut App, ns: u128, actions: Vec<Action>) {
    *app.world_mut().resource_mut::<FrameInput>() = FrameInput {
        elapsed_ns: ns,
        actions,
    };
    app.update();
}
fn command(tick: u64, actor: u32, sequence: u64, payload: Payload) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: ActorId::new(actor),
            producer_id: 1,
            sequence,
        },
        payload,
    )
}
fn schedule() -> BTreeMap<u64, Vec<Command>> {
    BTreeMap::from([
        (
            0,
            vec![command(
                1,
                2,
                1,
                Payload::SetMode(ProcessingMode::Scheduled),
            )],
        ),
        (10, vec![command(11, 20, 2, Payload::SetEnabled(0))]),
        (15, vec![command(16, 20, 3, Payload::SetEnabled(1))]),
        // A future admitted command ensures save/load retains pending state.
        (
            1,
            vec![command(
                40,
                4,
                4,
                Payload::SetMode(ProcessingMode::Scheduled),
            )],
        ),
    ])
}
fn headless() -> Vec<TickEvidence> {
    let mut sim = Simulation::restore_domain_checkpoint(DEMO).unwrap();
    let schedule = schedule();
    (0..100)
        .map(|tick| {
            let out = sim
                .drive(tick, schedule.get(&tick).map_or(&[], Vec::as_slice))
                .unwrap();
            TickEvidence {
                tick: sim.tick(),
                checkpoint: sim.domain_checkpoint_bytes(),
                digest: sim.event_digest(),
                next_event: sim.next_event_id(),
                events: out
                    .resource_events()
                    .iter()
                    .map(|e| e.canonical_bytes())
                    .collect(),
            }
        })
        .collect()
}

#[test]
fn c04_t02_exact_s4_fixture_and_five_goals() {
    let (mut sim, initial) = View::validated(DEMO).unwrap();
    assert_eq!(sim.domain_checkpoint_bytes(), DEMO);
    assert_eq!(initial.actors.len(), 20);
    assert_eq!(initial.sites, [1, 2, 9]);
    assert_eq!(initial.edges, [(1, 2, 1, 1), (2, 1, 2, 1)]);
    for a in &initial.actors {
        assert_eq!(
            (a.enabled, a.mode, a.next_wake, a.site, a.task),
            (1, 1, 1, 1, 0)
        );
        for (i, need) in a.needs.iter().enumerate() {
            assert_eq!(
                *need,
                if i == (a.id as usize - 1) % 5 {
                    1000
                } else {
                    0
                }
            );
        }
        assert_eq!(
            a.home,
            if matches!((a.id - 1) % 5 + 1, 2 | 4 | 5) {
                2
            } else {
                1
            }
        );
    }
    for tick in 0..4 {
        sim.drive(tick, &[]).unwrap();
        let v = View::from_sim(&sim).unwrap();
        assert_eq!(v.tasks.len(), 20);
        for t in &v.tasks {
            assert_eq!(u32::from(t.goal), (t.actor - 1) % 5 + 1);
            let short = matches!(t.goal, 1 | 3);
            let stage = match (tick, short) {
                (0, true) => 4,
                (0, false) => 3,
                (1, true) => 5,
                (1, false) => 4,
                (2, true) => 6,
                (2, false) => 5,
                _ => 6,
            };
            assert_eq!(t.stage, stage);
        }
    }
    let v = View::from_sim(&sim).unwrap();
    assert!(v.actors.iter().all(|a| a.task == 0));
    assert_eq!(
        v.stocks
            .iter()
            .filter(|s| s.owner_kind == 3 && s.owner == 2 && s.definition == 7)
            .map(|s| s.quantity)
            .sum::<u64>(),
        4
    );
    for s in &v.stocks {
        if [101, 102].contains(&s.id) {
            assert_eq!(s.quantity, 4);
        }
    }
    for a in &v.actors {
        assert_eq!(
            a.site,
            if matches!((a.id - 1) % 5 + 1, 2 | 4 | 5) {
                2
            } else {
                1
            }
        );
        if (a.id - 1) % 5 == 0 {
            assert_eq!(sim.resource_stock_quantity(u64::from(a.id)), Some(0));
        }
    }
}

#[test]
fn c04_t03_bevy_plugin_100_ticks_matches_headless_every_tick_for_all_frame_partitions() {
    let expected = headless();
    for partition in [
        vec![33_333_333],
        vec![16_666_667],
        vec![6_944_444],
        vec![1, 499_999_999, 2_500_000_000, 0, 123_456_789],
    ] {
        let mut app = app();
        {
            let mut s = app.world_mut().resource_mut::<Session>();
            s.capture = true;
            s.schedule = schedule();
        }
        frame(&mut app, 0, vec![Action::Resume]);
        let mut n = 0;
        while app.world().resource::<Session>().tick() < 100 {
            let s = app.world().resource::<Session>();
            let remaining = (100 - u128::from(s.tick())) * TICK_NS - s.debt_ns;
            frame(
                &mut app,
                partition[n % partition.len()].min(remaining),
                vec![],
            );
            n += 1;
            assert!(n < 20_000);
        }
        assert_eq!(app.world().resource::<Session>().evidence, expected);
    }
    let mut app = app();
    {
        let mut s = app.world_mut().resource_mut::<Session>();
        s.capture = true;
        s.schedule = schedule();
    }
    frame(&mut app, 0, vec![Action::Resume]);
    frame(&mut app, 100 * TICK_NS, vec![]);
    assert_eq!(app.world().resource::<Session>().tick(), 8);
    assert_eq!(app.world().resource::<Session>().debt_ns, 92 * TICK_NS);
    for _ in 0..12 {
        frame(&mut app, 0, vec![]);
    }
    assert_eq!(app.world().resource::<Session>().evidence, expected);
    assert_eq!(app.world().resource::<Session>().debt_ns, 0);
}

#[test]
fn c04_t04_pause_step_ui_and_invalid_drive_do_not_publish() {
    let mut app = app();
    let before = app.world().resource::<Session>().checkpoint();
    for _ in 0..100 {
        frame(
            &mut app,
            10 * TICK_NS,
            vec![Action::Language(Locale::Indonesian), Action::Select(4)],
        );
    }
    assert_eq!(app.world().resource::<Session>().checkpoint(), before);
    frame(&mut app, 100 * TICK_NS, vec![Action::Step]);
    assert_eq!(app.world().resource::<Session>().tick(), 1);
    assert!(app.world().resource::<Session>().paused);
    frame(&mut app, 100 * TICK_NS, vec![Action::Resume, Action::Step]);
    assert_eq!(app.world().resource::<Session>().tick(), 1);
    frame(&mut app, TICK_NS, vec![]);
    assert_eq!(app.world().resource::<Session>().tick(), 2);
    frame(&mut app, 100 * TICK_NS, vec![Action::Pause]);
    assert_eq!(app.world().resource::<Session>().tick(), 2);
    let before = app.world().resource::<Session>().checkpoint();
    app.world_mut()
        .resource_mut::<Session>()
        .schedule
        .insert(2, vec![command(3, 1, 99, Payload::SetEnabled(2))]);
    frame(&mut app, 0, vec![Action::Step]);
    let s = app.world().resource::<Session>();
    assert_eq!(s.checkpoint(), before);
    assert!(s.paused);
    assert_eq!(s.status, "status.error");
}

#[test]
fn c04_t04_ignored_running_step_must_not_discard_frame_time() {
    let mut plain = app();
    let mut with_disabled_step = app();
    frame(&mut plain, 0, vec![Action::Resume]);
    frame(&mut with_disabled_step, 0, vec![Action::Resume]);
    for _ in 0..10 {
        frame(&mut plain, TICK_NS, vec![]);
        frame(&mut with_disabled_step, TICK_NS, vec![Action::Step]);
        assert_eq!(
            with_disabled_step
                .world()
                .resource::<Session>()
                .checkpoint(),
            plain.world().resource::<Session>().checkpoint()
        );
    }
    assert_eq!(with_disabled_step.world().resource::<Session>().tick(), 10);
}

#[test]
fn c04_t05_file_resume_and_language_preserve_every_future_tick() {
    let expected = headless();
    for split in [2, 50] {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        {
            let mut s = app.world_mut().resource_mut::<Session>();
            s.capture = true;
            s.schedule = schedule();
            s.save_path = dir.path().join("save.c03");
        }
        for _ in 0..split {
            frame(&mut app, 0, vec![Action::Step]);
        }
        assert_eq!(
            app.world().resource::<Session>().checkpoint(),
            expected[split - 1].checkpoint
        );
        frame(&mut app, 0, vec![Action::Save]);
        let saved = app.world().resource::<Session>().checkpoint();
        frame(
            &mut app,
            0,
            vec![Action::Step, Action::Language(Locale::Indonesian)],
        );
        frame(&mut app, 0, vec![Action::Load]);
        assert_eq!(app.world().resource::<Session>().checkpoint(), saved);
        assert!(app.world().resource::<Session>().paused);
        assert!(app.world().resource::<Session>().recent.is_empty());
        assert_eq!(app.world().resource::<Session>().debt_ns, 0);
        for (tick, expected_tick) in expected.iter().enumerate().skip(split) {
            frame(
                &mut app,
                0,
                vec![
                    Action::Language(if tick % 2 == 0 {
                        Locale::English
                    } else {
                        Locale::Indonesian
                    }),
                    Action::Step,
                ],
            );
            let s = app.world().resource::<Session>();
            assert_eq!(s.evidence.last().unwrap(), expected_tick);
        }
    }
}

#[test]
fn c04_t05_bad_load_keeps_live_checkpoint_and_failed_save_keeps_prior_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app();
    let path = dir.path().join("save.c03");
    app.world_mut().resource_mut::<Session>().save_path = path.clone();
    frame(&mut app, 0, vec![Action::Step, Action::Save]);
    let before = app.world().resource::<Session>().checkpoint();
    let mut bad = before.clone();
    *bad.last_mut().unwrap() ^= 1;
    std::fs::write(&path, &bad).unwrap();
    frame(&mut app, 0, vec![Action::Load]);
    assert_eq!(app.world().resource::<Session>().checkpoint(), before);
    assert_eq!(app.world().resource::<Session>().status, "status.error");
    assert!(app.world().resource::<Session>().paused);
    std::fs::remove_file(&path).unwrap();
    frame(&mut app, 0, vec![Action::Load]);
    assert_eq!(app.world().resource::<Session>().checkpoint(), before);
    frame(&mut app, 0, vec![Action::Save]);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // The destination is a child of a file, so I/O fails before publication.
    app.world_mut().resource_mut::<Session>().save_path = path.join("blocked.c03");
    frame(&mut app, 0, vec![Action::Save]);
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(app.world().resource::<Session>().checkpoint(), before);
    assert_eq!(app.world().resource::<Session>().status, "status.error");
}
