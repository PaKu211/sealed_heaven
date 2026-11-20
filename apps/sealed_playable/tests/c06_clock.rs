use sealed_playable::clock::{self, PlannedBlock};
use sealed_playable::locale::{Catalog, Locale};
use sealed_playable::view::View;
use sealed_sim::player5::World;

#[test]
fn reading_the_clock_and_the_view_does_not_change_the_checkpoint() {
    let world = World::settlement();
    let hash = world.hash();
    let _ = clock::minute_of_day(world.tick());
    let _ = View::from_sim(world.npcs()).unwrap();
    assert_eq!(world.hash(), hash);
}

#[test]
fn the_calendar_is_not_the_kernel_goal() {
    let mut world = World::settlement();
    world.drive(0, &[]).unwrap();
    assert_eq!(world.tick(), 1);
    assert_eq!(clock::minute_of_day(world.tick()), 0);
    assert_eq!(clock::planned_block(0), PlannedBlock::Sleep);
    let view = View::from_sim(world.npcs()).unwrap();
    assert!(view.tasks.iter().any(|task| task.goal != 0));
    let again = World::settlement();
    let mut replay = World::restore(&again.checkpoint()).unwrap();
    replay.drive(0, &[]).unwrap();
    assert_eq!(replay.hash(), world.hash());
}

#[test]
fn c06_copy_exists_in_both_languages() {
    let catalog = Catalog::agency().unwrap();
    for key in [
        "c06.title",
        "c06.you",
        "c06.clock",
        "c06.schedule_note",
        "c06.actual",
        "c06.goal.1",
        "c06.block.sleep",
        "c06.yard",
        "c06.lane",
    ] {
        let english = catalog.text(Locale::English, key);
        let indonesian = catalog.text(Locale::Indonesian, key);
        assert_ne!(english, key);
        assert_ne!(indonesian, key);
        assert_ne!(english, indonesian);
    }
}
