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
