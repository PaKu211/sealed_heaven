use super::navigation::{in_reach, moved, next_step};
use super::*;

const MEDIC: ActorId = ActorId::new(1);
const WATCH: ActorId = ActorId::new(2);
const SUPPLIER: ActorId = ActorId::new(3);
const LISTENER: ActorId = ActorId::new(4);
const TRAVELER: ActorId = ActorId::new(22);

const fn cell(x: u32, y: u32) -> Position {
    Position { x, y }
}

const fn actor(id: ActorId, role: Role, glyph: char, spawn: Position) -> NpcDefinition {
    NpcDefinition {
        id,
        role,
        name: "Test actor",
        glyph,
        spawn,
        home: spawn,
        route: &[],
        hp: 6,
        rest_threshold: 40,
        responds_to_attack: false,
        shares_news: true,
        report_to: None,
    }
}

const SCENE: Scenario = Scenario {
    name: "Test crossing",
    width: 10,
    height: 5,
    terrain: &[
        "..........",
        ".....#....",
        "..........",
        ".....#....",
        "..........",
    ],
    entrance: cell(0, 2),
    clinic: cell(1, 1),
    patch: cell(1, 4),
    shrine: cell(2, 4),
    shrine_target: 5,
    initial_source: 12,
    regrow_every: 12,
    npcs: &[
        actor(MEDIC, Role::Medic, 'M', cell(1, 1)),
        NpcDefinition {
            report_to: Some(MEDIC),
            ..actor(WATCH, Role::Watch, 'W', cell(4, 2))
        },
        actor(SUPPLIER, Role::Supplier, 'S', cell(1, 4)),
        actor(LISTENER, Role::Traveler, 'L', cell(1, 0)),
        NpcDefinition {
            hp: 4,
            responds_to_attack: true,
            shares_news: false,
            ..actor(TRAVELER, Role::Traveler, 'T', cell(7, 2))
        },
    ],
    recruits: &[],
    contacts: &[(WATCH, MEDIC), (MEDIC, LISTENER)],
};

fn turn(world: &mut World, intent: Intent) -> Outcome {
    world.step(world.tick(), intent).unwrap()
}

fn wait(world: &mut World, ticks: usize) {
    for _ in 0..ticks {
        assert_eq!(turn(world, Intent::Wait), Outcome::Waited);
    }
}

/// Enter every cell through the normal player-intent pipeline.
fn walk(world: &mut World, destination: Position, reach: u32) {
    for _ in 0..128 {
        let position = world.player().position;
        if navigation::distance(position, destination) <= reach {
            return;
        }
        let Some(next) = next_step(world.scenario(), position, destination, reach, |cell| {
            world.walkable(cell) && !world.occupied(cell)
        }) else {
            wait(world, 1);
            continue;
        };
        let direction = DIRECTIONS
            .into_iter()
            .find(|&direction| moved(position, direction) == Some(next))
            .unwrap();
        assert_eq!(turn(world, Intent::Move(direction)), Outcome::Moved);
    }
    panic!("did not reach {destination:?}");
}

fn fight_from(position: Position) -> World {
    let mut world = World::new(&SCENE);
    walk(&mut world, position, 0);
    assert_eq!(
        turn(&mut world, Intent::Attack(TRAVELER)),
        Outcome::Struck(TRAVELER)
    );
    assert_eq!(world.player().hp, 4);
    assert_eq!(world.npc(TRAVELER).unwrap().activity.goal, Goal::Retaliate);
    assert_eq!(
        turn(&mut world, Intent::Attack(TRAVELER)),
        Outcome::Struck(TRAVELER)
    );
    assert!(!world.npc(TRAVELER).unwrap().alive());
    assert_eq!(
        world
            .social()
            .incidents()
            .iter()
            .filter(|incident| incident.fatal)
            .count(),
        1
    );
    world
}

#[test]
fn qinghe_waits_advance_real_positions_activity_and_shrine_stock() {
    let mut world = World::qinghe();
    let supplier = *world.npc(SUPPLIER).unwrap();
    let guard = *world.npc(WATCH).unwrap();
    wait(&mut world, 1);
    assert_eq!(world.npc(MEDIC).unwrap().activity.goal, Goal::TendClinic);
    assert_eq!(world.npc(WATCH).unwrap().activity.goal, Goal::Patrol);
    assert_eq!(
        world.npc(SUPPLIER).unwrap().activity.goal,
        Goal::CollectHerbs
    );
    assert_ne!(world.npc(SUPPLIER).unwrap().position, supplier.position);
    wait(&mut world, 1);
    assert_ne!(world.npc(WATCH).unwrap().position, guard.position);
    wait(&mut world, 220);
    assert_eq!(world.supply().shrine, world.scenario().shrine_target);
    assert_eq!(
        world.supply().npc_deliveries,
        world.scenario().shrine_target
    );
    let supply = world.supply();
    let carried = world.npcs().map(|npc| npc.herbs).sum::<u32>() + world.player().herbs;
    assert_eq!(
        supply.source + supply.shrine + supply.consumed + carried,
        world.scenario.initial_source + supply.regrown
    );
}

#[test]
fn qinghe_gate_timing_changes_whether_the_outrider_fight_reaches_the_clinic() {
    let mut arrival = World::qinghe();
    for _ in 0..8 {
        assert_eq!(
            turn(&mut arrival, Intent::Move(Direction::East)),
            Outcome::Moved
        );
    }
    assert_eq!(arrival.player().position, cell(8, 4));
    let mut seen = arrival.clone();
    let mut unseen = arrival;
    wait(&mut unseen, 3); // The courier has passed behind the wall.
    for world in [&mut seen, &mut unseen] {
        for _ in 0..2 {
            assert_eq!(
                turn(world, Intent::Attack(TRAVELER)),
                Outcome::Struck(TRAVELER)
            );
        }
        assert_eq!(world.player().hp, 4);
        wait(world, 22);
        assert_eq!(World::restore(&world.checkpoint()).unwrap(), *world);
    }
    let first = IncidentId(0);
    assert_eq!(
        seen.social().knowledge(SUPPLIER, first).unwrap().source,
        KnowledgeSource::Observed
    );
    assert!(seen.social().relationship(MEDIC, PLAYER).refuses_aid());
    assert!(
        seen.social()
            .incidents()
            .iter()
            .filter(|event| event.source == PLAYER)
            .any(|event| matches!(
                seen.social().knowledge(MEDIC, event.id),
                Some(Knowledge {
                    source: KnowledgeSource::Reported { .. },
                    ..
                })
            ))
    );
    assert_eq!(unseen.social().knowledge(SUPPLIER, first), None);
    assert_eq!(unseen.social().knowledge(MEDIC, first), None);
    assert!(!unseen.social().relationship(MEDIC, PLAYER).refuses_aid());
}

#[test]
fn an_assault_interrupts_a_tired_traveler_and_guard_reduces_retaliation() {
    let mut world = World::new(&SCENE);
    walk(&mut world, cell(6, 2), 0);
    let since = world.tick();
    let traveler = world.npcs.get_mut(&TRAVELER).unwrap();
    traveler.fatigue = 40;
    traveler.activity = Activity {
        goal: Goal::Rest,
        since,
    };
    assert_eq!(world.choose_goal(world.npc(TRAVELER).unwrap()), Goal::Rest);
    turn(&mut world, Intent::Attack(TRAVELER));
    assert_eq!(
        world.npc(TRAVELER).unwrap().activity,
        Activity {
            goal: Goal::Retaliate,
            since: since + 1
        }
    );
    assert_eq!(world.player().hp, 4);
    assert_eq!(turn(&mut world, Intent::Guard), Outcome::Guarded);
    assert_eq!(world.player().hp, 3);
    assert_eq!(world.social().incidents().last().unwrap().damage, 1);
}

#[test]
fn the_same_fight_changes_access_to_care_only_when_the_assailant_is_seen() {
    let mut seen = fight_from(cell(6, 2));
    let mut unseen = fight_from(cell(7, 3));
    let blow = IncidentId(0);
    let first = *seen.social().incident(blow).unwrap();
    assert_eq!((first.source, first.target), (PLAYER, TRAVELER));
    assert_eq!(
        seen.social().knowledge(WATCH, blow).unwrap().source,
        KnowledgeSource::Observed
    );
    assert_eq!(
        unseen.social().knowledge(TRAVELER, blow).unwrap().source,
        KnowledgeSource::Observed
    );
    assert_eq!(unseen.social().knowledge(WATCH, blow), None);
    assert_eq!(seen.social().knowledge(MEDIC, blow), None);

    for world in [&mut seen, &mut unseen] {
        wait(world, 10);
        walk(world, SCENE.clinic, 1);
    }
    assert_eq!(
        seen.social().knowledge(MEDIC, blow).unwrap().source,
        KnowledgeSource::Reported {
            sender: WATCH,
            witness: WATCH,
            hops: 1
        }
    );
    assert_eq!(
        seen.social().knowledge(LISTENER, blow).unwrap().source,
        KnowledgeSource::Reported {
            sender: MEDIC,
            witness: WATCH,
            hops: 2
        }
    );
    assert_eq!(seen.social().knowledge(SUPPLIER, blow), None);
    assert_eq!(unseen.social().knowledge(MEDIC, blow), None);
    assert_eq!(seen.social().relationship(MEDIC, PLAYER).regard, -2);
    assert_eq!(unseen.social().relationship(MEDIC, PLAYER).regard, 0);
    let seen_stock = seen.supply();
    let unseen_stock = unseen.supply();
    assert_eq!(
        turn(&mut seen, Intent::Talk(MEDIC)),
        Outcome::Rejected(Rejection::Refused)
    );
    assert_eq!(
        turn(&mut unseen, Intent::Talk(MEDIC)),
        Outcome::Treated {
            by: MEDIC,
            healed: 2
        }
    );
    assert_eq!((seen.player().hp, unseen.player().hp), (4, 6));
    assert_eq!(seen.supply().consumed, seen_stock.consumed);
    assert_eq!(unseen.supply().consumed, unseen_stock.consumed + 1);
    assert_eq!(unseen.social().relationship(PLAYER, MEDIC).regard, 1);
    wait(&mut unseen, 14);
    assert!(unseen.supply().npc_deliveries > unseen_stock.npc_deliveries);
}

