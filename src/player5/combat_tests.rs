use super::*;

fn command(world: &World, intent: Intent) -> Command {
    Command {
        key: Key {
            target_tick: world.tick() + 1,
            actor_id: PLAYER_ID,
            producer_id: 1,
            sequence: world.next_sequence(),
        },
        intent,
    }
}

fn act(world: &mut World, intent: Intent) -> Outcome {
    let command = command(world, intent);
    let events = world.drive(world.tick(), &[command]).unwrap();
    assert_eq!(events.len(), 1);
    events[0].outcome
}

fn beside(a: Position, b: Position) -> bool {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y) == 1
}

fn approach(world: &mut World) {
    for _ in 0..40 {
        if beside(world.player_position(), world.combat().unwrap().foe) {
            return;
        }
        act(world, Intent::Move(Direction::East));
    }
    panic!("the foe never became adjacent");
}

fn deaths(world: &World, target: u32) -> usize {
    world
        .combat()
        .unwrap()
        .log
        .iter()
        .filter(|event| event.target == target && event.kind == CombatKind::Death)
        .count()
}

#[test]
fn the_same_trace_has_one_hash_and_survives_reload() {
    let mut left = World::conflict();
    let mut right = World::conflict();
    for intent in [Intent::Wait, Intent::Move(Direction::East), Intent::Guard] {
        assert_eq!(act(&mut left, intent), act(&mut right, intent));
    }
    assert_eq!(left.hash(), right.hash());
    let saved = left.checkpoint();
    let tick = left.tick();
    let restored = World::restore(&saved).unwrap();
    assert_eq!(restored.tick(), tick);
    assert_eq!(restored.checkpoint(), saved);
    let mut continued = restored;
    let mut live = left;
    assert_eq!(
        act(&mut continued, Intent::Move(Direction::East)),
        act(&mut live, Intent::Move(Direction::East))
    );
    assert_eq!(continued.checkpoint(), live.checkpoint());
}

#[test]
fn attack_and_guard_use_the_published_integers() {
    let mut world = World::conflict();
    assert_eq!(
        act(&mut world, Intent::Attack),
        Outcome::Rejected(Rejection::OutOfRange)
    );
    approach(&mut world);
    let before = world.combat().unwrap().player_hp;
    assert_eq!(act(&mut world, Intent::Guard), Outcome::Guarded);
    let guarded = world.combat().unwrap();
    assert_eq!(guarded.player_hp, before - (STRIKE - GUARD_BLOCK));
    assert!(!guarded.guard);
    assert_eq!(act(&mut world, Intent::Attack), Outcome::Struck);
    assert_eq!(world.combat().unwrap().foe_hp, FOE_HP - STRIKE);
    assert_eq!(act(&mut world, Intent::Attack), Outcome::Struck);
    assert!(!world.combat().unwrap().foe_alive);
    assert_eq!(deaths(&world, FOE_ID), 1);
    let herbs = world.player_herbs() + world.source_herbs() + world.delivered_herbs();
    assert_eq!(herbs, 3);
    act(&mut world, Intent::Wait);
    assert_eq!(deaths(&world, FOE_ID), 1);
    assert_eq!(
        act(&mut world, Intent::Attack),
        Outcome::Rejected(Rejection::UnknownTarget)
    );
}

#[test]
fn a_dead_player_cannot_act_again_and_dies_once() {
    let mut world = World::conflict();
    approach(&mut world);
    for _ in 0..8 {
        if !world.combat().unwrap().player_alive {
            break;
        }
        act(&mut world, Intent::Wait);
    }
    assert_eq!(world.combat().unwrap().player_hp, 0);
    assert_eq!(deaths(&world, PLAYER_ID), 1);
    assert_eq!(
        act(&mut world, Intent::Move(Direction::West)),
        Outcome::Rejected(Rejection::Incapacitated)
    );
    assert_eq!(deaths(&world, PLAYER_ID), 1);
    assert_eq!(
        world.player_herbs() + world.source_herbs() + world.delivered_herbs(),
        3
    );
}

#[test]
fn settlement_checkpoints_remain_revision_one() {
    let bytes = World::settlement().checkpoint();
    assert_eq!(u16::from_le_bytes(bytes[12..14].try_into().unwrap()), 1);
    let conflict = World::conflict().checkpoint();
    assert_eq!(u16::from_le_bytes(conflict[12..14].try_into().unwrap()), 2);
    assert!(World::restore(&bytes).unwrap().combat().is_none());
}

#[test]
fn settlement_rejects_revision_two_intents_without_recording_or_advancing_time() {
    let mut world = World::settlement();
    let original = world.checkpoint();
    let original_hash = world.hash();
    let original_event_digest = world.event_digest();
    let original_sequence = world.next_sequence();
    for intent in [Intent::Attack, Intent::Guard, Intent::Wait] {
        let command = command(&world, intent);
        assert_eq!(
            world.drive(world.tick(), &[command]),
            Err(Error::UnsupportedIntent(intent))
        );
        assert_eq!(world.tick(), 0);
        assert_eq!(world.next_sequence(), original_sequence);
        assert_eq!(world.event_digest(), original_event_digest);
        assert_eq!(world.checkpoint(), original);
        assert_eq!(world.hash(), original_hash);
    }
    // Rejecting one of the new intents must not poison the command highwater.
    assert_eq!(
        act(&mut world, Intent::Move(Direction::East)),
        Outcome::Moved
    );
    assert_eq!(
        u16::from_le_bytes(world.checkpoint()[12..14].try_into().unwrap()),
        1
    );
    assert_eq!(World::restore(&world.checkpoint()).unwrap(), world);
}

#[test]
fn revision_one_reader_rejects_resigned_new_intent_tag() {
    let mut world = World::settlement();
    assert_eq!(
        act(&mut world, Intent::Move(Direction::East)),
        Outcome::Moved
    );
    let mut bytes = world.checkpoint();
    // First record follows the fixed rev1 header and one 24-byte Key:
    // 12 magic + 2 revision + 32 content + 8 tick + 4 id + 8 position
    // + 3 * 13 herb holdings + 2 flags + 2 * 8 cursors + 32 digest
    // + 4 registry length + 24 key = 183.
    const FIRST_INTENT: usize = 183;
    assert_eq!(bytes[FIRST_INTENT], 1);
    bytes[FIRST_INTENT] = 7; // Wait exists only in revision 2.
    bytes[FIRST_INTENT + 1..FIRST_INTENT + 5].copy_from_slice(&0u32.to_le_bytes());
    let checksum_at = bytes.len() - 32;
    let checksum = sha256(&bytes[..checksum_at]);
    bytes[checksum_at..].copy_from_slice(&checksum);
    assert_eq!(
        World::restore(&bytes),
        Err(Error::Restore(RestoreError::InvalidState))
    );
}

#[test]
fn authentic_c07_revision_one_checkpoint_roundtrips_without_byte_changes() {
    let bytes = include_bytes!("../../docs/evidence/c07-native/SealedHeaven/checkpoint.c05");
    assert_eq!(u16::from_le_bytes(bytes[12..14].try_into().unwrap()), 1);
    let restored = World::restore(bytes).unwrap();
    assert_eq!(restored.checkpoint(), bytes);
}
