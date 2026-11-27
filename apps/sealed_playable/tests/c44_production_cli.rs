use sealed_sim::living::{Background, Direction, Intent, Position, World, production::REVISION};
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
fn save(root: &Path) -> Vec<u8> {
    std::fs::read(root.join("SealedHeaven/checkpoint.c44")).unwrap()
}
fn dist(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn next(w: &World, target: Position) -> Option<Direction> {
    let start = w.player().position;
    let mut q = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((at, first)) = q.pop_front() {
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
            if !w.walkable(p) || w.npcs().any(|n| n.alive() && n.position == p) || !seen.insert(p) {
                continue;
            }
            q.push_back((p, first.or(Some(d))));
        }
    }
    None
}
fn trace() -> (World, Vec<&'static str>) {
    let mut w = World::settlement_production_campaign(7);
    let mut commands = vec!["background herbalist"];
    w.step(0, Intent::ChooseBackground(Background::Herbalist))
        .unwrap();
    let target = w.scenario().patch;
    for _ in 0..200 {
        if dist(w.player().position, target) <= 1 {
            assert_eq!(w.local_production().len(), 1);
            w.step(w.tick(), Intent::Gather).unwrap();
            commands.push("gather");
            return (w, commands);
        }
        let (intent, cmd) = match next(&w, target) {
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
        w.step(w.tick(), intent).unwrap();
        commands.push(cmd);
    }
    panic!("herb patch not reached");
}
#[test]
fn production_inspection_is_local_bilingual_and_read_only() {
    let root = tempfile::tempdir().unwrap();
    let (w, commands) = trace();
    let view = w.local_production()[0];
    for lang in ["en", "id"] {
        let dir = root.path().join(lang);
        let p = run(
            &dir,
            &["--c44", "--seed", "7"],
            &format!(
                "lang {lang};production;productionlog;{};production;help;save;reset;load;production;save;quit",
                commands.join(";")
            ),
        );
        assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
        let out = String::from_utf8(p.stdout).unwrap();
        let site = if lang == "en" {
            "Qinghe herb patch"
        } else {
            "Ladang herba Qinghe"
        };
        let report = format!("{site}: {}/{}", view.stock, view.capacity);
        assert!(out.contains(&report), "{out}");
        assert!(out.contains(if lang == "en" {
            "Growth check in"
        } else {
            "Pemeriksaan tumbuh dalam"
        }));
        assert!(!out.contains("PRODUCTION tick=") && !out.contains("OrderDispatch due="));
        assert!(!out.contains("Pineford grove:") && !out.contains("Rumpun Pineford:"));
        assert_eq!(save(&dir), w.checkpoint());
    }
    let p = run(
        &root.path().join("dev"),
        &["--c44", "--seed", "7", "--debug-world"],
        &format!("{};production;productionlog;save;quit", commands.join(";")),
    );
    assert!(p.status.success() && p.stderr.is_empty());
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(
        out.contains("PRODUCTION tick=")
            && out.contains("OrderDispatch due=")
            && out.contains("GroveGrowth due=")
    );
    assert_eq!(save(&root.path().join("dev")), w.checkpoint());
}
#[test]
fn save_revision_and_strict_launch_are_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::exact_lod_campaign(7).checkpoint();
    let path = dir.join("checkpoint.c43");
    std::fs::write(&path, &old).unwrap();
    let p = run(root.path(), &["--c44"], "seed 42;wait;production;save;quit");
    assert!(p.status.success() && p.stderr.is_empty());
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.checkpoint_revision(), REVISION);
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c44"), &old).unwrap();
    let p = run(root.path(), &["--c44"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    for args in [
        vec!["--c44", "--c43"],
        vec!["--c44", "--c44"],
        vec!["--c44", "--seed"],
        vec!["--c44", "--seed", "4294967296"],
        vec!["--c44", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &args, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{args:?}");
        assert!(!isolated.exists());
    }
}