#[test]
fn pending_report_checkpoint_replays_without_rebroadcast_or_duplicate_consequences() {
    let mut world = fight_from(cell(6, 2));
    let blow = IncidentId(0);
    let report = loop {
        if let Some(report) = world
            .social()
            .pending_reports()
            .find(|report| report.recipient == MEDIC)
            .copied()
        {
            break report;
        }
        assert!(world.tick() < 32, "the witness must find the medic");
        wait(&mut world, 1);
    };
    assert_eq!(report.incident, blow);
    assert_eq!(report.due, report.sent + 2);
    assert_eq!(world.social().knowledge(MEDIC, blow), None);
    let saved = world.checkpoint();
    assert_eq!(World::restore(&saved), Err(LoadError::ScenarioMismatch));
    let mut loaded = checkpoint::decode(&saved, &SCENE).unwrap();
    assert_eq!(loaded, world);
    assert_eq!(loaded.checkpoint(), saved);

    let boundary = loaded.tick();
    assert_eq!(turn(&mut loaded, Intent::Wait), Outcome::Waited);
    assert_eq!(
        loaded.step(boundary, Intent::Attack(TRAVELER)),
        Err(StepError::WrongBoundary {
            expected: boundary + 1,
            received: boundary
        })
    );
    wait(&mut world, 55);
    wait(&mut loaded, 54);
    assert_eq!(loaded.state_digest(), world.state_digest());
    assert_eq!(loaded.checkpoint(), world.checkpoint());
    assert_eq!(world.social().pending_reports().len(), 0);
    assert_eq!(world.social().incidents().len(), 3);
    assert_eq!(world.social().relationship(WATCH, PLAYER).regard, -3);
    assert_eq!(world.social().relationship(MEDIC, PLAYER).regard, -2);
    assert_eq!(world.social().relationship(LISTENER, PLAYER).regard, -1);
    assert_eq!(
        world.social().knowledge(MEDIC, blow).unwrap().acquired,
        report.due
    );
}

#[test]
fn death_leaves_the_world_running_and_retries_cannot_repeat_the_fatal_event() {
    let mut world = World::new(&SCENE);
    walk(&mut world, cell(6, 2), 0);
    turn(&mut world, Intent::Attack(TRAVELER));
    wait(&mut world, 2);
    assert!(!world.player().alive());
    let death_tick = world.tick();
    let last = *world.social().incidents().last().unwrap();
    assert!(last.fatal);
    assert_eq!(last.target, PLAYER);
    assert_eq!(
        turn(&mut world, Intent::Attack(TRAVELER)),
        Outcome::Rejected(Rejection::Dead)
    );
    let regrown = world.supply().regrown;
    wait(&mut world, 24);
    assert_eq!(world.tick(), death_tick + 25);
    assert!(world.supply().regrown > regrown);
    assert_eq!(world.social().incidents().last().unwrap(), &last);
    assert!(world.social().knowledge(MEDIC, IncidentId(0)).is_some());
    assert_eq!(
        checkpoint::decode(&world.checkpoint(), &SCENE).unwrap(),
        world
    );
}

#[test]
fn qinghe_replays_at_arbitrary_checkpoints_and_rejects_old_or_forged_files() {
    let input = [
        Intent::Move(Direction::East),
        Intent::Wait,
        Intent::Talk(MEDIC),
        Intent::Guard,
        Intent::Deliver,
        Intent::Move(Direction::North),
    ];
    let mut world = World::qinghe();
    for intent in input.into_iter().cycle().take(30) {
        turn(&mut world, intent);
    }
    let original = world.checkpoint();
    let mut restored = World::restore(&original).unwrap();
    for intent in input.into_iter().cycle().take(110) {
        assert_eq!(turn(&mut world, intent), turn(&mut restored, intent));
        assert_eq!(world.state_digest(), restored.state_digest());
    }
    assert_eq!(world.checkpoint(), restored.checkpoint());
    assert_eq!(
        World::restore(&crate::player5::World::conflict_with_memory().checkpoint()),
        Err(LoadError::LegacyCheckpoint)
    );

    let mut bad = original;
    bad[46] += 1;
    assert_eq!(World::restore(&bad), Err(LoadError::InvalidEnvelope));
    bad[46] -= 1;
    let checksum = bad.len() - 32;
    bad[checksum - 1] ^= 1;
    assert_eq!(World::restore(&bad), Err(LoadError::ChecksumMismatch));
    let resigned = crate::encoding::sha256(&bad[..checksum]);
    bad[checksum..].copy_from_slice(&resigned);
    assert_eq!(World::restore(&bad), Err(LoadError::StateMismatch));
}

#[test]
fn a_resigned_command_with_the_same_visible_final_state_changes_the_commitment() {
    let mut world = World::qinghe();
    wait(&mut world, 2);
    let mut bytes = world.checkpoint();
    bytes[50] = 6; // Replace the first wait with guard while no combat is present.
    let checksum = bytes.len() - 32;
    let resigned = crate::encoding::sha256(&bytes[..checksum]);
    bytes[checksum..].copy_from_slice(&resigned);
    assert_eq!(World::restore(&bytes), Err(LoadError::StateMismatch));
}

#[test]
fn sight_is_symmetric_and_identification_requires_more_than_seeing_the_target() {
    let world = World::new(&SCENE);
    let guard = cell(4, 2);
    let traveler = cell(7, 2);
    let visible = cell(6, 2);
    let hidden = cell(7, 3);
    for (position, expected) in [(visible, true), (traveler, true), (hidden, false)] {
        assert_eq!(world.can_see(guard, position), expected);
        assert_eq!(world.can_see(position, guard), expected);
    }
    assert!(in_reach(visible, traveler) && in_reach(hidden, traveler));
}

#[test]
fn wounded_npc_abandons_care_after_an_out_of_sight_medics_death() {
    let mut world = World::new(&SCENE);
    world.supply.shrine = 2;
    world.npcs.get_mut(&TRAVELER).unwrap().hp = 2;
    assert!(!world.perceives(world.npcs[&TRAVELER].position, world.npcs[&MEDIC].position));
    assert_eq!(world.choose_goal(&world.npcs[&TRAVELER]), Goal::SeekCare);

    world.npcs.get_mut(&MEDIC).unwrap().hp = 0;
    assert_eq!(world.choose_goal(&world.npcs[&TRAVELER]), Goal::Idle);
    let stock = world.supply.shrine;
    wait(&mut world, 1);
    assert_eq!(world.npcs[&TRAVELER].hp, 2);
    assert_eq!(world.supply.shrine, stock);
}

#[test]
fn an_en_route_report_does_not_reach_a_dead_recipient() {
    let mut world = fight_from(cell(6, 2));
    let report = loop {
        if let Some(report) = world
            .social()
            .pending_reports()
            .find(|report| report.recipient == MEDIC)
            .copied()
        {
            break report;
        }
        assert!(world.tick() < 32, "the watch should dispatch its report");
        wait(&mut world, 1);
    };
    world.npcs.get_mut(&MEDIC).unwrap().hp = 0;
    while world.tick() < report.due {
        wait(&mut world, 1);
    }
    assert_eq!(world.social().knowledge(MEDIC, report.incident), None);
    assert!(
        !world
            .social()
            .pending_reports()
            .any(|item| item.recipient == MEDIC)
    );
    assert!(
        !matches!(world.choose_goal(&world.npcs[&WATCH]), Goal::Report { recipient, .. } if recipient == MEDIC)
    );

    // The alternate is eligible only after the recipient trusts the witness.
    world.social.meet(WATCH, LISTENER);
    assert!(
        matches!(world.choose_goal(&world.npcs[&WATCH]), Goal::Report { recipient, .. } if recipient == LISTENER)
    );
}

#[test]
fn a_closed_passage_disables_care_until_a_route_opens() {
    // The shared watch stands on the only cell west of this gate. Park that
    // witness off the corridor so the player is the passage, not a second cork.
    const SPLIT_NPCS: &[NpcDefinition] = &[
        actor(MEDIC, Role::Medic, 'M', cell(1, 1)),
        NpcDefinition {
            report_to: Some(MEDIC),
            ..actor(WATCH, Role::Watch, 'W', cell(4, 0))
        },
        actor(SUPPLIER, Role::Supplier, 'S', cell(1, 4)),
        actor(LISTENER, Role::Traveler, 'L', cell(1, 0)),
        NpcDefinition {
            hp: 4,
            responds_to_attack: true,
            shares_news: false,
            ..actor(TRAVELER, Role::Traveler, 'T', cell(7, 2))
        },
    ];
    static SPLIT: Scenario = Scenario {
        terrain: &[
            ".....#....",
            ".....#....",
            "..........",
            ".....#....",
            ".....#....",
        ],
        npcs: SPLIT_NPCS,
        ..SCENE
    };
    let mut world = World::new(&SPLIT);
    world.supply.shrine = 2;
    world.npcs.get_mut(&TRAVELER).unwrap().hp = 2;
    world.player.position = cell(5, 2);
    assert_eq!(world.choose_goal(&world.npcs[&TRAVELER]), Goal::Idle);
    world.player.position = cell(0, 2);
    assert_eq!(world.choose_goal(&world.npcs[&TRAVELER]), Goal::SeekCare);
}

#[test]
fn a_herb_can_buy_practice_or_help_a_shortage_and_the_choice_survives_reload() {
    let mut world = World::new(&SCENE);
    walk(&mut world, cell(1, 3), 0);
    assert_eq!(turn(&mut world, Intent::Gather), Outcome::Gathered);
    walk(&mut world, cell(2, 3), 0);
    let mut offered = world.clone();
    let start_regard = offered.social().relationship(MEDIC, PLAYER).regard;
    assert!(
        matches!(turn(&mut offered, Intent::Deliver), Outcome::Contribution { witnesses } if witnesses > 0)
    );
    assert_eq!(
        offered.social().relationship(MEDIC, PLAYER).regard,
        start_regard + 1
    );
    assert_eq!(
        turn(&mut world, Intent::Practice),
        Outcome::Practiced { progress: 1 }
    );
    assert_eq!(world.supply().shrine + 1, offered.supply().shrine);
    assert_eq!(world.player().herbs, 0);
    assert_eq!(offered.player().herbs, 0);
    assert_eq!(world.breath_progress(), 1);
    assert_eq!(offered.breath_progress(), 0);
    for campaign in [&world, &offered] {
        assert_eq!(
            checkpoint::decode(&campaign.checkpoint(), &SCENE).unwrap(),
            *campaign
        );
    }
}

