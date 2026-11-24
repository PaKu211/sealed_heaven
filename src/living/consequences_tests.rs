use super::*;

const CARRIER: ActorId = ActorId::new(3);
const WATCH: ActorId = ActorId::new(2);
const MENTOR: ActorId = ActorId::new(33);

fn wait(world: &mut World) {
    world.step(world.tick(), Intent::Wait).unwrap();
}

fn corpse_with_herbs(world: &mut World, herbs: u32) {
    let body = world.npcs.get_mut(&CARRIER).unwrap();
    body.hp = 0;
    body.herbs = herbs;
    world.player.position = body.position;
}

fn herb_total(world: &World) -> u32 {
    world.player.herbs
        + world.supply.source
        + world.supply.shrine
        + world.npcs.values().map(|npc| npc.herbs).sum::<u32>()
}

fn regional_material_total(world: &World) -> u32 {
    let region = world.region().unwrap();
    herb_total(world)
        + world.supply().consumed
        + region.market_herbs
        + region.garden_herbs
        + region.hall_offerings
        + region.remedies * 2
        + region.remedies_used * 2
        + world.rivalry().unwrap().consumed_herbs
        + world.company().unwrap().herbs_used
        + world
            .npcs()
            .map(|npc| u32::from(npc.packed_herbs))
            .sum::<u32>()
}

#[test]
fn unseen_theft_moves_herbs_without_telling_the_distant_watch() {
    let mut world = World::consequences(0);
    corpse_with_herbs(&mut world, 4);
    world.npcs.get_mut(&MENTOR).unwrap().position = Position { x: 29, y: 8 };
    world.npcs.get_mut(&WATCH).unwrap().position = Position { x: 28, y: 8 };
    let mut withheld = world.clone();
    assert_eq!(
        world.step(world.tick(), Intent::Take(CARRIER)).unwrap(),
        Outcome::Took {
            from: CARRIER,
            herbs: 4
        }
    );
    wait(&mut withheld);
    assert_eq!(world.player.herbs, withheld.player.herbs + 4);
    assert_eq!(world.npcs[&CARRIER].herbs, 0);
    assert_eq!(withheld.npcs[&CARRIER].herbs, 4);
    assert_eq!(herb_total(&world), herb_total(&withheld));
    let incident = world.social.incidents().last().unwrap();
    assert_eq!(incident.source, PLAYER);
    assert_eq!(incident.damage, 0);
    assert!(world.social.knowledge(PLAYER, incident.id).is_some());
    assert!(world.social.knowledge(MENTOR, incident.id).is_none());
    assert!(world.social.knowledge(WATCH, incident.id).is_none());
}

#[test]
fn a_witness_learns_the_thief_and_loses_regard() {
    let mut world = World::consequences(1);
    corpse_with_herbs(&mut world, 2);
    let site = world.npcs[&CARRIER].position;
    world.npcs.get_mut(&MENTOR).unwrap().position = site;
    world.step(world.tick(), Intent::Take(CARRIER)).unwrap();
    let incident = world.social.incidents().last().unwrap();
    let known = world.social.knowledge(MENTOR, incident.id).unwrap();
    assert_eq!(known.source, KnowledgeSource::Observed);
    assert!(known.regard_change < 0);
    assert!(world.social.relationship(MENTOR, PLAYER).regard < 0);
}

#[test]
fn a_living_carrier_cannot_be_looted_and_keeps_the_cargo() {
    let mut world = World::consequences(0);
    world.npcs.get_mut(&CARRIER).unwrap().herbs = 3;
    world.player.position = world.npcs[&CARRIER].position;
    let mut withheld = world.clone();
    assert_eq!(
        world.step(world.tick(), Intent::Take(CARRIER)).unwrap(),
        Outcome::Rejected(Rejection::Refused)
    );
    wait(&mut withheld);
    assert_eq!(world.npcs[&CARRIER].herbs, withheld.npcs[&CARRIER].herbs);
    assert_eq!(world.player.herbs, withheld.player.herbs);
    assert_eq!(herb_total(&world), herb_total(&withheld));
    assert!(world.social.incidents().is_empty());
}

