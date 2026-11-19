use sealed_playable::agency::{Action, Session};
use sealed_playable::ascii;
use sealed_playable::locale::{Catalog, Locale};
use sealed_playable::supply::{Phase, Supply};
use sealed_sim::player5::{Intent, Outcome, Rejection};

#[test]
fn deliberation_does_not_tick_and_attack_guard_and_wait_do() {
    let mut session = Session::conflict();
    let start = session.world().tick();
    ascii::apply(&mut session, "t");
    ascii::apply(&mut session, "l");
    assert_eq!(session.world().tick(), start);
    assert_eq!(session.locale, Locale::Indonesian);
    ascii::apply(&mut session, "f");
    assert_eq!(session.world().tick(), start + 1);
    assert_eq!(
        session.recent.back().unwrap().outcome,
        Outcome::Rejected(Rejection::OutOfRange)
    );
    let before = session.world().checkpoint();
    ascii::apply(&mut session, "n");
    assert_eq!(session.world().tick(), start + 2);
    assert_ne!(session.world().checkpoint(), before);
    assert_eq!(session.recent.back().unwrap().outcome, Outcome::Waited);
    ascii::apply(&mut session, "b");
    assert_eq!(session.recent.back().unwrap().outcome, Outcome::Guarded);
    let tick = session.world().tick();
    let checkpoint = session.world().checkpoint();
    let dir = std::env::temp_dir().join("sealed-c08-conflict");
    let _ = std::fs::create_dir_all(&dir);
    session.save_path = dir.join("village.c05");
    session.act(Action::Save);
    session.act(Action::Load);
    assert_eq!(session.world().tick(), tick);
    assert_eq!(session.world().checkpoint(), checkpoint);
    session.locale = Locale::Indonesian;
    let frame = ascii::frame(&session, &Catalog::agency().unwrap());
    assert!(frame.contains('!'));
    assert!(frame.contains("Musuh"));
    assert!(frame.contains("Nyawa"));
}

#[test]
fn scripted_and_direct_commands_match_and_supply_still_completes() {
    let mut typed = Session::conflict();
    let mut direct = Session::conflict();
    for word in ["east", "east", "f", "b", "west"] {
        ascii::apply(&mut typed, word);
        let intent = match word {
            "east" => Intent::Move(sealed_sim::player5::Direction::East),
            "west" => Intent::Move(sealed_sim::player5::Direction::West),
            "f" => Intent::Attack,
            "b" => Intent::Guard,
            _ => unreachable!(),
        };
        direct.act(Action::Intent(intent));
        direct.act(Action::Step);
    }
    assert_eq!(typed.world().checkpoint(), direct.world().checkpoint());
    let mut supply = Supply::shrine();
    let status = supply.run_until_met().unwrap();
    assert_eq!(status.shrine, 5);
    assert_eq!(status.carrier, 7);
    assert_eq!(status.total, 12);
    assert_eq!(status.phase, Phase::Completed);
    for _ in 0..40 {
        ascii::apply(&mut typed, "n");
    }
    let live = typed.supply().status().unwrap();
    assert_eq!(live.shrine, 5);
    assert_eq!(live.carrier, 7);
    assert_eq!(live.total, 12);
    assert_eq!(live.phase, Phase::Completed);
}
