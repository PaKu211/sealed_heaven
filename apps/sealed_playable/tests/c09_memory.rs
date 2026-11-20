use sealed_playable::agency::{Action, Session};
use sealed_playable::ascii;
use sealed_playable::locale::{Catalog, Locale};
use sealed_playable::supply::Phase;
use sealed_sim::player5::{Direction, Intent, MemoryKind, Outcome};

fn approach_and_kill(session: &mut Session) {
    for _ in 0..20 {
        let world = session.world();
        let foe = world.combat().unwrap().foe;
        let player = world.player_position();
        if player.x.abs_diff(foe.x) + player.y.abs_diff(foe.y) == 1 {
            break;
        }
        ascii::apply(session, "east");
    }
    ascii::apply(session, "f");
    ascii::apply(session, "f");
    assert!(!session.world().combat().unwrap().foe_alive);
}

#[test]
fn c09_ascii_healer_memory_is_delayed_localized_and_persistent() {
    let mut session = Session::memory();
    let dir = tempfile::tempdir().unwrap();
    session.save_path = dir.path().join("memory.c05");
    let catalog = Catalog::agency().unwrap();
    let initial = session.world().checkpoint();
    ascii::apply(&mut session, "t");
    ascii::apply(&mut session, "l");
    assert_eq!(session.world().checkpoint(), initial);
    assert_eq!(session.locale, Locale::Indonesian);
    assert!(ascii::frame(&session, &catalog).contains("Sikap tabib: Netral"));

    approach_and_kill(&mut session);
    assert!(session.world().memory().unwrap().incident.is_some());
    assert!(!session.world().memory().unwrap().reported);
    let tick = session.world().tick();
    ascii::apply(&mut session, "v");
    ascii::apply(&mut session, "o");
    assert_eq!(session.world().tick(), tick);
    assert!(!session.world().memory().unwrap().reported);
    assert!(!ascii::frame(&session, &catalog).contains("Dilaporkan saksi"));

    ascii::apply(&mut session, "n");
    assert!(!session.world().memory().unwrap().reported);
    ascii::apply(&mut session, "n");
    assert!(session.world().memory().unwrap().reported);
    assert_eq!(session.world().memory().unwrap().healer_standing, -1);
    assert_eq!(session.world().memory().unwrap().log.len(), 2);
    assert_eq!(
        session.world().memory().unwrap().log[1].kind,
        MemoryKind::ReportDelivered
    );
    let frame = ascii::frame(&session, &catalog);
    assert!(frame.contains("Sikap tabib: Waspada"));
    assert!(frame.contains("Dilaporkan saksi"));
    ascii::apply(&mut session, "l");
    assert!(ascii::frame(&session, &catalog).contains("Healer's attitude: Wary"));

    let after_report = session.world().checkpoint();
    ascii::apply(&mut session, "v");
    ascii::apply(&mut session, "o");
    assert_eq!(session.world().checkpoint(), after_report);
    assert_eq!(session.world().memory().unwrap().log.len(), 2);

    for _ in 0..40 {
        ascii::apply(&mut session, "n");
    }
    let supply = session.supply().status().unwrap();
    assert_eq!(supply.shrine, 5);
    assert_eq!(supply.carrier, 7);
    assert_eq!(supply.total, 12);
    assert_eq!(supply.phase, Phase::Completed);
    assert_eq!(session.world().memory().unwrap().log.len(), 2);
    ascii::apply(&mut session, "r");
    assert_eq!(session.world().tick(), 0);
    assert!(session.world().memory().is_some());
    assert!(!session.world().memory().unwrap().reported);
}

#[test]
fn c09_scripted_and_direct_drives_share_one_checkpoint() {
    let mut scripted = Session::memory();
    let mut direct = Session::memory();
    for token in ["east", "east", "east", "east", "east", "f", "f", "n", "n"] {
        ascii::apply(&mut scripted, token);
        let intent = match token {
            "east" => Intent::Move(Direction::East),
            "f" => Intent::Attack,
            "n" => Intent::Wait,
            _ => unreachable!(),
        };
        direct.act(Action::Intent(intent));
        direct.act(Action::Step);
    }
    assert_eq!(scripted.world().checkpoint(), direct.world().checkpoint());
    assert_eq!(
        scripted.world().memory().unwrap(),
        direct.world().memory().unwrap()
    );
    assert_eq!(scripted.recent.back().unwrap().outcome, Outcome::Waited);
}
