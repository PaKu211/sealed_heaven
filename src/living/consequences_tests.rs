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