#[test]
fn revision_fourteen_refuses_theft_and_still_calls_an_empty_tile_empty() {
    let mut older = World::player_agency(0);
    corpse_with_herbs(&mut older, 3);
    assert_eq!(
        older.step(older.tick(), Intent::Take(CARRIER)).unwrap(),
        Outcome::Rejected(Rejection::Refused)
    );
    assert_eq!(older.npcs[&CARRIER].herbs, 3);
    assert_eq!(
        u16::from_le_bytes(older.checkpoint()[12..14].try_into().unwrap()),
        14
    );
    let mut empty = World::player_agency(0);
    assert_eq!(
        empty.step(empty.tick(), Intent::Examine).unwrap(),
        Outcome::Rejected(Rejection::Empty)
    );
    let mut current = World::consequences(0);
    assert_eq!(
        current.step(current.tick(), Intent::Examine).unwrap(),
        Outcome::Rejected(Rejection::NoEvidence)
    );
    assert_eq!(
        u16::from_le_bytes(current.checkpoint()[12..14].try_into().unwrap()),
        15
    );
}

#[test]
fn a_second_take_finds_nothing_and_a_dead_player_cannot_steal() {
    let mut world = World::consequences(0);
    corpse_with_herbs(&mut world, 1);
    world.step(world.tick(), Intent::Take(CARRIER)).unwrap();
    assert_eq!(
        world.step(world.tick(), Intent::Take(CARRIER)).unwrap(),
        Outcome::Rejected(Rejection::Empty)
    );
    assert_eq!(world.player.herbs, 1);
    world.player.hp = 0;
    world.npcs.get_mut(&CARRIER).unwrap().herbs = 2;
    assert_eq!(
        world.step(world.tick(), Intent::Take(CARRIER)).unwrap(),
        Outcome::Rejected(Rejection::Dead)
    );
    assert_eq!(world.npcs[&CARRIER].herbs, 2);
}

#[test]
fn matching_thefts_agree_and_a_clean_campaign_replays() {
    let mut world = World::consequences(0);
    corpse_with_herbs(&mut world, 1);
    let mut twin = world.clone();
    world.step(world.tick(), Intent::Take(CARRIER)).unwrap();
    twin.step(twin.tick(), Intent::Take(CARRIER)).unwrap();
    assert_eq!(world.checkpoint(), twin.checkpoint());
    assert_eq!(world, twin);

    for seed in [0, 1] {
        let mut clean = World::consequences(seed);
        for _ in 0..40 {
            wait(&mut clean);
        }
        clean.step(clean.tick(), Intent::Take(CARRIER)).unwrap();
        assert_eq!(World::restore(&clean.checkpoint()).unwrap(), clean);
        assert!(clean.is_consequences());
        assert!(clean.information().is_some());
    }
}

