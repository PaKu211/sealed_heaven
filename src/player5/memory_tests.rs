use super::*;

fn act(world: &mut World, intent: Intent) -> Outcome {
    let key = Key {
        target_tick: world.tick() + 1,
        actor_id: PLAYER_ID,
        producer_id: 1,
        sequence: world.next_sequence(),
    };
    let events = world
        .drive(world.tick(), &[Command { key, intent }])
        .unwrap();
    assert_eq!(events.len(), 1);
    events[0].outcome
}

fn approach_and_kill(world: &mut World) {
    for _ in 0..20 {
        let foe = world.combat().unwrap().foe;
        if adjacent(world.player_position(), foe) {
            break;
        }
        assert_eq!(act(world, Intent::Move(Direction::East)), Outcome::Moved);
    }
    assert!(adjacent(
        world.player_position(),
        world.combat().unwrap().foe
    ));
    assert_eq!(act(world, Intent::Attack), Outcome::Struck);
    assert_eq!(act(world, Intent::Attack), Outcome::Struck);
    assert!(!world.combat().unwrap().foe_alive);
}

#[test]
fn witnessed_killing_reports_after_two_more_ticks_and_only_once() {
    let mut world = World::conflict_with_memory();
    assert_eq!(
        u16::from_le_bytes(world.checkpoint()[12..14].try_into().unwrap()),
        3
    );
    assert_eq!(world.memory().unwrap().healer_standing, 0);
    approach_and_kill(&mut world);
    let after_death = world.tick();
    let memory = world.memory().unwrap();
    let incident = memory
        .incident
        .expect("the authored Qinghe witness must be nearby");
    assert_eq!(incident.tick, after_death);
    assert_eq!(incident.report_tick, after_death + 2);
    assert_eq!(incident.witness_id, memory::WITNESS_ID);
    assert!(!memory.reported);
    assert_eq!(memory.healer_standing, 0);
    assert_eq!(memory.log.len(), 1);
    assert_eq!(memory.log[0].kind, MemoryKind::WitnessedDeath);
    assert_eq!(memory.log[0].death_event_id, incident.death_event_id);

    assert_eq!(act(&mut world, Intent::Wait), Outcome::Waited);
    assert!(!world.memory().unwrap().reported);
    assert_eq!(act(&mut world, Intent::Wait), Outcome::Waited);
    let memory = world.memory().unwrap();
    assert!(memory.reported);
    assert_eq!(memory.healer_standing, -1);
    assert_eq!(memory.log.len(), 2);
    assert_eq!(memory.log[1].kind, MemoryKind::ReportDelivered);
    assert_eq!(memory.log[1].tick, incident.report_tick);
    for _ in 0..5 {
        act(&mut world, Intent::Wait);
    }
    assert_eq!(world.memory().unwrap().log.len(), 2);
    assert_eq!(world.memory().unwrap().healer_standing, -1);
}

#[test]
fn save_reload_pending_and_reported_memory_matches_uninterrupted_replay() {
    let mut live = World::conflict_with_memory();
    approach_and_kill(&mut live);
    let pending = live.checkpoint();
    let mut reloaded = World::restore(&pending).unwrap();
    assert_eq!(reloaded.checkpoint(), pending);
    assert_eq!(reloaded.memory(), live.memory());
    for _ in 0..3 {
        assert_eq!(
            act(&mut live, Intent::Wait),
            act(&mut reloaded, Intent::Wait)
        );
        assert_eq!(live.checkpoint(), reloaded.checkpoint());
    }
    let saved_after_report = live.checkpoint();
    let restored = World::restore(&saved_after_report).unwrap();
    assert_eq!(restored, live);
    assert_eq!(restored.memory().unwrap().log.len(), 2);
}

#[test]
fn retreat_or_unwitnessed_death_cannot_create_an_omniscient_report() {
    let mut world = World::conflict_with_memory();
    for _ in 0..4 {
        act(&mut world, Intent::Wait);
    }
    assert!(world.memory().unwrap().incident.is_none());
    assert_eq!(world.memory().unwrap().healer_standing, 0);
    assert!(!memory::witnessed_at(None, Position { x: 6, y: 4 }));
    assert!(!memory::witnessed_at(
        Some(Position { x: 0, y: 0 }),
        Position { x: 8, y: 4 }
    ));
    assert!(memory::witnessed_at(
        Some(Position { x: 3, y: 3 }),
        Position { x: 6, y: 4 }
    ));
}