#[test]
fn original_checkpoints_resume_on_original_rules_and_reject_new_intents_without_a_tick() {
    let mut world = World::with_rules(&qinghe::QINGHE, Rules::Original);
    // Independently encoded against the original C1.0 field layout.
    let initial = concat!(
        "53483a4331303a574f524c440100c76e7b263e5289085eeb5b61286b4fe409b6ad994828b51650d84e635ecc91da",
        "000000000f9d550ff05f135c64adbfece7e15b99297a4308d3a34b7d5ca57d59644872fe",
        "73f16543cf942053230237e0b3af803f2fed02e632846352b7ee86dcc2586f52",
    );
    let original_bytes: Vec<_> = initial
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(world.checkpoint(), original_bytes);
    assert_eq!(World::restore(&original_bytes).unwrap(), world);

    wait(&mut world, 64);
    let bytes = world.checkpoint();
    assert_eq!(&bytes[12..14], &1u16.to_le_bytes());
    let mut loaded = World::restore(&bytes).unwrap();
    assert_eq!(loaded.checkpoint(), bytes);
    assert_eq!(
        loaded.step(loaded.tick(), Intent::Practice),
        Err(StepError::UnavailableInOriginalRules)
    );
    assert_eq!(loaded.checkpoint(), bytes);
    wait(&mut loaded, 1);
    wait(&mut world, 1);
    assert_eq!(loaded.checkpoint(), world.checkpoint());
    assert_eq!(&World::qinghe().checkpoint()[12..14], &2u16.to_le_bytes());
}

#[test]
fn at_night_the_clinic_and_hauler_go_home_and_resume_after_dawn() {
    let mut world = World::new(&SCENE);
    assert!(!world.is_night());
    wait(&mut world, 1_460);
    assert!(world.is_night());
    assert_eq!(world.npcs[&MEDIC].activity.goal, Goal::Rest);
    assert_eq!(world.npcs[&SUPPLIER].activity.goal, Goal::Rest);
    assert_eq!(
        world.npcs[&SUPPLIER].position,
        world.npcs[&SUPPLIER].definition.home
    );
    assert_eq!(
        checkpoint::decode(&world.checkpoint(), &SCENE).unwrap(),
        world
    );
    wait(&mut world, 1_421);
    assert!(!world.is_night());
    assert_eq!(world.npcs[&MEDIC].activity.goal, Goal::TendClinic);
    assert_ne!(world.npcs[&SUPPLIER].activity.goal, Goal::Rest);
}

#[test]
fn breath_mastery_consumes_three_herbs_and_blocks_a_retaliatory_strike() {
    let mut world = World::new(&SCENE);
    walk(&mut world, cell(1, 3), 0);
    for _ in 0..BREATH_MASTERY {
        assert_eq!(turn(&mut world, Intent::Gather), Outcome::Gathered);
    }
    walk(&mut world, cell(2, 3), 0);
    for progress in 1..=BREATH_MASTERY {
        assert_eq!(
            turn(&mut world, Intent::Practice),
            Outcome::Practiced { progress }
        );
    }
    assert_eq!(world.player().herbs, 0);
    assert_eq!(
        turn(&mut world, Intent::Practice),
        Outcome::Rejected(Rejection::PracticeComplete)
    );
    let supply = world.supply();
    let carried = world.npcs().map(|npc| npc.herbs).sum::<u32>();
    assert_eq!(
        supply.source
            + supply.shrine
            + supply.consumed
            + carried
            + u32::from(world.breath_progress()),
        SCENE.initial_source + supply.regrown
    );
    walk(&mut world, cell(6, 2), 0);
    assert_eq!(
        turn(&mut world, Intent::Attack(TRAVELER)),
        Outcome::Struck(TRAVELER)
    );
    assert_eq!(world.player().hp, 4);
    assert_eq!(turn(&mut world, Intent::Guard), Outcome::Guarded);
    assert_eq!(world.player().hp, 4);
    assert_eq!(
        checkpoint::decode(&world.checkpoint(), &SCENE).unwrap(),
        world
    );
}

fn total_regional_herbs(world: &World) -> u32 {
    let region = world.region().unwrap();
    let supply = world.supply();
    let cargo = world
        .npcs()
        .map(|npc| npc.herbs + u32::from(npc.packed_herbs))
        .sum::<u32>();
    let school_herbs = world.school().map_or(0, |school| {
        (school.duty == Duty::Carrying) as u32 + 2 * school.form_learned as u32
    });
    school_herbs
        + world.company().map_or(0, |company| company.herbs_used)
        + region.market_herbs
        + region.garden_herbs
        + region.hall_offerings
        + 2 * (region.remedies + region.remedies_used)
        + supply.source
        + supply.shrine
        + supply.consumed
        + u32::from(world.breath_progress())
        + world.player().herbs
        + cargo
}

fn at_hall_service(world: &mut World, intent: Intent) -> Outcome {
    for _ in 0..100 {
        let outcome = turn(world, intent);
        if outcome != Outcome::Rejected(Rejection::HallClosed) {
            return outcome;
        }
    }
    panic!("the hall did not reopen");
}

