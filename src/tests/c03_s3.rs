//! S3 production and navigation. Expected fixtures come from the written contract.
use crate::fixture3::{self, PathStep, Stock, World};
use crate::{ActorId, Command, CommandKey, Leg, Payload, Simulation, StepError, encoding};

fn command(tick: u64, sequence: u64, payload: Payload) -> Command {
    Command::new(
        CommandKey {
            target_tick: tick,
            actor_id: ActorId::ONE,
            producer_id: 1,
            sequence,
        },
        payload,
    )
}

fn start(goal: usize, home: u64) -> World {
    let mut w = World::standard(100);
    w.insert_actor(1, 0);
    let a = w.actors.get_mut(&1).unwrap();
    a.next_wake = 1;
    a.home = home;
    a.channels[goal - 1] = 1000;
    w
}

fn stock(definition: u64, class: u8, quantity: u64, kind: u8, owner: u64) -> Stock {
    Stock {
        definition_id: definition,
        class,
        quantity,
        owner_kind: kind,
        owner_id: owner,
    }
}

fn reload(sim: &Simulation) -> Simulation {
    Simulation::restore_domain_checkpoint(&sim.domain_checkpoint_bytes()).unwrap()
}

fn kinds(sim: &mut Simulation, tick: u64, batch: &[Command]) -> Vec<u8> {
    sim.drive(tick, batch)
        .unwrap()
        .resource_events()
        .iter()
        .map(|e| e.kind)
        .collect()
}

#[test]
fn rest_and_return_home_travel_one_tick_then_complete_without_reservations() {
    for goal in [2, 5] {
        let mut sim = fixture3::into_simulation(start(goal, 2));
        assert_eq!(kinds(&mut sim, 0, &[]), [7, 6]);
        let w = sim.world3.as_ref().unwrap();
        assert_eq!(w.tasks[&1].steps, [PathStep::Edge(1)]);
        assert_eq!((w.tasks[&1].stage, w.tasks[&1].due), (3, 4));
        assert_eq!(w.actors[&1].site, 1, "creation does not move");
        let mut resumed = reload(&sim);
        for tick in 1..4 {
            let expected = sim.drive(tick, &[]).unwrap();
            assert_eq!(resumed.drive(tick, &[]).unwrap(), expected);
            assert_eq!(
                resumed.domain_checkpoint_bytes(),
                sim.domain_checkpoint_bytes()
            );
        }
        let w = sim.world3.as_ref().unwrap();
        assert_eq!(w.actors[&1].site, 2);
        assert_eq!(w.tasks[&1].stage, 6);
        assert_eq!(w.tasks[&1].progress, 3);
        assert!(w.reservations.is_empty());
        assert!(w.transactions[&1].legs.is_empty());
    }
}

#[test]
fn work_selects_lowest_distinct_tool_and_station_and_releases_without_debit() {
    let mut w = start(3, 1);
    for (id, definition) in [(1, 5), (2, 4), (3, 4), (4, 5)] {
        w.stocks.insert(id, stock(definition, 2, 1, 3, 1));
    }
    w.next_stock_id = 5;
    let mut sim = fixture3::into_simulation(w);
    assert_eq!(kinds(&mut sim, 0, &[]), [1, 1, 7, 6]);
    let w = sim.world3.as_ref().unwrap();
    assert_eq!(w.reservations[&2].stock_id, 1);
    assert_eq!(w.reservations[&3].stock_id, 2);
    assert_eq!(kinds(&mut sim, 1, &[]), [7]);
    assert_eq!(kinds(&mut sim, 2, &[]), [4, 3, 3, 7]);
    let w = sim.world3.as_ref().unwrap();
    assert!(w.stocks.values().all(|s| s.quantity == 1));
    assert!(w.reservations.values().all(|r| r.state == 2));
    assert!(w.transactions[&1].legs.is_empty());
    assert_eq!(
        reload(&sim).domain_checkpoint_bytes(),
        sim.domain_checkpoint_bytes()
    );
}

#[test]
fn haul_reserves_and_moves_lowest_inventory_stock_to_existing_or_new_home_stock() {
    for existing in [false, true] {
        let mut w = start(4, 2);
        w.stocks.insert(1, stock(7, 1, 1, 1, 1));
        w.stocks.insert(2, stock(8, 1, 1, 1, 1));
        if existing {
            w.stocks.insert(3, stock(7, 1, 2, 3, 2));
        }
        w.next_stock_id = 4;
        let mut sim = fixture3::into_simulation(w);
        assert_eq!(kinds(&mut sim, 0, &[]), [1, 7, 6]);
        assert_eq!(sim.world3.as_ref().unwrap().reservations[&2].expiry_tick, 5);
        for tick in 1..4 {
            kinds(&mut sim, tick, &[]);
        }
        let w = sim.world3.as_ref().unwrap();
        assert_eq!(w.actors[&1].site, 2);
        assert_eq!(w.stocks[&1].quantity, 0);
        assert_eq!(w.stocks[&2].quantity, 1);
        let dest = if existing { 3 } else { 4 };
        assert_eq!(
            w.stocks[&dest],
            stock(7, 1, if existing { 3 } else { 1 }, 3, 2)
        );
        assert_eq!(w.reservations[&2].state, 2);
        assert_eq!(
            reload(&sim).domain_checkpoint_bytes(),
            sim.domain_checkpoint_bytes()
        );
    }
}