#[test]
fn public_road_ambush_delays_freight_and_causes_a_local_contract_and_physical_news() {
    let mut world = World::violence_logistics(0);
    let mut peaceful = world.clone();
    for _ in 0..14 {
        assert_eq!(
            world
                .step(world.tick(), Intent::Move(Direction::East))
                .unwrap(),
            Outcome::Moved
        );
        peaceful
            .step(peaceful.tick(), Intent::Move(Direction::East))
            .unwrap();
    }
    let courier = ActorId::new(32);
    let watch = ActorId::new(2);
    assert_eq!(world.npc(courier).unwrap().herbs, 1);
    for action in [
        Intent::Wait,
        Intent::Attack(courier),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::North),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::South),
        Intent::Attack(courier),
        Intent::Attack(courier),
    ] {
        assert!(!matches!(
            world.step(world.tick(), action).unwrap(),
            Outcome::Rejected(_)
        ));
        wait(&mut peaceful);
    }
    assert_eq!(world.tick(), 26);
    assert!(!world.npc(courier).unwrap().alive());
    assert_eq!(world.npc(courier).unwrap().herbs, 1);
    assert_eq!(world.player().hp, 2);
    assert_eq!(world.social().incidents().last().unwrap().id, IncidentId(4));
    assert_eq!(world.social().knowledge(watch, IncidentId(4)), None);
    assert_eq!(herb_total(&world), herb_total(&peaceful));
    let saved = world.checkpoint();
    assert_eq!(u16::from_le_bytes(saved[12..14].try_into().unwrap()), 16);
    let mut resumed = World::restore(&saved).unwrap();
    assert_eq!(resumed, world);
    for _ in 26..180 {
        wait(&mut world);
        wait(&mut resumed);
        wait(&mut peaceful);
        assert_eq!(resumed.state_digest(), world.state_digest());
        if matches!(world.tick(), 43 | 68 | 84) {
            assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        }
    }
    assert_eq!(world.region().unwrap().shipped_to_qinghe, 0);
    assert!(peaceful.region().unwrap().shipped_to_qinghe > 0);
    assert!(
        world
            .contracts()
            .unwrap()
            .history
            .iter()
            .any(|event| event.need == Need::ShrineMedicine
                && event.kind == ContractEventKind::Observed)
    );
    assert!(
        !peaceful
            .contracts()
            .unwrap()
            .history
            .iter()
            .any(|event| event.need == Need::ShrineMedicine
                && event.kind == ContractEventKind::Observed)
    );
    assert_eq!(world.social().knowledge(watch, IncidentId(4)), None);
    let watch_discovery = world
        .social()
        .discoveries()
        .find(|event| event.observer == watch && event.body == courier)
        .unwrap();
    assert_eq!(watch_discovery.source, KnowledgeSource::Observed);
    assert_eq!(watch_discovery.nearby, None);
    let steward_discovery = world
        .social()
        .discoveries()
        .find(|event| event.observer == ActorId::new(36) && event.body == courier)
        .unwrap();
    assert!(
        matches!(steward_discovery.source, KnowledgeSource::Reported { sender, hops: 1, .. } if sender == watch)
    );
    let mentor_discovery = world
        .social()
        .discoveries()
        .find(|event| event.observer == ActorId::new(33) && event.body == courier)
        .unwrap();
    assert!(matches!(
        mentor_discovery.source,
        KnowledgeSource::Reported { hops: 2, .. }
    ));
    assert_eq!(
        world.social().knowledge(ActorId::new(33), IncidentId(4)),
        None
    );
    assert!(
        world
            .contracts()
            .unwrap()
            .history
            .iter()
            .any(|event| event.need == Need::ShrineMedicine
                && event.kind == ContractEventKind::Delivered)
    );
    assert!(
        world
            .contract_memory()
            .unwrap()
            .history
            .iter()
            .any(|audit| audit.need == Need::ShrineMedicine
                && audit.finding == AuditFinding::Delivered)
    );
}

#[test]
fn old_c24_replays_without_c25_body_reports() {
    let mut old = World::consequences(0);
    for _ in 0..100 {
        wait(&mut old);
    }
    let checkpoint = old.checkpoint();
    assert_eq!(
        u16::from_le_bytes(checkpoint[12..14].try_into().unwrap()),
        15
    );
    assert_eq!(World::restore(&checkpoint).unwrap(), old);
    assert_eq!(old.social().discoveries().count(), 0);
}

