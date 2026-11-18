//! S4: the twenty-actor fixture, combined production, and replay/rollback.
use crate::fixture3::{self, Stock, World};
use crate::{
    ActorId, Command, CommandKey, Leg, Payload, ProcessingMode, Simulation, StepError, encoding,
};

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

fn twenty() -> World {
    let mut w = World::standard(100);
    for actor in 1..=20 {
        w.insert_actor(actor, u64::from(actor));
        w.insert_scripted_task(u64::from(actor), actor);
    }
    for (id, definition, class) in [(101, 3, 2), (102, 4, 2), (103, 1, 1), (104, 5, 2)] {
        w.stocks.insert(
            id,
            Stock {
                definition_id: definition,
                class,
                quantity: 5,
                owner_kind: 3,
                owner_id: 1,
            },
        );
    }
    w.next_stock_id = 105;
    w
}

fn bundles() -> Vec<Command> {
    (1..=20)
        .map(|actor| {
            command(
                1,
                actor,
                1,
                Payload::ReserveBundle {
                    task_id: u64::from(actor),
                    expiry_tick: 10,
                    pair_count: 4,
                    stocks: [101, 102, 103, 104],
                    quantities: [1; 4],
                },
            )
        })
        .collect()
}

fn sha(bytes: &[u8]) -> String {
    encoding::sha256(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn reload(sim: &Simulation) -> Simulation {
    Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap()
}

fn event_bytes(id: u64, tick: u64, actor: u32, kind: u8, payload: &[u8]) -> Vec<u8> {
    [
        id.to_le_bytes().as_slice(),
        tick.to_le_bytes().as_slice(),
        actor.to_le_bytes().as_slice(),
        &[kind],
        payload,
    ]
    .concat()
}

fn contention_events() -> Vec<Vec<u8>> {
    let mut expected = Vec::new();
    for actor in 1..=5u32 {
        for offset in 0..4u64 {
            let reservation = 21 + 4 * (u64::from(actor) - 1) + offset;
            let payload = [
                reservation.to_le_bytes(),
                (101 + offset).to_le_bytes(),
                1u64.to_le_bytes(),
            ]
            .concat();
            expected.push(event_bytes(
                expected.len() as u64 + 1,
                1,
                actor,
                1,
                &payload,
            ));
        }
        expected.push(event_bytes(
            expected.len() as u64 + 1,
            1,
            actor,
            7,
            &[u64::from(actor).to_le_bytes().as_slice(), &[2]].concat(),
        ));
    }
    for actor in 6..=20u32 {
        expected.push(event_bytes(
            expected.len() as u64 + 1,
            1,
            actor,
            2,
            &[
                101u64.to_le_bytes().as_slice(),
                1u64.to_le_bytes().as_slice(),
                &[4],
            ]
            .concat(),
        ));
    }
    for actor in 1..=5u32 {
        expected.push(event_bytes(
            expected.len() as u64 + 1,
            1,
            actor,
            7,
            &[u64::from(actor).to_le_bytes().as_slice(), &[4]].concat(),
        ));
    }
    expected
}

#[test]
fn m0_t06_twenty_actor_contention_matches_published_checkpoint_and_event_contract() {
    let expected = contention_events();
    assert_eq!(
        sha(&expected.concat()),
        "c04541cec2c8bde50054a3d47cd8e525b0a3dbdcfca8039befdd09bbd264cb54"
    );
    for permutation in 0..4 {
        let mut batch = bundles();
        match permutation {
            1 => batch.reverse(),
            2 => batch.rotate_left(7),
            3 => batch.sort_by_key(|c| (c.key().actor_id.get() % 3, c.key().actor_id.get())),
            _ => {}
        }
        let mut sim = fixture3::into_simulation(twenty());
        let result = sim.drive(0, &batch).unwrap();
        assert_eq!(
            result
                .resource_events()
                .iter()
                .map(|e| e.canonical_bytes())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            sha(&sim.domain_checkpoint_bytes()),
            "528bc334bc1f081ccbf06c3247ec46909abd6f1073594dfb13ebd7ae16d640c9"
        );
        let w = sim.world3.as_ref().unwrap();
        assert_eq!((w.next_record_id, w.next_event_id), (41, 46));
        assert_eq!(w.reservations.len(), 20);
        for actor in 1..=20u64 {
            let t = &w.tasks[&actor];
            assert_eq!(
                (t.stage, t.due, t.progress),
                if actor <= 5 { (4, 3, 1) } else { (1, 0, 0) }
            );
        }
        let mut digest = encoding::sha256(b"SH:C03:EVENT-INIT:v1");
        for bytes in &expected {
            digest = encoding::sha256(&[b"SH:C03:EVENT:v1".as_slice(), &digest, bytes].concat());
        }
        assert_eq!(sim.event_digest(), digest);
        assert_eq!(
            reload(&sim).domain_checkpoint_bytes(),
            sim.domain_checkpoint_bytes()
        );
    }
    let mut w = twenty();
    for a in w.actors.values_mut() {
        a.mode = 2;
    }
    let mut sim = fixture3::into_simulation(w);
    assert_eq!(
        sim.drive(0, &bundles())
            .unwrap()
            .resource_events()
            .iter()
            .map(|e| e.canonical_bytes())
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn twenty_actor_later_overreserve_rolls_back_all_earlier_allocations_then_recovers() {
    let mut batch = bundles();
    batch[19] = command(
        1,
        20,
        1,
        Payload::ReserveBundle {
            task_id: 20,
            expiry_tick: 10,
            pair_count: 4,
            stocks: [101, 102, 103, 104],
            quantities: [1, 1, 1, 6],
        },
    );
    let mut sim = fixture3::into_simulation(twenty());
    let before = sim.domain_checkpoint_bytes();
    assert_eq!(sim.drive(0, &batch), Err(StepError::InvalidState));
    assert_eq!(sim.domain_checkpoint_bytes(), before);
    let mut pristine = reload(&sim);
    assert_eq!(
        sim.drive(0, &bundles()).unwrap(),
        pristine.drive(0, &bundles()).unwrap()
    );
    assert_eq!(
        sim.domain_checkpoint_bytes(),
        pristine.domain_checkpoint_bytes()
    );
}

#[test]
fn twenty_actor_completion_future_command_restore_and_grouping_match_every_boundary() {
    let mut w = twenty();
    for t in w.tasks.values_mut() {
        t.legs.push(Leg {
            op: 1,
            stock_id: 103,
            definition_id: 1,
            class: 1,
            quantity: 1,
            owner_kind_after: 0,
            owner_id_after: 0,
        });
    }
    let mut schedule = [bundles(), Vec::new(), Vec::new(), Vec::new()];
    schedule[0].push(command(
        4,
        20,
        2,
        Payload::SetMode(ProcessingMode::Scheduled),
    ));
    for actor in 1..=5 {
        schedule[1].push(command(
            2,
            actor,
            2,
            Payload::CompleteTask(u64::from(actor)),
        ));
    }
    for actor in 6..=20 {
        schedule[3].push(command(4, actor, 3, Payload::CancelReservation(0)));
    }
    let initial = fixture3::into_simulation(w);
    let mut baseline = initial.clone();
    let mut checkpoints = vec![baseline.domain_checkpoint_bytes()];
    let mut outputs = Vec::new();
    for (tick, batch) in schedule.iter().enumerate() {
        outputs.push(baseline.drive(tick as u64, batch).unwrap());
        checkpoints.push(baseline.domain_checkpoint_bytes());
    }
    for split in 0..=4 {
        let mut resumed = Simulation::restore_domain_checkpoint(&checkpoints[split]).unwrap();
        for tick in split..4 {
            let mut batch = schedule[tick].clone();
            batch.reverse();
            assert_eq!(resumed.drive(tick as u64, &batch).unwrap(), outputs[tick]);
            assert_eq!(resumed.domain_checkpoint_bytes(), checkpoints[tick + 1]);
        }
    }
    // Grouping is host scheduling, not a second kernel stepping API.
    for groups in [vec![1, 1, 1, 1], vec![2, 2], vec![4]] {
        let mut grouped = initial.clone();
        let mut tick = 0;
        for count in groups {
            for _ in 0..count {
                assert_eq!(
                    grouped.drive(tick as u64, &schedule[tick]).unwrap(),
                    outputs[tick]
                );
                assert_eq!(grouped.domain_checkpoint_bytes(), checkpoints[tick + 1]);
                tick += 1;
            }
        }
    }
    let w = baseline.world3.as_ref().unwrap();
    assert_eq!(w.stocks[&103].quantity, 0);
    assert_eq!(w.transactions.len(), 5);
    assert_eq!(w.registry.len(), 5);
    assert!(w.reservations.values().all(|r| r.state == 2));
    assert_eq!(w.actors[&20].mode, 2);
    for actor in 1..=5 {
        assert!(
            baseline
                .drive(
                    4 + u64::from(actor - 1),
                    &[command(
                        2,
                        actor,
                        2,
                        Payload::CompleteTask(u64::from(actor))
                    )]
                )
                .unwrap()
                .resource_events()
                .is_empty()
        );
    }
    assert_eq!(baseline.world3.as_ref().unwrap().transactions.len(), 5);
}

#[test]
fn twenty_living_actors_run_all_five_goals_in_both_modes_and_resume_every_tick() {
    let mut w = World::standard(100);
    for actor in 1..=20u32 {
        w.insert_actor(actor, 0);
        let goal = (actor as usize - 1) % 5;
        let a = w.actors.get_mut(&actor).unwrap();
        a.next_wake = 1;
        a.channels[goal] = 1000;
        if matches!(goal, 1 | 3 | 4) {
            a.home = 2;
        }
        if goal == 0 || goal == 3 {
            w.stocks.insert(
                u64::from(actor),
                Stock {
                    definition_id: if goal == 0 { 1 } else { 7 },
                    class: 1,
                    quantity: 1,
                    owner_kind: 1,
                    owner_id: u64::from(actor),
                },
            );
        }
    }
    for (id, definition) in [(101, 4), (102, 5)] {
        w.stocks.insert(
            id,
            Stock {
                definition_id: definition,
                class: 2,
                quantity: 4,
                owner_kind: 3,
                owner_id: 1,
            },
        );
    }
    w.next_stock_id = 103;
    let mut active = fixture3::into_simulation(w.clone());
    let mut mixed = fixture3::into_simulation(w);
    for tick in 0..4 {
        let batch: Vec<_> = if tick == 0 {
            (1..=20)
                .filter(|a| a % 2 == 0)
                .map(|a| command(1, a, 1, Payload::SetMode(ProcessingMode::Scheduled)))
                .collect()
        } else {
            vec![]
        };
        let mut resumed = reload(&mixed);
        let output = mixed.drive(tick, &batch).unwrap();
        assert_eq!(active.drive(tick, &[]).unwrap(), output);
        assert_eq!(resumed.drive(tick, &batch).unwrap(), output);
        assert_eq!(
            resumed.domain_checkpoint_bytes(),
            mixed.domain_checkpoint_bytes()
        );
    }
    let w = mixed.world3.as_ref().unwrap();
    assert_eq!(w.tasks.len(), 20);
    assert_eq!(w.transactions.len(), 20);
    assert!(w.tasks.values().all(|t| t.stage == 6));
    assert!(w.actors.values().all(|a| a.task_id == 0));
    assert!(w.reservations.values().all(|r| r.state == 2));
    for actor in 1..=20u32 {
        let goal = (actor - 1) % 5 + 1;
        let task = w.tasks.values().find(|t| t.actor_id == actor).unwrap();
        assert_eq!(task.goal, goal as u8);
        assert_eq!(
            w.actors[&actor].site,
            if matches!(goal, 2 | 4 | 5) { 2 } else { 1 }
        );
    }
    assert_eq!(w.stocks[&101].quantity, 4);
    assert_eq!(w.stocks[&102].quantity, 4);
    assert_eq!(
        w.stocks
            .values()
            .filter(|s| s.owner_kind == 3 && s.owner_id == 2 && s.definition_id == 7)
            .map(|s| s.quantity)
            .sum::<u64>(),
        4
    );
}

#[test]
fn m0_t08_empty_rest_return_completion_and_disablement_precedence() {
    for goal in [2, 5] {
        for disabled in [false, true] {
            let mut w = World::standard(100);
            w.insert_actor(1, 1);
            w.insert_scripted_task(1, 1);
            let t = w.tasks.get_mut(&1).unwrap();
            t.stage = 5;
            t.goal = goal;
            t.due = 1;
            let mut sim = fixture3::into_simulation(w);
            let complete = command(1, 1, 1, Payload::CompleteTask(1));
            let mut batch = vec![complete];
            if disabled {
                batch.push(command(1, 1, 2, Payload::SetEnabled(0)));
            }
            let output = sim.drive(0, &batch).unwrap();
            assert_eq!(
                output
                    .resource_events()
                    .iter()
                    .map(|e| e.kind)
                    .collect::<Vec<_>>(),
                if disabled { vec![7, 5] } else { vec![4, 7] }
            );
            let w = sim.world3.as_ref().unwrap();
            assert_eq!(
                w.registry[&complete.key()].role,
                if disabled { 2 } else { 1 }
            );
            assert_eq!(w.transactions.len(), if disabled { 0 } else { 1 });
            let mut sim = reload(&sim);
            assert!(
                sim.drive(1, &[complete])
                    .unwrap()
                    .resource_events()
                    .is_empty()
            );
        }
    }
    let mut sim = fixture3::into_simulation(twenty());
    sim.drive(0, &bundles()).unwrap();
    sim.drive(1, &[]).unwrap();
    let w = sim.world3.as_mut().unwrap();
    for r in w.reservations.values_mut() {
        r.expiry_tick = 3;
    }
    let mut batch = Vec::new();
    for actor in 1..=5 {
        batch.push(command(
            3,
            actor,
            2,
            Payload::CompleteTask(u64::from(actor)),
        ));
        batch.push(command(3, actor, 3, Payload::CancelReservation(0)));
        batch.push(command(3, actor, 4, Payload::SetEnabled(0)));
    }
    let output = sim.drive(2, &batch).unwrap();
    assert_eq!(output.resource_events().len(), 30);
    for e in output.resource_events() {
        match e.kind {
            3 | 5 => assert_eq!(e.payload.last(), Some(&1)),
            7 => assert_eq!(e.payload.last(), Some(&8)),
            _ => panic!("unexpected completion"),
        }
    }
    assert!(sim.world3.as_ref().unwrap().transactions.is_empty());
    assert_eq!(
        reload(&sim).domain_checkpoint_bytes(),
        sim.domain_checkpoint_bytes()
    );
}