#[test]
fn grid_ties_choose_north_north_east_east_and_resume_stored_path() {
    let mut w = World::standard(100);
    w.insert_actor(1, 50);
    let a = w.actors.get_mut(&1).unwrap();
    a.locus = 2;
    w.insert_scripted_task(50, 1);
    let t = w.tasks.get_mut(&50).unwrap();
    t.goal = 2;
    t.stage = 3;
    t.due = 6;
    t.target_locus = 2;
    t.target_x = 2;
    t.target_y = 2;
    let mut sim = fixture3::into_simulation(w);
    assert!(kinds(&mut sim, 0, &[]).is_empty());
    let w = sim.world3.as_ref().unwrap();
    assert_eq!(
        w.tasks[&50].steps,
        [
            PathStep::Grid(1),
            PathStep::Grid(1),
            PathStep::Grid(2),
            PathStep::Grid(2)
        ]
    );
    assert_eq!(w.tasks[&50].path_index, 1);
    let mut resumed = reload(&sim);
    for tick in 1..6 {
        let output = sim.drive(tick, &[]).unwrap();
        assert_eq!(resumed.drive(tick, &[]).unwrap(), output);
        assert_eq!(
            resumed.domain_checkpoint_bytes(),
            sim.domain_checkpoint_bytes()
        );
    }
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&50].stage, 6);
    assert_eq!(sim.world3.as_ref().unwrap().tasks[&50].path_index, 4);
}

fn edge_world(task: bool) -> World {
    let mut w = World::standard(100);
    w.insert_actor(1, if task { 50 } else { 0 });
    let a = w.actors.get_mut(&1).unwrap();
    a.locus = 3;
    a.site = 0;
    a.edge = 1;
    if task {
        w.insert_scripted_task(50, 1);
        let t = w.tasks.get_mut(&50).unwrap();
        t.goal = 5;
        t.stage = 3;
        t.due = 3;
        t.target_site = 2;
    }
    w
}

#[test]
fn edge_start_and_post_cancel_residual_save_preserve_cost_even_when_disabled() {
    let mut normal = fixture3::into_simulation(edge_world(true));
    let mut resumed = reload(&normal);
    assert_eq!(
        normal.drive(0, &[]).unwrap(),
        resumed.drive(0, &[]).unwrap()
    );
    assert_eq!(normal.world3.as_ref().unwrap().actors[&1].site, 2);
    for payload in [Payload::CancelReservation(0), Payload::SetEnabled(0)] {
        let mut sim = fixture3::into_simulation(edge_world(true));
        assert_eq!(kinds(&mut sim, 0, &[command(1, 1, payload)]), [7, 5]);
        let a = &sim.world3.as_ref().unwrap().actors[&1];
        assert_eq!((a.task_id, a.locus, a.edge, a.traversed), (0, 3, 1, 0));
        let mut resumed = reload(&sim);
        for tick in 1..3 {
            assert!(kinds(&mut sim, tick, &[]).is_empty());
            assert!(kinds(&mut resumed, tick, &[]).is_empty());
            assert_eq!(
                resumed.domain_checkpoint_bytes(),
                sim.domain_checkpoint_bytes()
            );
        }
        let a = &sim.world3.as_ref().unwrap().actors[&1];
        assert_eq!((a.locus, a.site, a.edge, a.traversed), (1, 2, 0, 0));
    }
    let mut w = edge_world(false);
    w.actors.get_mut(&1).unwrap().enabled = 0;
    let mut sim = fixture3::into_simulation(w);
    assert!(kinds(&mut sim, 0, &[]).is_empty());
    assert_eq!(sim.world3.as_ref().unwrap().actors[&1].site, 2);
}

#[test]
fn unreachable_home_rejects_before_creation_and_unreachable_task_releases() {
    let mut sim = fixture3::into_simulation(start(5, 9));
    assert_eq!(kinds(&mut sim, 0, &[]), [6]);
    let trace = &sim.world3.as_ref().unwrap().traces[&1];
    for goal in [2usize, 4, 5] {
        assert_eq!(trace[25 + (goal - 1) * 59 + 2], 5);
    }
    assert!(sim.world3.as_ref().unwrap().tasks.is_empty());
    let mut w = fixture3::two_claimants();
    w.tasks.get_mut(&1).unwrap().target_site = 9;
    w.tasks.get_mut(&1).unwrap().stage = 3;
    w.tasks.get_mut(&1).unwrap().due = 3;
    let mut sim = fixture3::into_simulation(w);
    let reserve = command(
        1,
        1,
        Payload::ReserveStock {
            stock_id: 1,
            quantity: 1,
            expiry_tick: 5,
            task_id: 1,
        },
    );
    assert_eq!(kinds(&mut sim, 0, &[reserve]), [1, 3, 7, 5]);
    let w = sim.world3.as_ref().unwrap();
    assert_eq!(w.tasks[&1].interruption, 5);
    assert_eq!(w.stocks[&1].owner_id, 1);
    assert_eq!(w.reservations[&3].state, 2);
}