#[test]
fn hall_duty_carries_real_stock_and_unlocks_a_versioned_technique() {
    let mut world = World::reed_hall(0);
    let initial = total_regional_herbs(&world);
    assert_eq!(
        turn(&mut world, Intent::ChooseBackground(Background::Herbalist)),
        Outcome::BackgroundChosen(Background::Herbalist)
    );
    let patch = world.scenario().patch;
    let shrine = world.scenario().shrine;
    let hall = Region::sites().hall;
    walk(&mut world, patch, 1);
    for _ in 0..4 {
        assert_eq!(turn(&mut world, Intent::Gather), Outcome::GatheredPair);
    }
    walk(&mut world, shrine, 1);
    for progress in 1..=BREATH_MASTERY {
        assert_eq!(
            turn(&mut world, Intent::Practice),
            Outcome::Practiced { progress }
        );
    }
    walk(&mut world, hall, 1);
    assert_eq!(
        at_hall_service(&mut world, Intent::JoinHall),
        Outcome::Enrolled
    );
    assert_eq!(
        at_hall_service(&mut world, Intent::StudyForm),
        Outcome::Rejected(Rejection::RankRequired)
    );
    assert_eq!(
        at_hall_service(&mut world, Intent::HallDuty),
        Outcome::DutyAccepted
    );
    assert_eq!(world.hall_rank(), Some(HallRank::Apprentice));
    assert_eq!(world.region().unwrap().hall_offerings, 0);
    assert_eq!(world.school().unwrap().duty, Duty::Carrying);
    assert_eq!(world.player().herbs, 4);
    assert_eq!(
        turn(&mut world, Intent::HallDuty),
        Outcome::Rejected(Rejection::OutOfReach)
    );

    walk(&mut world, cell(22, 4), 0);
    let bytes = world.checkpoint();
    assert_eq!(&bytes[12..14], &4u16.to_le_bytes());
    let mut world = World::restore(&bytes).unwrap();
    walk(&mut world, shrine, 1);
    let deliveries = world.supply().player_deliveries;
    assert_eq!(turn(&mut world, Intent::HallDuty), Outcome::DutyDelivered);
    assert_eq!(world.supply().player_deliveries, deliveries + 1);
    assert_eq!(
        turn(&mut world, Intent::HallDuty),
        Outcome::Rejected(Rejection::OutOfReach)
    );
    assert_eq!(world.supply().player_deliveries, deliveries + 1);
    assert_eq!(world.hall_rank(), Some(HallRank::Apprentice));
    walk(&mut world, hall, 1);
    assert_eq!(
        at_hall_service(&mut world, Intent::HallDuty),
        Outcome::DutyReported
    );
    assert_eq!(world.hall_rank(), Some(HallRank::Disciple));
    assert_eq!(
        at_hall_service(&mut world, Intent::HallDuty),
        Outcome::Rejected(Rejection::DutyComplete)
    );
    assert_eq!(
        at_hall_service(&mut world, Intent::StudyForm),
        Outcome::FormLearned
    );
    assert_eq!(world.player().herbs, 2);
    for total in 1..=MAX_QI {
        assert_eq!(
            turn(&mut world, Intent::CenterQi),
            Outcome::QiCentered { total }
        );
    }
    assert_eq!(
        turn(&mut world, Intent::CenterQi),
        Outcome::Rejected(Rejection::QiFull)
    );
    assert_eq!(
        total_regional_herbs(&world),
        initial + world.supply().regrown + world.region().unwrap().garden_regrown
    );
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

#[test]
fn school_campaign_keeps_the_v3_campaign_rules_and_roster_separate() {
    let mut old = World::riverlands(1);
    // Independently encoded from the C1.2 initial-state wire layout.
    let initial = concat!(
        "53483a4331303a574f524c440300",
        "1124f09aeeba1f585404cd4890fa34aad2a7a11250c51ead2a0ffa20a40bfde0",
        "0000000001000000",
        "63c4e823f64fc5b007c300a6ea26e27ee97846e4164d1cf531740cad26bfe009",
        "a7594b2b38f9da767cba183d236e0523aa748cea3ffbbb1763da5d2eb93514c3",
    );
    let original_bytes: Vec<_> = initial
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(old.checkpoint(), original_bytes);
    assert_eq!(World::restore(&original_bytes).unwrap(), old);
    wait(&mut old, 12);
    let old_save = old.checkpoint();
    assert_eq!(
        old.step(12, Intent::HallDuty),
        Err(StepError::SchoolCampaignRequired)
    );
    assert_eq!(old.checkpoint(), old_save);
    assert_eq!(World::restore(&old_save).unwrap(), old);
    assert_eq!(&old_save[12..14], &3u16.to_le_bytes());
    assert!(old.school().is_none());
    assert_eq!(old.npcs().len(), 7);
    let new = World::reed_hall(1);
    assert_eq!(new.npcs().len(), 8);
    assert_eq!(new.school().unwrap().leader, Some(ActorId::new(33)));
}

#[test]
fn reed_strike_spends_qi_only_for_an_accepted_target_and_records_an_incident() {
    let mut world = World::reed_hall(0);
    let target = ActorId::new(22);
    world.school.as_mut().unwrap().form_learned = true;
    world.school.as_mut().unwrap().qi = FORM_QI_COST;
    assert_eq!(
        turn(&mut world, Intent::ReedStrike(target)),
        Outcome::Rejected(Rejection::OutOfReach)
    );
    assert_eq!(world.school().unwrap().qi, FORM_QI_COST);
    let position = world.npc(target).unwrap().position;
    world.player.position = cell(position.x - 1, position.y);
    assert_eq!(
        turn(&mut world, Intent::ReedStrike(target)),
        Outcome::FormStruck(target)
    );
    assert_eq!(world.school().unwrap().qi, 0);
    assert_eq!(world.npc(target).unwrap().hp, 1);
    assert!(world.social().incidents().iter().any(|incident| {
        incident.source == PLAYER && incident.target == target && incident.damage == 3
    }));
    let position = world.npc(target).unwrap().position;
    world.player.position = cell(position.x - 1, position.y);
    assert_eq!(
        turn(&mut world, Intent::ReedStrike(target)),
        Outcome::Rejected(Rejection::InsufficientQi)
    );
    assert_eq!(world.npc(target).unwrap().hp, 1);
}

#[test]
fn succession_changes_the_existing_deputys_job_and_handles_a_vacant_hall() {
    let mut world = World::reed_hall(1);
    let former = ActorId::new(33);
    let deputy = ActorId::new(34);
    let roster = world.npcs().len();
    let hall = Region::sites().hall;
    world.player.position = cell(hall.x - 1, hall.y);
    world.npcs.get_mut(&former).unwrap().hp = STRIKE;
    assert_eq!(
        turn(&mut world, Intent::Attack(former)),
        Outcome::Struck(former)
    );
    assert_eq!(world.school().unwrap().leader, Some(deputy));
    assert!(world.is_hall_teacher(world.npc(deputy).unwrap()));
    assert_eq!(
        world.school().unwrap().succession,
        Some(Succession {
            at: 1,
            former,
            successor: Some(deputy),
        })
    );
    assert_eq!(world.npcs().len(), roster);
    assert!(world.social().relationship(deputy, PLAYER).refuses_aid());
    world.npcs.get_mut(&deputy).unwrap().hp = 0;
    wait(&mut world, 2);
    assert_eq!(world.school().unwrap().leader, None);
    assert_eq!(
        world.school().unwrap().succession,
        Some(Succession {
            at: 2,
            former: deputy,
            successor: None
        })
    );
    assert_eq!(
        turn(&mut world, Intent::JoinHall),
        Outcome::Rejected(Rejection::HallClosed)
    );
}

#[test]
fn witnessed_succession_replays_from_player_commands() {
    let mut world = World::reed_hall(0);
    let hall = Region::sites().hall;
    let mentor = ActorId::new(33);
    let deputy = ActorId::new(34);
    walk(&mut world, cell(hall.x - 1, hall.y), 0);
    for _ in 0..80 {
        if world.npc(mentor).unwrap().position == hall {
            break;
        }
        wait(&mut world, 1);
    }
    assert_eq!(world.npc(mentor).unwrap().position, hall);
    for _ in 0..3 {
        assert_eq!(
            turn(&mut world, Intent::Attack(mentor)),
            Outcome::Struck(mentor)
        );
    }
    assert!(world.player().alive());
    assert!(!world.npc(mentor).unwrap().alive());
    assert_eq!(world.school().unwrap().leader, Some(deputy));
    assert_eq!(world.school().unwrap().succession.unwrap().at, world.tick());
    let mut restored = World::restore(&world.checkpoint()).unwrap();
    wait(&mut world, 60);
    wait(&mut restored, 60);
    assert_eq!(restored, world);
    assert_eq!(world.npcs().filter(|npc| npc.id() == deputy).count(), 1);
}

#[test]
fn selected_seed_controls_route_and_balances_and_is_bound_to_the_save() {
    let mut ridge = World::riverlands(0);
    let lower = World::riverlands(1);
    assert!(ridge.walkable(cell(13, 4)) && !lower.walkable(cell(13, 4)));
    assert!(!ridge.walkable(cell(13, 7)) && lower.walkable(cell(13, 7)));
    assert_eq!(
        ridge.region().unwrap().market_herbs,
        lower.region().unwrap().market_herbs
    );
    assert_ne!(
        ridge.region().unwrap().market_herbs,
        World::riverlands(2).region().unwrap().market_herbs
    );
    assert_eq!(
        turn(&mut ridge, Intent::ChooseBackground(Background::Peddler)),
        Outcome::BackgroundChosen(Background::Peddler)
    );
    let region = ridge.region().unwrap();
    assert_eq!((region.coins, region.market_coins), (6, 18));
    assert_eq!(
        turn(&mut ridge, Intent::ChooseBackground(Background::Herbalist)),
        Outcome::Rejected(Rejection::BackgroundLocked)
    );
    let mut bytes = ridge.checkpoint();
    assert_eq!(&bytes[12..14], &3u16.to_le_bytes());
    assert_eq!(World::restore(&bytes).unwrap(), ridge);
    bytes[50..54].copy_from_slice(&2u32.to_le_bytes());
    let checksum = bytes.len() - 32;
    let digest = crate::encoding::sha256(&bytes[..checksum]);
    bytes[checksum..].copy_from_slice(&digest);
    assert_eq!(World::restore(&bytes), Err(LoadError::StateMismatch));

    let mut old = World::qinghe();
    let before = old.checkpoint();
    assert_eq!(
        old.step(0, Intent::BuyHerb),
        Err(StepError::RegionalCampaignRequired)
    );
    assert_eq!(old.checkpoint(), before);
    assert_eq!(World::restore(&before).unwrap(), old);
}

#[test]
fn freight_crosses_both_roads_and_survives_reload_with_its_owner() {
    let courier = ActorId::new(32);
    for seed in [0, 1] {
        let mut world = World::riverlands(seed);
        let initial = total_regional_herbs(&world);
        wait(&mut world, 3);
        let freight = world.npc(courier).unwrap().herbs;
        assert!(freight > 0);
        assert_eq!(world.region().unwrap().shipped_to_qinghe, 0);
        assert_eq!(world.region().unwrap().order_remaining, 0);
        let mut resumed = World::restore(&world.checkpoint()).unwrap();
        wait(&mut world, 80);
        wait(&mut resumed, 80);
        assert_eq!(resumed.checkpoint(), world.checkpoint());
        assert!(world.region().unwrap().shipped_to_qinghe >= freight);
        assert_eq!(world.npcs().filter(|npc| npc.id() == courier).count(), 1);
        let supply = world.supply();
        let region = world.region().unwrap();
        assert_eq!(
            total_regional_herbs(&world),
            initial + supply.regrown + region.garden_regrown
        );
    }
}

#[test]
fn a_new_order_reaches_the_market_on_its_scheduled_turn_after_reload() {
    let mut world = World::riverlands(1);
    wait(&mut world, 48);
    let in_transit = world.region().unwrap().order_in_transit.unwrap();
    assert_eq!(in_transit.0, 78);
    let mut world = World::restore(&world.checkpoint()).unwrap();
    wait(&mut world, 29);
    assert_eq!(world.region().unwrap().order_in_transit, Some(in_transit));
    wait(&mut world, 1);
    assert_eq!(world.region().unwrap().order_in_transit, None);
}

#[test]
fn walking_to_willowford_and_back_preserves_people_and_transfers_stock() {
    let mut world = World::riverlands(1);
    let initial = total_regional_herbs(&world);
    turn(&mut world, Intent::ChooseBackground(Background::Peddler));
    let sites = Region::sites();
    walk(&mut world, sites.market, 1);
    let before = world.region().unwrap().coins;
    let priced = world.region().unwrap().quote().buy;
    assert_eq!(
        turn(&mut world, Intent::BuyHerb),
        Outcome::Traded {
            bought: true,
            coins: priced
        }
    );
    assert_eq!(world.region().unwrap().coins, before - priced);
    assert_eq!(world.player().herbs, 1);
    assert!(matches!(
        turn(&mut world, Intent::SellHerb),
        Outcome::Traded { bought: false, .. }
    ));
    assert_eq!(world.player().herbs, 0);
    assert!(world.region().unwrap().coins <= before);
    assert!(matches!(
        turn(&mut world, Intent::BuyHerb),
        Outcome::Traded { bought: true, .. }
    ));
    let mut world = World::restore(&world.checkpoint()).unwrap();
    walk(&mut world, cell(3, 5), 1);
    assert!(matches!(
        turn(&mut world, Intent::Deliver),
        Outcome::Delivered | Outcome::Contribution { .. }
    ));
    assert_eq!(world.player().herbs, 0);
    assert_eq!(
        world.region().unwrap().market_coins + world.region().unwrap().coins,
        24
    );
    assert_eq!(
        total_regional_herbs(&world),
        initial + world.supply().regrown + world.region().unwrap().garden_regrown
    );
    assert_eq!(world.npcs().len(), 7);
}

#[test]
fn joining_and_brewing_require_training_membership_and_real_cargo() {
    let mut world = World::riverlands(0);
    let initial = total_regional_herbs(&world);
    turn(&mut world, Intent::ChooseBackground(Background::Herbalist));
    let patch = world.scenario.patch;
    walk(&mut world, patch, 1);
    for _ in 0..2 {
        assert_eq!(turn(&mut world, Intent::Gather), Outcome::GatheredPair);
    }
    walk(&mut world, cell(3, 5), 1);
    assert_eq!(
        turn(&mut world, Intent::Practice),
        Outcome::Practiced { progress: 1 }
    );
    walk(&mut world, cell(27, 6), 1);
    // The mentor follows a duty cycle. Try the action on canonical turns
    // until the service reopens, keeping all attempted actions in the save.
    let first = (0..40)
        .map(|_| turn(&mut world, Intent::Brew))
        .find(|outcome| *outcome != Outcome::Rejected(Rejection::HallClosed));
    assert_eq!(
        first,
        Some(Outcome::Rejected(Rejection::MembershipRequired))
    );
    let enrolled = (0..40)
        .map(|_| turn(&mut world, Intent::JoinHall))
        .find(|outcome| *outcome != Outcome::Rejected(Rejection::HallClosed));
    assert_eq!(enrolled, Some(Outcome::Enrolled));
    let brewed = (0..40)
        .map(|_| turn(&mut world, Intent::Brew))
        .find(|outcome| *outcome != Outcome::Rejected(Rejection::HallClosed));
    assert_eq!(brewed, Some(Outcome::Brewed));
    assert_eq!(world.region().unwrap().remedies, 1);
    assert_eq!(
        turn(&mut world, Intent::UseRemedy),
        Outcome::Rejected(Rejection::Uninjured)
    );
    assert_eq!(
        total_regional_herbs(&world),
        initial + world.supply().regrown + world.region().unwrap().garden_regrown
    );
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

/// Wait through the outrider's patrol and fatigue cycle without changing any state outside step.
fn meet_on_patrol(world: &mut World, intent: Intent) -> Outcome {
    for _ in 0..100 {
        if in_reach(world.player.position, world.npcs[&TRAVELER].position) {
            return turn(world, intent);
        }
        wait(world, 1);
    }
    panic!("outrider did not return to the crossing");
}

#[test]
fn company_recruitment_cargo_and_separation_replay_from_inputs_on_both_roads() {
    for seed in [0, 1] {
        let mut world = World::road_company(seed);
        let opening = total_regional_herbs(&world);
        walk(&mut world, cell(9, 3), 0);
        turn(&mut world, Intent::Gather);
        assert_eq!(world.player.herbs, 1);
        assert_eq!(
            meet_on_patrol(&mut world, Intent::Invite(TRAVELER)),
            Outcome::Rejected(Rejection::IntroductionRequired)
        );
        assert_eq!(
            meet_on_patrol(&mut world, Intent::Talk(TRAVELER)),
            Outcome::Spoke(TRAVELER)
        );
        assert_eq!(
            meet_on_patrol(&mut world, Intent::Invite(TRAVELER)),
            Outcome::JoinedCompany(TRAVELER)
        );
        assert_eq!(
            turn(&mut world, Intent::Invite(TRAVELER)),
            Outcome::Rejected(Rejection::CompanyFull)
        );
        assert_eq!(
            turn(&mut world, Intent::Provision(TRAVELER)),
            Outcome::Provisioned(TRAVELER)
        );
        let bytes = world.checkpoint();
        assert_eq!(&bytes[12..14], &5u16.to_le_bytes());
        let mut restored = World::restore(&bytes).unwrap();
        assert_eq!(world, restored);
        let since = world.tick();
        let start = world.npcs[&TRAVELER].position;
        for game in [&mut world, &mut restored] {
            assert!(game.step(since - 1, Intent::Provision(TRAVELER)).is_err());
            assert_eq!(game.checkpoint(), bytes);
            // The open northern edge leads away from the patrol. Wait at each
            // step to allow the companion to finish a field rest and catch up.
            for destination in [cell(9, 0), cell(7, 0)] {
                walk(game, destination, 0);
                for _ in 0..20 {
                    if in_reach(game.player.position, game.npcs[&TRAVELER].position) {
                        break;
                    }
                    wait(game, 1);
                }
            }
            assert_ne!(game.npcs[&TRAVELER].position, start);
            assert_eq!(
                turn(game, Intent::Part(TRAVELER)),
                Outcome::Parted(TRAVELER)
            );
            assert_eq!(game.npcs[&TRAVELER].packed_herbs, 1);
            wait(game, 8);
            assert!(game.company().unwrap().journey.is_none());
            assert_eq!(game.company().unwrap().history.len(), 2);
            assert_eq!(
                total_regional_herbs(game),
                opening + game.supply.regrown + game.region().unwrap().garden_regrown
            );
        }
        assert_eq!(restored.checkpoint(), world.checkpoint());
    }
}

fn company_fixture() -> World {
    let mut world = World::road_company(0);
    world.player.position = cell(9, 3);
    world.social.meet(TRAVELER, PLAYER);
    assert_eq!(
        turn(&mut world, Intent::Invite(TRAVELER)),
        Outcome::JoinedCompany(TRAVELER)
    );
    world
}

#[test]
fn field_medicine_uses_the_carriers_stock_and_checks_life_at_commit() {
    let mut world = company_fixture();
    world.player.herbs = 3;
    turn(&mut world, Intent::Provision(TRAVELER));
    turn(&mut world, Intent::Provision(TRAVELER));
    assert_eq!(
        turn(&mut world, Intent::Provision(TRAVELER)),
        Outcome::Rejected(Rejection::PackFull)
    );
    let shrine = world.supply.consumed;
    world.player.hp -= 1;
    let previous = world.social.relationship(PLAYER, TRAVELER).regard;
    wait(&mut world, 1);
    assert_eq!(world.player.hp, PLAYER_HP);
    assert_eq!(world.npcs[&TRAVELER].packed_herbs, 1);
    assert_eq!(world.company().unwrap().herbs_used, 1);
    assert_eq!(world.supply.consumed, shrine);
    assert_eq!(
        world.social.relationship(PLAYER, TRAVELER).regard,
        previous + 1
    );
    let entry = world.company().unwrap().history.last().unwrap();
    assert!(entry.witnessed && entry.kind == JourneyEvent::FieldCare);
    world.player.hp = 0;
    world.field_care(TRAVELER);
    assert_eq!(world.player.hp, 0);
    assert_eq!(world.npcs[&TRAVELER].packed_herbs, 1);
    wait(&mut world, 1);
    assert_eq!(
        world.company().unwrap().history.last().unwrap().kind,
        JourneyEvent::PlayerFallen
    );
    let records = world.company().unwrap().history.len();
    wait(&mut world, 64);
    assert_eq!(world.company().unwrap().history.len(), records);
    assert!(world.region().unwrap().shipped_to_qinghe > 0);
}

#[test]
fn companion_camps_when_tired_and_searches_only_the_last_seen_tile() {
    let mut world = company_fixture();
    world.npcs.get_mut(&TRAVELER).unwrap().fatigue = 20;
    let camp = world.npcs[&TRAVELER].position;
    wait(&mut world, 1);
    assert_eq!(world.npcs[&TRAVELER].activity.goal, Goal::Rest);
    assert_eq!(world.npcs[&TRAVELER].position, camp);
    assert_eq!(world.npcs[&TRAVELER].fatigue, 16);
    let known = world.companion(TRAVELER).unwrap();
    world.player.position = cell(25, 0);
    wait(&mut world, 8);
    assert_eq!(world.companion(TRAVELER), Some(known));
    assert_eq!(
        world.npcs[&TRAVELER].activity.goal,
        Goal::Follow(known.last_seen)
    );
    wait(&mut world, 4);
    assert!(world.companion(TRAVELER).is_none());
    let ended = world.company().unwrap().history.last().unwrap();
    assert_eq!(ended.kind, JourneyEvent::LostContact);
    assert!(!ended.witnessed);
    assert_eq!(ended.at, known.seen_at + 12);
}

#[test]
fn observed_violence_breaks_a_partnership_but_an_unseen_attack_does_not() {
    let mut observed = company_fixture();
    let mut unseen = observed.clone();
    // Same blow and victim, with the observer on opposite sides of Qinghe's wall.
    let attacker = cell(7, 2);
    for world in [&mut observed, &mut unseen] {
        world.player.position = attacker;
        world.npcs.get_mut(&SUPPLIER).unwrap().position = cell(8, 2);
    }
    unseen.npcs.get_mut(&TRAVELER).unwrap().position = cell(5, 2);
    observed.npcs.get_mut(&TRAVELER).unwrap().position = cell(8, 3);
    for world in [&mut observed, &mut unseen] {
        assert_eq!(
            turn(world, Intent::Attack(SUPPLIER)),
            Outcome::Struck(SUPPLIER)
        );
    }
    assert!(observed.companion(TRAVELER).is_none());
    assert_eq!(
        observed.company().unwrap().history.last().unwrap().kind,
        JourneyEvent::Distrust
    );
    assert!(unseen.companion(TRAVELER).is_some());
    assert!(unseen.social.knowledge(TRAVELER, IncidentId(0)).is_none());
}

#[test]
fn a_deputy_needs_evidence_before_leaving_the_party_to_take_office() {
    let mut world = World::road_company(1);
    let former = ActorId::new(33);
    let deputy = ActorId::new(34);
    let courier = ActorId::new(32);
    world.player.position = cell(26, 7);
    world.social.meet(deputy, PLAYER);
    assert_eq!(world.invite(deputy), Err(Rejection::MembershipRequired));
    world.region.as_mut().unwrap().enrolled = true;
    assert_eq!(world.invite(deputy), Ok(Outcome::JoinedCompany(deputy)));
    world.player.position = cell(26, 6);
    world.npcs.get_mut(&deputy).unwrap().position = cell(20, 7);
    world.npcs.get_mut(&courier).unwrap().position = cell(26, 5);
    world.damage(PLAYER, former, 6, false);
    world.update_succession();
    world.update_journey();
    assert_eq!(world.school().unwrap().leader, None);
    assert!(world.companion(deputy).is_some());
    assert!(world.social.knowledge(deputy, IncidentId(0)).is_none());
    // The eyewitness meets the deputy out of sight of the body.
    world.npcs.get_mut(&courier).unwrap().position = cell(21, 7);
    // Other contacts cannot intercept this isolated conversation.
    world.queue_reports();
    assert!(
        world
            .social
            .pending_reports()
            .any(|report| report.recipient == deputy)
    );
    wait(&mut world, 1);
    assert_eq!(world.school().unwrap().leader, None);
    wait(&mut world, 1);
    let school = world.school().unwrap();
    assert_eq!(school.leader, Some(deputy));
    assert_eq!(school.history.len(), 2);
    assert_eq!(school.history[0].successor, None);
    assert_eq!(school.history[1].at, 2);
    assert!(
        matches!(world.social.knowledge(deputy, IncidentId(0)).unwrap().source,
        KnowledgeSource::Reported { sender, .. } if sender == courier)
    );
    assert_eq!(
        world.company().unwrap().history.last().unwrap().kind,
        JourneyEvent::Office
    );
    assert!(world.social.relationship(deputy, PLAYER).refuses_aid());
    assert!(world.companion(deputy).is_none());
    wait(&mut world, 24);
    assert_eq!(world.school().unwrap().history.len(), 2);
}

#[test]
fn v4_refuses_v5_actions_before_mutation_and_checks_tags_on_restore() {
    // Frozen v4 seed-one envelope, encoded independently of World::checkpoint.
    let frozen = concat!(
        "53483a4331303a574f524c440400",
        "a74dbb94e1b95a34e4b224de384a8826b89ed2d227ef70b5254e4f0ca5959a15",
        "0000000001000000",
        "0dede4fd952e6a171a098024a459eae94eab7aa602c0c5093f637709a8a39c7e",
        "5ce08e67b06ff31f3d58dfe600b4a2145e4847bd85aaa00a5a9ab8fd9bc12fb8",
    );
    let frozen: Vec<_> = frozen
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let mut old = World::restore(&frozen).unwrap();
    assert_eq!(old, World::reed_hall(1));
    assert_eq!(old.checkpoint(), frozen);
    wait(&mut old, 1);
    let before = old.checkpoint();
    assert_eq!(
        old.step(1, Intent::Invite(TRAVELER)),
        Err(StepError::CompanyCampaignRequired)
    );
    assert_eq!(old.checkpoint(), before);
    assert_eq!(World::restore(&before).unwrap(), old);
    let mut bytes = before;
    bytes[54] = 18;
    bytes[55..59].copy_from_slice(&TRAVELER.get().to_le_bytes());
    let checksum = bytes.len() - 32;
    let digest = crate::encoding::sha256(&bytes[..checksum]);
    bytes[checksum..].copy_from_slice(&digest);
    assert_eq!(World::restore(&bytes), Err(LoadError::InvalidIntent));
}

#[test]
fn shrine_job_reserves_only_westbound_freight() {
    let mut world = World::road_company(0);
    let courier = ActorId::new(32);
    let grower = ActorId::new(31);
    world.supply.shrine = world.scenario.shrine_target - 1;
    world.npcs.get_mut(&grower).unwrap().herbs = 2;
    world.npcs.get_mut(&TRAVELER).unwrap().packed_herbs = 2;
    world.npcs.get_mut(&courier).unwrap().herbs = 1;
    assert_ne!(
        world.choose_goal(&world.npcs[&SUPPLIER]),
        Goal::CollectHerbs
    );
    world.npcs.get_mut(&courier).unwrap().herbs = 0;
    world.npcs.get_mut(&SUPPLIER).unwrap().position = world.scenario.patch;
    assert_eq!(
        world.choose_goal(&world.npcs[&SUPPLIER]),
        Goal::CollectHerbs
    );
    let source = world.supply.source;
    wait(&mut world, 1);
    assert_eq!(world.supply.source, source - 1);
    assert_eq!(world.npcs[&SUPPLIER].herbs, 1);
}

#[test]
fn a_distant_companion_does_not_learn_of_the_players_death_over_the_horizon() {
    let mut world = company_fixture();
    world.player.position = cell(27, 0);
    world.player.hp = 0;
    let last = world.companion(TRAVELER).unwrap();
    wait(&mut world, 1);
    assert_eq!(world.companion(TRAVELER), Some(last));
    wait(&mut world, 11);
    let entry = world.company().unwrap().history.last().unwrap();
    assert_eq!(entry.kind, JourneyEvent::LostContact);
    assert!(!entry.witnessed);
}

#[test]
fn lethal_retaliation_resolves_before_a_higher_id_companions_planned_care() {
    let mut world = company_fixture();
    world.npcs.get_mut(&WATCH).unwrap().position = cell(8, 3);
    // Only the watch and local bystanders know of the earlier blow.
    world.npcs.get_mut(&TRAVELER).unwrap().position = cell(25, 0);
    world.damage(PLAYER, WATCH, 1, false);
    world.npcs.get_mut(&TRAVELER).unwrap().position = cell(9, 4);
    world.npcs.get_mut(&TRAVELER).unwrap().packed_herbs = 1;
    world.player.hp = STRIKE;
    assert_eq!(world.choose_goal(&world.npcs[&TRAVELER]), Goal::FieldCare);
    assert_eq!(world.choose_goal(&world.npcs[&WATCH]), Goal::Retaliate);
    wait(&mut world, 1);
    assert!(!world.player.alive());
    assert_eq!(world.npcs[&TRAVELER].packed_herbs, 1);
    assert_eq!(world.company().unwrap().herbs_used, 0);
    assert_eq!(
        world.company().unwrap().history.last().unwrap().kind,
        JourneyEvent::PlayerFallen
    );
}

#[test]
fn discovering_the_teachers_body_confers_office_without_identifying_the_killer() {
    let mut world = World::road_company(0);
    let teacher = ActorId::new(33);
    let deputy = ActorId::new(34);
    world.player.position = cell(26, 6);
    world.npcs.get_mut(&deputy).unwrap().position = cell(9, 3);
    world.damage(PLAYER, teacher, 6, false);
    world.update_succession();
    assert_eq!(world.school().unwrap().leader, None);
    world.player.position = cell(8, 4);
    world.npcs.get_mut(&deputy).unwrap().position = cell(27, 7);
    wait(&mut world, 1);
    assert_eq!(world.school().unwrap().leader, Some(deputy));
    assert!(world.social.knowledge(deputy, IncidentId(0)).is_none());
    assert!(!world.social.relationship(deputy, PLAYER).refuses_aid());
}

#[test]
fn stonebank_envoy_uses_real_market_stock_and_petitions_on_both_roads() {
    for seed in [0, 1] {
        let mut world = World::sect_rivalry(seed);
        assert!(world.scenario().valid());
        let initial_market = world.region().unwrap().market_herbs;
        let initial_lodge = world.rivalry().unwrap().lodge_coins;
        wait(&mut world, 60);
        let rival = world.rivalry().unwrap();
        assert!(
            rival.market_purchases > 0,
            "seed {seed}: envoy did not trade"
        );
        assert!(
            rival.petitions > 0,
            "seed {seed}: envoy did not reach the hall"
        );
        assert_eq!(rival.petitions, rival.consumed_herbs);
        assert_eq!(
            rival.market_purchases,
            rival.petitions + world.npc(ActorId::new(35)).unwrap().herbs
        );
        assert!(rival.lodge_coins < initial_lodge);
        assert!(
            world.region().unwrap().market_herbs
                <= initial_market + 10 + world.region().unwrap().garden_regrown
        );
        assert_eq!(
            world.region().unwrap().market_coins,
            24 - 3 + initial_lodge - rival.lodge_coins
        );
        assert!(
            rival
                .history
                .iter()
                .any(|event| event.kind == SectEventKind::Petitioned)
        );
        assert!(
            rival.history.iter().all(|event| !event.witnessed),
            "player at the entrance should not have omniscient rivalry history"
        );
        assert_eq!(
            world.market_quote().unwrap().buy,
            world.region().unwrap().quote().buy + u32::from(rival.contested(world.tick()))
        );
    }
}

#[test]
fn sect_rivalry_replays_purchases_claims_and_market_effects_exactly() {
    let mut world = World::sect_rivalry(0);
    wait(&mut world, 100);
    let checkpoint = world.checkpoint();
    assert_eq!(&checkpoint[..12], b"SH:C10:WORLD");
    assert_eq!(u16::from_le_bytes([checkpoint[12], checkpoint[13]]), 6);
    assert_eq!(World::restore(&checkpoint), Ok(world.clone()));
    let mut restored = World::restore(&checkpoint).unwrap();
    wait(&mut restored, 50);
    wait(&mut world, 50);
    assert_eq!(restored, world);
}

#[test]
fn accord_and_patronage_have_material_cost_and_distinct_outcomes() {
    let mut accord = World::sect_rivalry(0);
    accord.player.position = accord.npc(ActorId::new(35)).unwrap().position;
    accord.player.herbs = 1;
    accord.rivalry.as_mut().unwrap().pressure = 6;
    assert_eq!(
        turn(&mut accord, Intent::OfferAccord),
        Outcome::LodgeAccord { until: 49 }
    );
    let rival = accord.rivalry().unwrap();
    assert_eq!(rival.pressure, 3);
    assert_eq!(rival.accord_offerings, 1);
    assert!(rival.at_peace(accord.tick()));
    assert_eq!(accord.player().herbs, 0);
    assert_eq!(rival.history.last().unwrap().kind, SectEventKind::Accord);

    let mut patronage = World::sect_rivalry(1);
    patronage.player.position = Region::sites().hall;
    patronage.player.herbs = 1;
    patronage.rivalry.as_mut().unwrap().pressure = 6;
    assert_eq!(
        turn(&mut patronage, Intent::SupportHall),
        Outcome::Rejected(Rejection::MembershipRequired)
    );
    patronage.region.as_mut().unwrap().enrolled = true;
    assert_eq!(
        turn(&mut patronage, Intent::SupportHall),
        Outcome::HallSupported
    );
    assert_eq!(patronage.rivalry().unwrap().pressure, 4);
    assert_eq!(patronage.rivalry().unwrap().hall_patronage, 1);
    assert_eq!(patronage.region().unwrap().hall_offerings, 1);
    assert_eq!(patronage.player().herbs, 0);
}

#[test]
fn older_campaigns_reject_new_intents_without_advancing_and_keep_revision_five() {
    let mut old = World::road_company(0);
    let checkpoint = old.checkpoint();
    assert_eq!(u16::from_le_bytes([checkpoint[12], checkpoint[13]]), 5);
    for action in [Intent::OfferAccord, Intent::SupportHall] {
        assert_eq!(
            old.step(old.tick(), action),
            Err(StepError::RivalryCampaignRequired)
        );
        assert_eq!(old.checkpoint(), checkpoint);
    }
    assert_eq!(World::restore(&checkpoint), Ok(old));
}

#[test]
fn contested_teaching_and_market_cost_are_reversible() {
    let mut world = World::sect_rivalry(0);
    world.player.position = Region::sites().hall;
    world.region.as_mut().unwrap().enrolled = true;
    world.rivalry.as_mut().unwrap().pressure = 4;
    assert_eq!(world.hall_mentor(), Err(Rejection::SchoolContested));
    assert_eq!(
        world.market_quote().unwrap().buy,
        world.region().unwrap().quote().buy + 1
    );
    world.rivalry.as_mut().unwrap().pressure = 2;
    assert_eq!(
        world.market_quote().unwrap().buy,
        world.region().unwrap().quote().buy
    );
    assert_ne!(world.hall_mentor(), Err(Rejection::SchoolContested));
}

#[test]
fn envoy_trade_obeys_market_hours_and_finite_treasury() {
    let mut world = World::sect_rivalry(0);
    assert!(world.rival_can_buy());
    world.tick = (DUSK_MINUTE - DAWN_MINUTE) * TURNS_PER_MINUTE;
    assert!(world.is_night());
    assert!(!world.rival_can_buy());
    world.tick = 0;
    world.rivalry.as_mut().unwrap().lodge_coins = 0;
    assert!(!world.rival_can_buy());
    world.rivalry.as_mut().unwrap().lodge_coins = 16;
    world.region.as_mut().unwrap().market_herbs = 0;
    assert!(!world.rival_can_buy());
}

#[test]
fn competing_hall_and_lodge_buyers_have_finite_material_ledgers_on_both_roads() {
    for seed in [0, 1] {
        let mut world = World::sect_politics(seed);
        assert!(world.scenario().valid());
        wait(&mut world, 240);
        let politics = world.politics().unwrap();
        let rival = world.rivalry().unwrap();
        assert!(
            rival.market_purchases > 0,
            "seed {seed}: lodge did not trade"
        );
        assert!(rival.petitions > 0, "seed {seed}: no Hall petition");
        assert!(
            politics.hall_purchases > 0,
            "seed {seed}: Hall steward did not trade"
        );
        assert!(
            politics.hall_supplies > 0,
            "seed {seed}: Hall steward did not return"
        );
        assert_eq!(
            politics.hall_purchases,
            politics.hall_supplies + world.npc(ActorId::new(36)).unwrap().herbs
        );
        assert_eq!(
            rival.market_purchases,
            rival.petitions + world.npc(ActorId::new(35)).unwrap().herbs
        );
        assert_eq!(
            world.region().unwrap().market_coins,
            24 - 3 + (16 - rival.lodge_coins) + (18 - politics.hall_coins)
        );
        assert_eq!(
            politics.hall_supplies,
            world.region().unwrap().hall_offerings
        );
        assert!(
            politics
                .history
                .iter()
                .any(|event| event.kind == PoliticalEventKind::HallSupply)
        );
        assert!(
            politics.history.iter().all(|event| !event.witnessed),
            "events out of the player's sight must stay private"
        );
    }
}

#[test]
fn political_checkpoint_revision_seven_replays_and_old_revision_six_remains_isolated() {
    let mut world = World::sect_politics(1);
    wait(&mut world, 110);
    let bytes = world.checkpoint();
    assert_eq!(u16::from_le_bytes([bytes[12], bytes[13]]), 7);
    assert_eq!(World::restore(&bytes), Ok(world.clone()));
    let mut loaded = World::restore(&bytes).unwrap();
    wait(&mut loaded, 90);
    wait(&mut world, 90);
    assert_eq!(loaded, world);

    let mut old = World::sect_rivalry(0);
    let baseline = old.checkpoint();
    assert_eq!(u16::from_le_bytes([baseline[12], baseline[13]]), 6);
    for intent in [Intent::PledgeLodge, Intent::AidLodge, Intent::RenounceLodge] {
        assert_eq!(
            old.step(old.tick(), intent),
            Err(StepError::PoliticsCampaignRequired)
        );
        assert_eq!(old.checkpoint(), baseline);
    }
    assert_eq!(World::restore(&baseline), Ok(old));
}

#[test]
fn lodge_oath_requires_introduction_spends_real_herbs_and_forfeits_hall_rank() {
    let mut world = World::sect_politics(0);
    world.player.position = world.npc(ActorId::new(35)).unwrap().position;
    world.player.herbs = 2;
    world.region.as_mut().unwrap().enrolled = true;
    world.school.as_mut().unwrap().duty = Duty::Reported;
    assert_eq!(
        turn(&mut world, Intent::PledgeLodge),
        Outcome::Rejected(Rejection::IntroductionRequired)
    );
    world.player.position = world.npc(ActorId::new(35)).unwrap().position;
    assert_eq!(
        turn(&mut world, Intent::Talk(ActorId::new(35))),
        Outcome::Spoke(ActorId::new(35))
    );
    world.player.position = world.npc(ActorId::new(35)).unwrap().position;
    assert_eq!(turn(&mut world, Intent::PledgeLodge), Outcome::LodgePledged);
    assert_eq!(world.politics().unwrap().allegiance, Allegiance::Stonebank);
    assert_eq!(world.hall_rank(), None);
    assert_eq!(world.school().unwrap().duty, Duty::Unassigned);
    assert_eq!(world.player.herbs, 1);
    assert_eq!(world.politics().unwrap().lodge_oaths, 1);

    world.player.position = world.npc(ActorId::new(35)).unwrap().position;
    assert_eq!(turn(&mut world, Intent::AidLodge), Outcome::LodgeAided);
    assert_eq!(world.player.herbs, 0);
    assert_eq!(world.politics().unwrap().lodge_aid, 1);
    assert_eq!(
        world.rivalry().unwrap().market_purchases
            + world.politics().unwrap().lodge_oaths
            + world.politics().unwrap().lodge_aid,
        world.rivalry().unwrap().petitions + world.npc(ActorId::new(35)).unwrap().herbs
    );
    world.player.position = Region::sites().hall;
    assert_eq!(
        turn(&mut world, Intent::RenounceLodge),
        Outcome::LodgeRenounced
    );
    assert_eq!(
        world.politics().unwrap().allegiance,
        Allegiance::Independent
    );
}

#[test]
fn an_unfinished_hall_parcel_blocks_defection_without_consuming_it() {
    let mut world = World::sect_politics(0);
    world.player.position = world.npc(ActorId::new(35)).unwrap().position;
    world.player.herbs = 1;
    world.social.meet(ActorId::new(35), PLAYER);
    world.school.as_mut().unwrap().duty = Duty::Carrying;
    assert_eq!(
        turn(&mut world, Intent::PledgeLodge),
        Outcome::Rejected(Rejection::BoundToDuty)
    );
    assert_eq!(world.player.herbs, 1);
    assert_eq!(
        world.politics().unwrap().allegiance,
        Allegiance::Independent
    );
}

#[test]
fn allegiance_limits_companion_recruitment_and_renunciation_dissolves_it() {
    let envoy = ActorId::new(35);
    let mut world = World::sect_politics(0);
    world.player.position = world.npc(envoy).unwrap().position;
    world.player.herbs = 2;
    world.social.meet(envoy, PLAYER);
    assert_eq!(
        turn(&mut world, Intent::Invite(envoy)),
        Outcome::Rejected(Rejection::OathConflict)
    );
    world.player.position = world.npc(envoy).unwrap().position;
    assert_eq!(turn(&mut world, Intent::PledgeLodge), Outcome::LodgePledged);
    world.player.position = world.npc(envoy).unwrap().position;
    assert_eq!(
        turn(&mut world, Intent::Invite(envoy)),
        Outcome::Rejected(Rejection::FactionTrustRequired)
    );
    world.player.position = world.npc(envoy).unwrap().position;
    assert_eq!(turn(&mut world, Intent::AidLodge), Outcome::LodgeAided);
    world.player.position = world.npc(envoy).unwrap().position;
    assert_eq!(
        turn(&mut world, Intent::Invite(envoy)),
        Outcome::JoinedCompany(envoy)
    );
    assert_eq!(world.company().unwrap().journey.unwrap().actor, envoy);
    world.player.position = Region::sites().hall;
    assert_eq!(
        turn(&mut world, Intent::RenounceLodge),
        Outcome::LodgeRenounced
    );
    assert_eq!(world.company().unwrap().journey, None);
    assert_eq!(
        world.company().unwrap().history.last().unwrap().kind,
        JourneyEvent::Distrust
    );
}

#[test]
fn hall_steward_cannot_buy_after_closing_or_without_treasury() {
    let mut world = World::sect_politics(0);
    world.rivalry.as_mut().unwrap().pressure = 3;
    assert!(world.hall_can_buy());
    world.tick = (DUSK_MINUTE - DAWN_MINUTE) * TURNS_PER_MINUTE;
    assert!(!world.hall_can_buy());
    world.tick = 0;
    world.politics.as_mut().unwrap().hall_coins = 0;
    assert!(!world.hall_can_buy());
    world.politics.as_mut().unwrap().hall_coins = 18;
    world.region.as_mut().unwrap().market_herbs = 0;
    assert!(!world.hall_can_buy());
}

#[test]
fn hall_patronage_unlocks_the_steward_as_a_companion() {
    let steward = ActorId::new(36);
    let mut world = World::sect_politics(1);
    world.region.as_mut().unwrap().enrolled = true;
    world.player.herbs = 1;
    world.social.meet(steward, PLAYER);
    world.player.position = world.npc(steward).unwrap().position;
    assert_eq!(
        turn(&mut world, Intent::Invite(steward)),
        Outcome::Rejected(Rejection::FactionTrustRequired)
    );
    world.player.position = Region::sites().hall;
    assert_eq!(
        turn(&mut world, Intent::SupportHall),
        Outcome::HallSupported
    );
    assert_eq!(world.politics().unwrap().hall_favor, 1);
    world.player.position = world.npc(steward).unwrap().position;
    assert_eq!(
        turn(&mut world, Intent::Invite(steward)),
        Outcome::JoinedCompany(steward)
    );
}

#[test]
fn a_occupied_one_tile_gate_stays_blocked_until_the_occupant_moves() {
    const GATE: Scenario = Scenario {
        name: "Narrow gate",
        width: 5,
        height: 5,
        terrain: &["..#..", "..#..", ".....", "..#..", "..#.."],
        entrance: cell(0, 2),
        clinic: cell(0, 1),
        patch: cell(0, 3),
        shrine: cell(1, 3),
        shrine_target: 1,
        initial_source: 2,
        regrow_every: 12,
        npcs: &[actor(WATCH, Role::Watch, 'W', cell(2, 2))],
        contacts: &[],
        recruits: &[],
    };
    let mut world = World::new(&GATE);
    let start = cell(1, 2);
    let goal = cell(3, 2);
    assert!(
        next_step(world.scenario(), start, goal, 0, |cell| {
            world.walkable(cell) && !world.occupied(cell)
        })
        .is_none()
    );
    wait(&mut world, 5);
    assert_eq!(world.npc(WATCH).unwrap().position, cell(2, 2));
    world.npcs.get_mut(&WATCH).unwrap().position = cell(4, 3);
    assert_eq!(
        next_step(world.scenario(), start, goal, 0, |cell| {
            world.walkable(cell) && !world.occupied(cell)
        }),
        Some(cell(2, 2))
    );
}

const COURTESY_GATE: Scenario = Scenario {
    name: "Traffic test gate",
    width: 5,
    height: 5,
    terrain: &["..#..", "..#..", ".....", "..#..", "..#.."],
    entrance: cell(0, 2),
    clinic: cell(0, 1),
    patch: cell(0, 3),
    shrine: cell(1, 3),
    shrine_target: 1,
    initial_source: 2,
    regrow_every: 12,
    npcs: &[
        actor(WATCH, Role::Watch, 'W', cell(2, 2)),
        actor(SUPPLIER, Role::Traveler, 'S', cell(1, 2)),
    ],
    contacts: &[],
    recruits: &[],
};

#[test]
fn traffic_courtesy_player_exchanges_one_cell_without_double_moving_the_blocker() {
    let mut world = World::with_rules(&COURTESY_GATE, Rules::TrafficCourtesy);
    world.npcs.get_mut(&SUPPLIER).unwrap().position = cell(4, 2);
    assert_eq!(
        turn(&mut world, Intent::Move(Direction::East)),
        Outcome::Moved
    );
    assert_eq!(world.player().position, cell(1, 2));
    assert_eq!(
        turn(&mut world, Intent::Move(Direction::East)),
        Outcome::Moved
    );
    assert_eq!(world.player().position, cell(2, 2));
    assert_eq!(world.npc(WATCH).unwrap().position, cell(1, 2));
    assert_eq!(world.traffic().unwrap().swaps, 1);
    let event = world.traffic().unwrap().history[0];
    assert_eq!(event.kind, TrafficEventKind::PlayerPass);
    assert_eq!(event.other, WATCH);
    assert!(event.witnessed);
    // An eligible actor that already yielded must not execute another action
    // on the same canonical turn.
    assert_eq!(world.traffic().unwrap().history.len(), 1);
}

#[test]
fn traffic_courtesy_npc_reaches_the_gate_after_two_failed_requests() {
    let mut world = World::with_rules(&COURTESY_GATE, Rules::TrafficCourtesy);
    let goal = cell(3, 2);
    world.travel(SUPPLIER, goal, 0);
    assert_eq!(world.npc(SUPPLIER).unwrap().position, cell(1, 2));
    assert_eq!(world.traffic().unwrap().stalls[&SUPPLIER], 1);
    world.travel(SUPPLIER, goal, 0);
    assert_eq!(world.npc(SUPPLIER).unwrap().position, cell(2, 2));
    assert_eq!(world.npc(WATCH).unwrap().position, cell(1, 2));
    assert_eq!(world.traffic().unwrap().swaps, 1);
    assert_eq!(
        world.traffic().unwrap().history[0].kind,
        TrafficEventKind::NpcPass
    );
    assert!(world.traffic().unwrap().was_committed(SUPPLIER));
    assert!(world.traffic().unwrap().was_committed(WATCH));
}

#[test]
fn traffic_courtesy_refuses_busy_and_hostile_blockers() {
    let mut world = World::with_rules(&COURTESY_GATE, Rules::TrafficCourtesy);
    world.npcs.get_mut(&SUPPLIER).unwrap().position = cell(4, 2);
    world.player.position = cell(1, 2);
    world.npcs.get_mut(&WATCH).unwrap().activity.goal = Goal::Retaliate;
    assert_eq!(
        turn(&mut world, Intent::Move(Direction::East)),
        Outcome::Rejected(Rejection::Occupied)
    );
    assert_eq!(world.player().position, cell(1, 2));
    assert_eq!(world.traffic().unwrap().swaps, 0);
    world.npcs.get_mut(&WATCH).unwrap().activity.goal = Goal::Idle;
    world
        .social
        .relations
        .entry((WATCH, PLAYER))
        .or_default()
        .regard = -8;
    assert_eq!(
        turn(&mut world, Intent::Move(Direction::East)),
        Outcome::Rejected(Rejection::Occupied)
    );
    assert_eq!(world.traffic().unwrap().swaps, 0);
}

#[test]
fn old_campaigns_do_not_gain_courtesy_or_change_their_revision() {
    let mut old = World::with_rules(&COURTESY_GATE, Rules::Qinghe);
    old.npcs.get_mut(&SUPPLIER).unwrap().position = cell(4, 2);
    old.player.position = cell(1, 2);
    assert_eq!(
        turn(&mut old, Intent::Move(Direction::East)),
        Outcome::Rejected(Rejection::Occupied)
    );
    assert_eq!(old.player().position, cell(1, 2));
    for seed in [0, 1] {
        let mut old = World::sect_politics(seed);
        wait(&mut old, 25);
        let save = old.checkpoint();
        assert_eq!(u16::from_le_bytes([save[12], save[13]]), 7);
        assert_eq!(World::restore(&save).unwrap(), old);
    }
}

#[test]
fn traffic_courtesy_replays_two_seed_campaigns_and_keeps_unique_positions() {
    for seed in [0, 1] {
        let mut world = World::traffic_courtesy(seed);
        for _ in 0..200 {
            turn(&mut world, Intent::Wait);
            let occupied: Vec<_> = world
                .npcs()
                .filter(|npc| npc.alive())
                .map(|npc| npc.position)
                .collect();
            let unique: BTreeSet<_> = occupied.iter().copied().collect();
            assert_eq!(unique.len(), occupied.len());
            assert!(!unique.contains(&world.player().position));
        }
        let save = world.checkpoint();
        assert_eq!(u16::from_le_bytes([save[12], save[13]]), 8);
        let mut restored = World::restore(&save).unwrap();
        assert_eq!(restored, world);
        assert_eq!(restored.checkpoint(), save);
        for _ in 0..12 {
            assert_eq!(
                turn(&mut world, Intent::Wait),
                turn(&mut restored, Intent::Wait)
            );
            assert_eq!(world.state_digest(), restored.state_digest());
        }
    }
}

#[test]
fn seeded_player_exchange_survives_replay_with_its_witnessed_history() {
    let mut world = World::traffic_courtesy(0);
    for direction in [
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::South,
    ] {
        assert_eq!(turn(&mut world, Intent::Move(direction)), Outcome::Moved);
    }
    assert_eq!(world.player().position, cell(4, 5));
    assert_eq!(world.traffic().unwrap().swaps, 1);
    assert_eq!(world.npc(WATCH).unwrap().position, cell(4, 4));
    assert!(
        world
            .traffic()
            .unwrap()
            .history
            .iter()
            .any(|event| { event.kind == TrafficEventKind::PlayerPass && event.witnessed })
    );
    let checkpoint = world.checkpoint();
    assert_eq!(World::restore(&checkpoint).unwrap(), world);
}

// This exact test driver is appended to the isolated C3.6 baseline's
// living/tests.rs for provenance. It also drives the C3.7 acceptance matrix.
#[test]
fn c37_golden_content_matrix() {
    use sha2::{Digest, Sha256};
    let rules = [
        Rules::Original,
        Rules::Qinghe,
        Rules::Riverlands,
        Rules::ReedHall,
        Rules::RoadCompany,
        Rules::SectRivalry,
        Rules::SectPolitics,
        Rules::TrafficCourtesy,
        Rules::LivingAgency,
        Rules::FactionAgency,
        Rules::InstitutionalAgency,
        Rules::ContractAccountability,
        Rules::InformationDisputes,
        Rules::PlayerAgency,
        Rules::Consequences,
        Rules::ViolenceLogistics,
        Rules::PlayerReporting,
        Rules::PhysicalRecovery,
        Rules::ContestedResources,
        Rules::SectCultivation,
        Rules::MartialMentorship,
        Rules::MeridianCultivation,
        Rules::PersistentInjury,
        Rules::AutonomousAgency,
        Rules::SectHierarchy,
        Rules::DutyCoherence,
    ];
    let mut rows = Vec::new();
    for (index, rule) in rules.into_iter().enumerate() {
        let rev = index + 1;
        let seeds: &[u32] = if rev <= 2 { &[0] } else { &[0, 1, 2, 7] };
        for &seed in seeds {
            let mut world = if rev <= 2 {
                World::with_rules(&qinghe::QINGHE, rule)
            } else {
                World::seeded(seed, rule)
            };
            assert_eq!(usize::from(world.checkpoint_revision()), rev);
            let mut checkpoint_rows = Vec::new();
            for tick in 0..=1000 {
                if matches!(tick, 0 | 40 | 1000) {
                    let bytes = world.checkpoint();
                    let hash = Sha256::digest(&bytes)
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    assert_eq!(World::restore(&bytes).unwrap().checkpoint(), bytes);
                    checkpoint_rows.push((tick, hash));
                }
                if tick == 1000 {
                    break;
                }
                let intent = match tick % 19 {
                    0 => Intent::Move(Direction::East),
                    1 => Intent::Move(Direction::South),
                    2 => Intent::Talk(ActorId::new(1)),
                    3 => Intent::Gather,
                    4 => Intent::Move(Direction::North),
                    5 => Intent::Guard,
                    6 => Intent::Deliver,
                    7 if rev >= 2 => Intent::Practice,
                    8 if rev >= 3 => Intent::BuyHerb,
                    9 if rev >= 3 => Intent::SellHerb,
                    10 if rev >= 4 => Intent::Brew,
                    11 if rev >= 4 => Intent::JoinHall,
                    12 if rev >= 5 => Intent::Invite(ActorId::new(22)),
                    13 if rev >= 6 => Intent::OfferAccord,
                    14 if rev >= 19 => Intent::ClaimLot(0),
                    15 if rev >= 21 => Intent::Drill(ActorId::new(33)),
                    16 => Intent::Move(Direction::West),
                    _ => Intent::Wait,
                };
                world.step(world.tick(), intent).unwrap();
            }
            for (tick, hash) in checkpoint_rows {
                rows.push(format!("{rev},{seed},{tick},{hash}"));
            }
        }
    }
    let expected = include_str!("../../tests/fixtures/living/c36_registry_golden.csv");
    let mut lines = expected.lines();
    assert_eq!(lines.next(), Some("revision,seed,tick,checkpoint_sha256"));
    assert_eq!(rows.len(), 294);
    assert_eq!(rows, lines.collect::<Vec<_>>());
}