#[test]
fn a_killing_lured_west_of_qinghe_goes_unreported_without_a_witness() {
    let mut world = World::conflict_with_memory();
    // Draw the foe west, away from NPC 2's eastern site. The same death event
    // is authoritative, but no nearby witness is eligible to report it.
    for direction in [
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::East,
        Direction::West,
        Direction::West,
        Direction::West,
        Direction::East,
    ] {
        assert_eq!(act(&mut world, Intent::Move(direction)), Outcome::Moved);
    }
    assert_eq!(act(&mut world, Intent::Attack), Outcome::Struck);
    assert_eq!(act(&mut world, Intent::Attack), Outcome::Struck);
    assert!(!world.combat().unwrap().foe_alive);
    assert!(world.memory().unwrap().incident.is_none());
    for _ in 0..3 {
        act(&mut world, Intent::Wait);
    }
    assert!(!world.memory().unwrap().reported);
    assert_eq!(world.memory().unwrap().healer_standing, 0);
    let bytes = world.checkpoint();
    assert_eq!(World::restore(&bytes).unwrap().checkpoint(), bytes);
}

#[test]
fn revision_one_two_three_remain_distinct_and_three_checks_memory_integrity() {
    for (world, revision) in [
        (World::settlement(), 1u16),
        (World::conflict(), 2),
        (World::conflict_with_memory(), 3),
    ] {
        let bytes = world.checkpoint();
        assert_eq!(
            u16::from_le_bytes(bytes[12..14].try_into().unwrap()),
            revision
        );
        assert_eq!(World::restore(&bytes).unwrap().checkpoint(), bytes);
    }
    let mut forged = World::conflict_with_memory().checkpoint();
    let checksum_at = forged.len() - 32;
    forged[checksum_at - 1] = 255; // Claim -1 standing without a report.
    let checksum = sha256(&forged[..checksum_at]);
    forged[checksum_at..].copy_from_slice(&checksum);
    assert_eq!(
        World::restore(&forged),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn resigned_incident_attribution_does_not_survive_replay_validation() {
    let mut world = World::conflict_with_memory();
    approach_and_kill(&mut world);
    let mut forged = world.checkpoint();
    let checksum_at = forged.len() - 32;
    // The revision-3 tail ends in [incident fields:28, reported:u8,
    // healer_standing:i8]. Modify the incident's originating death event ID.
    let death_id_at = checksum_at - 2 - 28;
    forged[death_id_at] ^= 1;
    let checksum = sha256(&forged[..checksum_at]);
    forged[checksum_at..].copy_from_slice(&checksum);
    assert_eq!(
        World::restore(&forged),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn report_can_arrive_on_empty_world_ticks_even_without_player_input() {
    let mut world = World::conflict_with_memory();
    approach_and_kill(&mut world);
    assert!(world.memory().unwrap().incident.is_some());
    let mut other = World::restore(&world.checkpoint()).unwrap();
    for _ in 0..2 {
        world.drive(world.tick(), &[]).unwrap();
        other.drive(other.tick(), &[]).unwrap();
    }
    assert!(world.memory().unwrap().reported);
    assert_eq!(world.hash(), other.hash());
}

#[test]
fn world_continues_after_player_death_without_creating_false_social_memory() {
    let mut world = World::conflict_with_memory();
    for _ in 0..20 {
        if adjacent(world.player_position(), world.combat().unwrap().foe) {
            break;
        }
        act(&mut world, Intent::Move(Direction::East));
    }
    for _ in 0..8 {
        if !world.combat().unwrap().player_alive {
            break;
        }
        act(&mut world, Intent::Wait);
    }
    assert!(!world.combat().unwrap().player_alive);
    assert_eq!(
        act(&mut world, Intent::Wait),
        Outcome::Rejected(Rejection::Incapacitated)
    );
    let tick = world.tick();
    world.drive(tick, &[]).unwrap();
    assert_eq!(world.tick(), tick + 1);
    assert!(world.memory().unwrap().incident.is_none());
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}