fn transfer(op: u8, id: u64, definition: u64, kind: u8, owner: u64) -> Leg {
    Leg {
        op,
        stock_id: id,
        definition_id: definition,
        class: 1,
        quantity: 1,
        owner_kind_after: kind,
        owner_id_after: owner,
    }
}

#[test]
fn m0_t07_four_locations_recipe_and_later_leg_rollback_at_tick_ten() {
    let mut w = World::standard(100);
    w.wake_interval = 100;
    w.insert_actor(1, 0);
    w.stocks.insert(10, stock(1, 1, 1, 1, 1));
    w.next_stock_id = 11;
    w.next_record_id = 401;
    let mut sim = fixture3::into_simulation(w);
    let blob = sim.domain_checkpoint_bytes();
    assert_eq!(
        encoding::sha256(&blob)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "bd803a2c4ae4320ce2a1a5ceb49a2941aec48921c4cbc0a60b15c607b62148c3"
    );
    let mut expected_event_id = 1;
    let mut stream = Vec::new();
    for hop in 0..5u64 {
        let (definition, owner_kind, owner, recipe) = match hop {
            0 => (1, 2, 20, 0),
            1 => (1, 4, 30, 0),
            2 => (1, 3, 2, 0),
            3 => (2, 3, 2, 1),
            _ => (2, 3, 0, 0),
        };
        let begin = Payload::BeginTask {
            goal: if hop == 3 { 3 } else { 4 },
            stage: 5,
            recipe_id: recipe,
            target_locus: 1,
            target_site: 1,
            target_x: 0,
            target_y: 0,
            due_tick: hop * 2 + 2,
            leg_count: 2,
            legs: [
                transfer(
                    if hop == 3 { 3 } else { 1 },
                    10 + hop,
                    if hop < 4 { 1 } else { 2 },
                    0,
                    0,
                ),
                transfer(
                    if hop == 3 { 4 } else { 2 },
                    0,
                    definition,
                    owner_kind,
                    owner,
                ),
                transfer(0, 0, 0, 0, 0),
                transfer(0, 0, 0, 0, 0),
            ],
        };
        let create_tick = hop * 2 + 1;
        let result = sim
            .drive(
                create_tick - 1,
                &[
                    command(create_tick, hop * 2 + 1, begin),
                    command(create_tick, hop * 2 + 2, Payload::CompleteTask(0)),
                ],
            )
            .unwrap();
        assert_eq!(result.resource_events().len(), 1);
        let e = &result.resource_events()[0];
        assert_eq!((e.id, e.kind), (expected_event_id, 7));
        stream.extend(e.canonical_bytes());
        expected_event_id += 1;
        let mut resumed = reload(&sim);
        if hop == 4 {
            let before = sim.domain_checkpoint_bytes();
            assert_eq!(sim.drive(9, &[]), Err(StepError::InvalidLeg));
            assert_eq!(resumed.drive(9, &[]), Err(StepError::InvalidLeg));
            assert_eq!(sim.domain_checkpoint_bytes(), before);
            assert_eq!(resumed.domain_checkpoint_bytes(), before);
            assert_eq!(sim.next_event_id(), 14);
            assert_eq!(sim.world3.as_ref().unwrap().stocks[&14].quantity, 1);
        } else {
            let output = sim.drive(create_tick, &[]).unwrap();
            assert_eq!(resumed.drive(create_tick, &[]).unwrap(), output);
            assert_eq!(
                resumed.domain_checkpoint_bytes(),
                sim.domain_checkpoint_bytes()
            );
            assert_eq!(
                output
                    .resource_events()
                    .iter()
                    .map(|e| e.kind)
                    .collect::<Vec<_>>(),
                [4, 7]
            );
            assert_eq!(output.resource_events()[0].id, expected_event_id);
            expected_event_id += 2;
            for event in output.resource_events() {
                stream.extend(event.canonical_bytes());
            }
            let w = sim.world3.as_ref().unwrap();
            assert_eq!(w.stocks[&(10 + hop)].quantity, 0);
            assert_eq!(
                w.stocks[&(11 + hop)],
                stock(definition, 1, 1, owner_kind, owner)
            );
            assert_eq!(w.transactions.len(), hop as usize + 1);
        }
    }
    assert_eq!(
        encoding::sha256(&stream)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "ba129bbc9e420be0ae4afcaef0d18abd6a2557c9a6b1902c0f0c5449b56a1327"
    );
    assert_eq!(
        sim.event_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "3496e28f3771c4ac6bbf1e394ff8246384f1bd0c1f9a56bfa618cc0076f37852"
    );
}