#[test]
fn public_witnessed_violence_is_not_a_corpse_discovery_or_remote_guilt() {
    let mut world = World::violence_logistics(0);
    for _ in 0..8 {
        assert_eq!(
            world
                .step(world.tick(), Intent::Move(Direction::East))
                .unwrap(),
            Outcome::Moved
        );
    }
    let traveler = ActorId::new(22);
    for _ in 0..2 {
        assert_eq!(
            world.step(world.tick(), Intent::Attack(traveler)).unwrap(),
            Outcome::Struck(traveler)
        );
    }
    let incident = world
        .social()
        .incidents()
        .iter()
        .find(|event| event.source == PLAYER && event.target == traveler)
        .unwrap()
        .id;
    assert_eq!(
        world.social().knowledge(CARRIER, incident).unwrap().source,
        KnowledgeSource::Observed
    );
    assert_eq!(world.social().knowledge(ActorId::new(1), incident), None);
    for _ in 0..22 {
        wait(&mut world);
    }
    assert!(matches!(
        world
            .social()
            .knowledge(ActorId::new(1), incident)
            .unwrap()
            .source,
        KnowledgeSource::Reported { hops: 1.., .. }
    ));
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

#[test]
fn finding_a_body_near_the_player_does_not_invent_an_eyewitness() {
    let mut world = World::violence_logistics(0);
    let body = ActorId::new(32);
    let witness = ActorId::new(33);
    // This test isolates the classification boundary; the road-ambush test
    // above proves that actual corpses are reachable via public commands.
    let site = world.npc(body).unwrap().position;
    world.npcs.get_mut(&body).unwrap().hp = 0;
    world.npcs.get_mut(&witness).unwrap().position = site;
    world.player.position = site;
    wait(&mut world);
    let discovery = world
        .social()
        .discoveries()
        .find(|discovery| discovery.observer == witness && discovery.body == body)
        .unwrap();
    assert_eq!(discovery.nearby, Some(PLAYER));
    assert_eq!(discovery.source, KnowledgeSource::Observed);
    assert!(world.social().incidents().is_empty());
    assert_eq!(world.social().relationship(witness, PLAYER).regard, 0);
}

#[test]
fn public_road_witness_reports_corpse_without_reporting_a_murderer() {
    let mut world = World::player_reporting(0);
    let courier = ActorId::new(32);
    for _ in 0..14 {
        world
            .step(world.tick(), Intent::Move(Direction::East))
            .unwrap();
    }
    for action in [
        Intent::Wait,
        Intent::Attack(courier),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::North),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::South),
        Intent::Attack(courier),
        Intent::Attack(courier),
    ] {
        world.step(world.tick(), action).unwrap();
    }
    assert_eq!(
        world
            .step(world.tick(), Intent::Move(Direction::South))
            .unwrap(),
        Outcome::Moved
    );
    for _ in 0..8 {
        assert_eq!(
            world
                .step(world.tick(), Intent::Move(Direction::West))
                .unwrap(),
            Outcome::Moved
        );
    }
    for _ in 0..3 {
        wait(&mut world);
    }
    let watch = ActorId::new(2);
    assert_eq!(world.tick(), 38);
    let mut withheld = world.clone();
    assert_eq!(
        world
            .step(
                world.tick(),
                Intent::ReportBody {
                    to: watch,
                    body: courier
                }
            )
            .unwrap(),
        Outcome::BodyReported {
            to: watch,
            body: courier
        }
    );
    wait(&mut withheld);
    assert_eq!(
        world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == courier),
        None
    );
    assert_eq!(world.social().pending_body_reports().count(), 1);
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
    wait(&mut world);
    wait(&mut withheld);
    assert_eq!(
        world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == courier),
        None
    );
    wait(&mut world);
    wait(&mut withheld);
    let report = world
        .social()
        .discoveries()
        .find(|r| r.observer == watch && r.body == courier)
        .unwrap();
    assert_eq!(report.at, 41);
    assert_eq!(report.nearby, None);
    assert_eq!(
        report.source,
        KnowledgeSource::Reported {
            sender: PLAYER,
            witness: PLAYER,
            hops: 1
        }
    );
    assert_eq!(
        withheld
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == courier),
        None
    );
    assert_eq!(
        world.social().relationship(watch, PLAYER).regard,
        withheld.social().relationship(watch, PLAYER).regard
    );
    assert_eq!(world.social().knowledge(watch, IncidentId(4)), None);
    wait(&mut world);
    assert_eq!(
        world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == courier)
            .unwrap()
            .source,
        KnowledgeSource::Observed
    );
    assert_eq!(
        world
            .social()
            .discoveries()
            .find(|r| r.observer == watch && r.body == courier)
            .unwrap()
            .prior_report,
        Some((
            41,
            KnowledgeSource::Reported {
                sender: PLAYER,
                witness: PLAYER,
                hops: 1
            }
        ))
    );
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

