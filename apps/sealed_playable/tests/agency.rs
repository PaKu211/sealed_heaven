use bevy::prelude::*;
use sealed_playable::{
    agency::{Action, AgencyPlugin, Input, MAX_INPUTS, Session, TICK_NS},
    locale::Locale,
};
use sealed_sim::player5::{
    Command, Direction::*, Intent, Key, Outcome, PLAYER_ID, Position, Target, World,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(AgencyPlugin);
    app.world_mut().resource_mut::<Session>().capture = true;
    app
}

fn frame(app: &mut App, ns: u128, actions: Vec<Action>) {
    let mut input = app.world_mut().resource_mut::<Input>();
    input.elapsed_ns = ns;
    input.actions = actions;
    app.update();
}

fn loop_intents() -> Vec<Intent> {
    use Intent::*;
    vec![
        Move(East),
        Move(East),
        Move(North),
        Talk(1),
        Move(South),
        Move(East),
        Move(East),
        Move(East),
        Move(East),
        Move(East),
        Move(East),
        Move(North),
        Move(North),
        Gather,
        Move(South),
        Move(South),
        Move(West),
        Move(West),
        Move(West),
        Move(West),
        Move(West),
        Deliver,
    ]
}

#[test]
fn c05_paused_input_is_bounded_and_only_step_or_resume_commits() {
    let mut app = app();
    let before = app.world().resource::<Session>().world().checkpoint();
    frame(
        &mut app,
        TICK_NS * 100,
        vec![
            Action::Intent(Intent::Move(East)),
            Action::Select(Target::HerbPatch),
            Action::Language(Locale::Indonesian),
        ],
    );
    let session = app.world().resource::<Session>();
    assert_eq!(session.world().checkpoint(), before);
    assert_eq!(session.queued(), 1);
    frame(&mut app, 0, vec![Action::Step]);
    let session = app.world().resource::<Session>();
    assert_eq!(session.world().tick(), 1);
    assert_eq!(session.world().player_position(), Position { x: 1, y: 4 });
    assert_eq!(session.queued(), 0);
    for _ in 0..MAX_INPUTS + 2 {
        frame(&mut app, 0, vec![Action::Intent(Intent::Move(East))]);
    }
    let session = app.world().resource::<Session>();
    assert_eq!(session.queued(), MAX_INPUTS);
    assert_eq!(session.status, "c05.queue_full");
    assert_eq!(session.world().tick(), 1);
}

#[test]
fn c05_running_disabled_step_and_catchup_preserve_all_elapsed_time() {
    let mut a = app();
    let mut b = app();
    frame(&mut a, 0, vec![Action::Resume]);
    frame(&mut b, 0, vec![Action::Resume]);
    for _ in 0..10 {
        frame(&mut a, TICK_NS, vec![]);
        frame(&mut b, TICK_NS, vec![Action::Step]);
        assert_eq!(
            a.world().resource::<Session>().world().checkpoint(),
            b.world().resource::<Session>().world().checkpoint()
        );
    }
    frame(&mut a, TICK_NS * 20, vec![]);
    assert_eq!(a.world().resource::<Session>().world().tick(), 18);
    frame(&mut a, 0, vec![]);
    frame(&mut a, 0, vec![]);
    assert_eq!(a.world().resource::<Session>().world().tick(), 30);
    assert_eq!(a.world().resource::<Session>().debt_ns, 0);
    frame(&mut a, 0, vec![Action::Pause]);
    let before = a.world().resource::<Session>().world().checkpoint();
    frame(&mut a, TICK_NS * 100, vec![]);
    assert_eq!(a.world().resource::<Session>().world().checkpoint(), before);
}

#[test]
fn c05_full_loop_save_load_every_action_matches_headless_and_frame_partitions() {
    let mut direct = World::settlement();
    let mut commands = Vec::new();
    let mut expected = Vec::new();
    for intent in loop_intents() {
        let command = Command {
            key: Key {
                target_tick: direct.tick() + 1,
                actor_id: PLAYER_ID,
                producer_id: 1,
                sequence: direct.next_sequence(),
            },
            intent,
        };
        let events = direct.drive(direct.tick(), &[command]).unwrap();
        assert!(
            !matches!(events[0].outcome, Outcome::Rejected(_)),
            "{command:?} {:?}",
            events[0]
        );
        commands.push(command);
        expected.push((direct.checkpoint(), events));
    }
    assert!(direct.completed());
    assert_eq!(
        (
            direct.source_herbs(),
            direct.player_herbs(),
            direct.delivered_herbs()
        ),
        (2, 0, 1)
    );
    let dir = tempfile::tempdir().unwrap();
    for ns in [16_666_667, 33_333_333, 6_944_444, TICK_NS * 9] {
        let mut app = app();
        {
            let mut s = app.world_mut().resource_mut::<Session>();
            for c in &commands {
                s.schedule.insert(c.key.target_tick - 1, vec![*c]);
            }
        }
        frame(&mut app, 0, vec![Action::Resume]);
        while app.world().resource::<Session>().world().tick() < commands.len() as u64 {
            // Clamp the final time to avoid introducing unscripted extra ticks.
            let s = app.world().resource::<Session>();
            let remaining =
                (commands.len() as u64 - s.world().tick()) as u128 * TICK_NS - s.debt_ns;
            frame(&mut app, ns.min(remaining), vec![]);
        }
        let s = app.world().resource::<Session>();
        assert_eq!(s.evidence.len(), expected.len());
        for (actual, (checkpoint, events)) in s.evidence.iter().zip(&expected) {
            assert_eq!(&actual.checkpoint, checkpoint);
            assert_eq!(&actual.events, events);
        }
    }
    let mut live = app();
    live.world_mut().resource_mut::<Session>().save_path = dir.path().join("loop.c05");
    for (i, intent) in loop_intents().into_iter().enumerate() {
        frame(&mut live, 0, vec![Action::Intent(intent), Action::Step]);
        assert_eq!(
            live.world().resource::<Session>().world().checkpoint(),
            expected[i].0
        );
        frame(&mut live, 0, vec![Action::Save]);
        // Queued stale intent must not leak into the reloaded timeline.
        frame(
            &mut live,
            0,
            vec![Action::Intent(Intent::Move(West)), Action::Load],
        );
        let s = live.world().resource::<Session>();
        assert_eq!(s.world().checkpoint(), expected[i].0);
        assert!(s.paused);
        assert_eq!(s.queued(), 0);
        assert!(s.schedule.is_empty());
    }
    assert!(live.world().resource::<Session>().world().completed());
}

#[test]
fn c05_invalid_load_keeps_live_state_and_reset_clears_timeline_inputs() {
    let mut app = app();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.c05");
    std::fs::write(&path, sealed_playable::DEMO).unwrap();
    app.world_mut().resource_mut::<Session>().save_path = path;
    frame(
        &mut app,
        0,
        vec![Action::Intent(Intent::Move(East)), Action::Step],
    );
    let before = app.world().resource::<Session>().world().checkpoint();
    frame(&mut app, 0, vec![Action::Load]);
    let s = app.world().resource::<Session>();
    assert_eq!(s.world().checkpoint(), before);
    assert!(!s.diagnostic.is_empty());
    assert!(s.paused);
    frame(
        &mut app,
        0,
        vec![Action::Intent(Intent::Gather), Action::Reset],
    );
    let s = app.world().resource::<Session>();
    assert_eq!(s.world().checkpoint(), World::settlement().checkpoint());
    assert_eq!(s.queued(), 0);
    assert!(s.diagnostic.is_empty());
}
