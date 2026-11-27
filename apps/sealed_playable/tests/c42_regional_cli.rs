use sealed_sim::living::{
    Background, Direction, Intent, Outcome, PLAYER, Position, Region, World,
    expansion::CARRIER,
    regional_news::{MAX_HOPS, STOCK_LIFETIME, Site},
};
use std::{
    collections::{BTreeSet, VecDeque},
    path::Path,
    process::{Command, Output},
};

fn run(root: &Path, flags: &[&str], commands: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sealed_agency"))
        .args(flags)
        .env("LOCALAPPDATA", root)
        .env("SEALED_ASCII_COMMANDS", commands)
        .env_remove("SEALED_LOCALE_DIR")
        .output()
        .unwrap()
}
fn output(root: &Path, flags: &[&str], commands: &str) -> String {
    let p = run(root, flags, commands);
    assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
    String::from_utf8(p.stdout).unwrap()
}
fn save(root: &Path) -> Vec<u8> {
    std::fs::read(root.join("SealedHeaven/checkpoint.c42")).unwrap()
}
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn next(w: &World, target: Position) -> Option<Direction> {
    let start = w.player().position;
    let mut queue = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((at, first)) = queue.pop_front() {
        if dist(at, target) <= 1 {
            return first;
        }
        for (d, x, y) in [
            (Direction::North, Some(at.x), at.y.checked_sub(1)),
            (Direction::East, at.x.checked_add(1), Some(at.y)),
            (Direction::South, Some(at.x), at.y.checked_add(1)),
            (Direction::West, at.x.checked_sub(1), Some(at.y)),
        ] {
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            let p = Position { x, y };
            if !w.walkable(p)
                || dist(p, Region::sites().market) <= 1
                || w.npcs().any(|n| n.alive() && n.position == p)
                || !seen.insert(p)
            {
                continue;
            }
            queue.push_back((p, first.or(Some(d))));
        }
    }
    None
}
fn trace_to_carrier() -> (World, Vec<&'static str>) {
    let mut w = World::regional_knowledge_campaign(7);
    let mut trace = vec!["background wanderer"];
    w.step(0, Intent::ChooseBackground(Background::Wanderer))
        .unwrap();
    for _ in 0..1000 {
        let carrier = w.npc(CARRIER).unwrap();
        let known = w
            .regional_news()
            .unwrap()
            .stock(CARRIER, Site::WillowfordMarket);
        if carrier.position.x >= 30
            && dist(w.player().position, carrier.position) <= 1
            && known.is_some_and(|k| {
                k.receipt.hops < MAX_HOPS && k.receipt.fresh(w.tick() + 3, STOCK_LIFETIME)
            })
        {
            assert_eq!(
                w.step(w.tick(), Intent::Talk(CARRIER)).unwrap(),
                Outcome::Spoke(CARRIER)
            );
            trace.push("talk 37");
            return (w, trace);
        }
        let target = if carrier.position.x >= 30 && known.is_some() {
            carrier.position
        } else {
            Position { x: 43, y: 7 }
        };
        let (intent, command) = match next(&w, target) {
            Some(d) => (
                Intent::Move(d),
                match d {
                    Direction::North => "north",
                    Direction::East => "east",
                    Direction::South => "south",
                    Direction::West => "west",
                },
            ),
            None => (Intent::Wait, "wait"),
        };
        trace.push(command);
        w.step(w.tick(), intent).unwrap();
    }
    panic!("carrier never contacted");
}

#[test]
fn ascii_news_provenance_delay_and_language_preserve_exact_pending_save() {
    let root = tempfile::tempdir().unwrap();
    let (pending, trace) = trace_to_carrier();
    let mut expected = pending.clone();
    expected.step(expected.tick(), Intent::Wait).unwrap();
    assert!(
        expected
            .regional_news()
            .unwrap()
            .stock(PLAYER, Site::WillowfordMarket)
            .is_none()
    );
    let one = expected.checkpoint();
    expected.step(expected.tick(), Intent::Wait).unwrap();
    let k = expected
        .regional_news()
        .unwrap()
        .stock(PLAYER, Site::WillowfordMarket)
        .unwrap();
    assert_eq!(k.receipt.via, Some(CARRIER));
    let after = expected.checkpoint();
    for lang in ["en", "id"] {
        let dir = root.path().join(lang);
        let name = if lang == "en" {
            "Willowford market:"
        } else {
            "Pasar Willowford:"
        };
        let unit = if lang == "en" {
            "herbs at t"
        } else {
            "herba pada t"
        };
        let forbidden = format!("{name} {} {unit}{}", k.amount, k.receipt.observed_at);
        let first = output(
            &dir,
            &["--c42", "--seed", "7"],
            &format!("lang {lang};{};news;newsnext;save;quit", trace.join(";")),
        );
        assert!(!first.contains(&forbidden));
        assert_eq!(save(&dir), pending.checkpoint());
        let second = output(
            &dir,
            &["--c42", "--seed", "7"],
            &format!("lang {lang};load;wait;news;save;quit"),
        );
        assert!(!second.contains(&forbidden));
        assert_eq!(save(&dir), one);
        let third = output(
            &dir,
            &["--c42", "--seed", "7"],
            &format!("lang {lang};load;wait;news;newsnext;save;quit"),
        );
        assert!(
            third.contains(&forbidden) && third.contains("via Li Wen"),
            "{third}"
        );
        assert!(third.contains(if lang == "en" {
            "recent estimate"
        } else {
            "perkiraan baru"
        }));
        assert!(!third.contains("living.regional."));
        assert_eq!(save(&dir), after);
    }
}

#[test]
fn player_news_hides_unobserved_chronicle_and_is_read_only() {
    let root = tempfile::tempdir().unwrap();
    let cmd = "wait;".repeat(150) + "news;newsnext;chronicle;save;quit";
    let normal = output(&root.path().join("normal"), &["--c42", "--seed", "7"], &cmd);
    assert!(normal.contains("No regional knowledge yet."));
    assert!(!normal.contains("developer record"));
    assert!(!normal.contains("freight loaded") && !normal.contains("freight arrived"));
    let developer = output(
        &root.path().join("dev"),
        &["--c42", "--seed", "7", "--debug-world"],
        &cmd,
    );
    assert!(developer.contains("developer record") && developer.contains("freight loaded"));
    assert_eq!(
        save(&root.path().join("normal")),
        save(&root.path().join("dev"))
    );
    let restored = World::restore(&save(&root.path().join("normal"))).unwrap();
    assert_eq!(restored.tick(), 150);
    assert!(restored.regional_news().unwrap().events().len() > 4);
}

#[test]
fn c42_isolated_slot_seed_and_strict_launch_retain_legacy_files() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::expanded_riverlands(7).checkpoint();
    std::fs::write(dir.join("checkpoint.c41"), &old).unwrap();
    output(
        root.path(),
        &["--c42"],
        "seed 42;wait;save;reset;load;news;save;quit",
    );
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.checkpoint_revision(), 29);
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    assert_eq!(std::fs::read(dir.join("checkpoint.c41")).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c42"), &old).unwrap();
    let bad = output(root.path(), &["--c42"], "load;save;quit");
    assert!(bad.contains("Load failed") && bad.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    assert_eq!(std::fs::read(dir.join("checkpoint.c41")).unwrap(), old);
    for args in [
        vec!["--c42", "--c41"],
        vec!["--c42", "--c42"],
        vec!["--c42", "--seed"],
        vec!["--c42", "--seed", "4294967296"],
        vec!["--c42", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &args, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{args:?}");
        assert!(!isolated.exists());
    }
}