#[test]
fn unfounded_duplicate_and_unencodable_reports_never_become_testimony() {
    let watch = ActorId::new(2);
    let body = ActorId::new(32);
    let mut world = World::player_reporting(1);
    world.player.position = world.npc(watch).unwrap().position;
    assert_eq!(
        world
            .step(world.tick(), Intent::ReportBody { to: watch, body })
            .unwrap(),
        Outcome::Rejected(Rejection::NoEvidence)
    );
    assert_eq!(world.social().pending_body_reports().count(), 0);
    let before = world.checkpoint();
    assert_eq!(
        world.step(
            world.tick(),
            Intent::ReportBody {
                to: ActorId::new(256),
                body
            }
        ),
        Err(StepError::InvalidBodyReport)
    );
    assert_eq!(world.checkpoint(), before);
    let mut legacy = World::violence_logistics(1);
    let before = legacy.checkpoint();
    assert_eq!(
        legacy.step(legacy.tick(), Intent::ReportBody { to: watch, body }),
        Err(StepError::InvalidBodyReport)
    );
    assert_eq!(legacy.checkpoint(), before);
    assert_eq!(World::restore(&before).unwrap(), legacy);
}

#[test]
fn reported_body_redirects_a_free_watch_but_not_an_exhausted_one() {
    let mut world = World::player_reporting(0);
    let body = ActorId::new(32);
    let watch = ActorId::new(2);
    // Isolate institutional priorities: a watch without an active mission
    // receives a true report from a player who personally saw a remote body.
    let site = Position { x: 18, y: 3 };
    world.npcs.get_mut(&body).unwrap().position = site;
    world.npcs.get_mut(&body).unwrap().hp = 0;
    world.player.position = site;
    wait(&mut world);
    assert!(
        world
            .social()
            .discoveries()
            .any(|r| r.observer == PLAYER && r.body == body)
    );
    world.player.position = world.npc(watch).unwrap().position;
    world.supply.shrine = world.scenario.shrine_target;
    let mut withheld = world.clone();
    assert_eq!(
        world
            .step(world.tick(), Intent::ReportBody { to: watch, body })
            .unwrap(),
        Outcome::BodyReported { to: watch, body }
    );
    for _ in 0..2 {
        wait(&mut world);
        wait(&mut withheld);
    }
    assert_eq!(
        world.choose_goal(world.npc(watch).unwrap()),
        Goal::InvestigateBody(body)
    );
    assert_ne!(
        withheld.choose_goal(withheld.npc(watch).unwrap()),
        Goal::InvestigateBody(body)
    );
    let old = world.npc(watch).unwrap().position;
    wait(&mut world);
    assert_ne!(world.npc(watch).unwrap().position, old);
    // Fixture-injected positions are deliberately not used as save evidence;
    // the public route above proves checkpoint replay.
    world.npcs.get_mut(&watch).unwrap().fatigue = 40;
    assert_ne!(
        world.choose_goal(world.npc(watch).unwrap()),
        Goal::InvestigateBody(body)
    );
}

#[test]
fn a_dead_player_cannot_deliver_an_undelivered_body_report() {
    let mut world = World::player_reporting(0);
    let body = ActorId::new(32);
    let watch = ActorId::new(2);
    // Only the initial discovery is prepared here. The full public-command
    // murder and travel route is tested separately above.
    let site = world.npc(body).unwrap().position;
    world.npcs.get_mut(&body).unwrap().hp = 0;
    world.player.position = site;
    wait(&mut world);
    world.player.position = world.npc(watch).unwrap().position;
    assert_eq!(
        world
            .step(world.tick(), Intent::ReportBody { to: watch, body })
            .unwrap(),
        Outcome::BodyReported { to: watch, body }
    );
    world.player.hp = 0;
    for _ in 0..3 {
        wait(&mut world);
    }
    assert_eq!(world.social().pending_body_reports().count(), 0);
    assert!(
        world
            .social()
            .discoveries()
            .all(|discovery| discovery.observer != watch || discovery.body != body)
    );
}

#[test]
fn public_ambush_watch_salvages_one_real_herb_and_resolves_the_extra_contract() {
    let mut world = World::physical_recovery(0);
    let body = ActorId::new(32);
    let initial_material = regional_material_total(&world);
    for _ in 0..14 {
        world
            .step(world.tick(), Intent::Move(Direction::East))
            .unwrap();
    }
    for action in [
        Intent::Wait,
        Intent::Attack(body),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::North),
        Intent::Move(Direction::North),
        Intent::Move(Direction::East),
        Intent::Move(Direction::East),
        Intent::Move(Direction::South),
        Intent::Attack(body),
        Intent::Attack(body),
    ] {
        world.step(world.tick(), action).unwrap();
    }
    assert_eq!(world.npc(body).unwrap().herbs, 1);
    let mut restored = World::restore(&world.checkpoint()).unwrap();
    let mut stolen = world.clone();
    for turn in 26..75 {
        wait(&mut world);
        wait(&mut restored);
        if turn == 26 {
            assert_eq!(
                stolen.step(stolen.tick(), Intent::Take(body)).unwrap(),
                Outcome::Took {
                    from: body,
                    herbs: 1
                }
            );
        } else {
            wait(&mut stolen);
        }
        assert_eq!(world.state_digest(), restored.state_digest());
        assert_eq!(
            regional_material_total(&world),
            initial_material + world.supply().regrown + world.region().unwrap().garden_regrown,
            "material ledger at tick {}",
            world.tick()
        );
        if matches!(world.tick(), 47 | 67) {
            assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
        }
    }
    let events = world.social().recoveries();
    assert_eq!(events.len(), 2);
    assert_eq!((events[0].id, events[1].id), (0, 1));
    assert_eq!((events[0].herbs, events[1].herbs), (1, 1));
    assert!(!events[0].delivered);
    assert!(events[1].delivered);
    assert!(events[0].witnessed);
    assert!(!events[1].witnessed);
    assert_eq!(world.npc(body).unwrap().herbs, 0);
    assert_eq!(world.npc(ActorId::new(2)).unwrap().herbs, 0);
    assert!(world.supply().shrine >= world.scenario().shrine_target);
    assert!(
        world
            .contracts()
            .unwrap()
            .history
            .iter()
            .any(|event| event.need == Need::ShrineMedicine
                && event.kind == ContractEventKind::ResolvedLocally)
    );
    assert_eq!(world.contracts().unwrap().failed, 0);
    assert_eq!(stolen.player().herbs, 1);
    assert!(stolen.social().recoveries().is_empty());
    assert_eq!(World::restore(&stolen.checkpoint()).unwrap(), stolen);
    assert!(
        stolen
            .contracts()
            .unwrap()
            .active
            .iter()
            .any(|case| case.need == Need::ShrineMedicine)
    );
    let mut legacy = World::player_reporting(0);
    for _ in 0..75 {
        wait(&mut legacy);
    }
    assert_eq!(World::restore(&legacy.checkpoint()).unwrap(), legacy);
    assert!(legacy.social().recoveries().is_empty());
}

#[test]
fn fallen_recovery_worker_keeps_the_herb_on_their_body() {
    let mut world = World::physical_recovery(0);
    let body = ActorId::new(32);
    let watch = ActorId::new(2);
    world.npcs.get_mut(&body).unwrap().hp = 0;
    world.npcs.get_mut(&body).unwrap().herbs = 1;
    world.npcs.get_mut(&watch).unwrap().position = world.npc(body).unwrap().position;
    world.social.discoveries.insert(
        (watch, body),
        BodyDiscovery {
            observer: watch,
            body,
            at: 0,
            nearby: None,
            source: KnowledgeSource::Observed,
            prior_report: None,
        },
    );
    world.recover_corpse_cargo(watch, body);
    assert_eq!(world.npc(body).unwrap().herbs, 0);
    assert_eq!(world.npc(watch).unwrap().herbs, 1);
    let total = herb_total(&world);
    world.damage(PLAYER, watch, 6, false);
    assert!(!world.npc(watch).unwrap().alive());
    assert!(world.social.recovery_cargo.is_empty());
    assert_eq!(world.npc(watch).unwrap().herbs, 1);
    assert_eq!(herb_total(&world), total);
    world.player.position = world.npc(watch).unwrap().position;
    assert_eq!(
        world.step(world.tick(), Intent::Take(watch)).unwrap(),
        Outcome::Took {
            from: watch,
            herbs: 1
        }
    );
    assert_eq!(world.npc(watch).unwrap().herbs, 0);
    assert_eq!(world.player().herbs, 1);
}
