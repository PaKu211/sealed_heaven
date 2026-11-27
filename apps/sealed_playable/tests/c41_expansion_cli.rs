use sealed_sim::living::{
    Background, Direction, Intent, Outcome, Position, World,
    expansion::{self, DEPOT, GROVE},
};
use std::{
    collections::{BTreeSet, VecDeque},
    path::Path,
    process::{Command, Output},
};

fn run(root: &Path, args: &[&str], commands: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sealed_agency"))
        .args(args)
        .env("LOCALAPPDATA", root)
        .env("SEALED_ASCII_COMMANDS", commands)
        .env_remove("SEALED_LOCALE_DIR")
        .output()
        .unwrap()
}
fn save(root: &Path) -> Vec<u8> {
    std::fs::read(root.join("SealedHeaven/checkpoint.c41")).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn expansion_manifest_is_read_only_and_invalid_launches_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("untouched");
    let a = run(&root, &["--expansion-info", "42"], "wait;save;quit");
    let b = run(&root, &["--expansion-info", "42"], "quit");
    assert!(a.status.success() && a.stderr.is_empty());
    assert_eq!(a.stdout, b.stdout);
    let value: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    let world = World::expanded_riverlands(42);
    let map = world.expanded_region().unwrap();
    assert_eq!(value["width"], 54);
    assert_eq!(value["height"], 15);
    assert_eq!(value["checkpoint_revision"], 28);
    assert_eq!(value["overlay_sha256"], expansion::CONTENT_SHA256);
    assert_eq!(value["world_sha256"], hex(&map.digest()));
    assert_eq!(
        value["authored_overlay"],
        serde_json::from_str::<serde_json::Value>(expansion::manifest_json()).unwrap()
    );
    for row in value["terrain_rows"].as_array().unwrap() {
        assert_eq!(row.as_str().unwrap().len(), 54);
    }
    assert_eq!(value["tiles"].as_array().unwrap().len(), 15);
    assert!(!root.exists());
    for flags in [
        vec!["--expansion-info"],
        vec!["--expansion-info", "-1"],
        vec!["--expansion-info", "4294967296"],
        vec!["--expansion-info", "42", "--worldgen-info", "42"],
        vec!["--expansion-info", "42", "--c41"],
        vec!["--c41", "--c40"],
        vec!["--c41", "--c41"],
        vec!["--c41", "--seed"],
        vec!["--c41", "--seed", "42", "--seed", "43"],
        vec!["--c41", "--pixel", "--debug-world"],
    ] {
        let bad = run(&root, &flags, "wait;save;quit");
        assert_eq!(bad.status.code(), Some(2), "{flags:?}");
        assert!(!root.exists());
    }
}

fn distance(a: Position, b: Position) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn next(w: &World, target: Position) -> Option<Direction> {
    let start = w.player().position;
    let mut queue = VecDeque::from([(start, None)]);
    let mut seen = BTreeSet::from([start]);
    while let Some((at, first)) = queue.pop_front() {
        if distance(at, target) <= 1 {
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
            queue.push_back((p, first.or(Some(d))));
        }
    }
    None
}
fn play(
    w: &mut World,
    commands: &mut Vec<&'static str>,
    intent: Intent,
    command: &'static str,
) -> Outcome {
    commands.push(command);
    w.step(w.tick(), intent).unwrap()
}
fn travel(w: &mut World, commands: &mut Vec<&'static str>, target: Position) {
    for _ in 0..400 {
        if distance(w.player().position, target) <= 1 {
            return;
        }
        let (intent, command) = match next(w, target) {
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
        play(w, commands, intent, command);
    }
    panic!("cannot reach target");
}

#[test]
fn actual_ascii_eastern_journey_and_localized_inspection_match_canonical_world() {
    let root = tempfile::tempdir().unwrap();
    let initial = run(
        &root.path().join("initial"),
        &["--c41", "--seed", "7"],
        "survey;quit",
    );
    let initial = String::from_utf8(initial.stdout).unwrap();
    assert!(initial.contains("Pineford lies east"));
    assert!(!initial.contains("Pineford depot:") && !initial.contains("Pineford grove:"));
    let mut w = World::expanded_riverlands(7);
    let mut commands = vec!["survey"];
    play(
        &mut w,
        &mut commands,
        Intent::ChooseBackground(Background::Peddler),
        "background peddler",
    );
    travel(&mut w, &mut commands, DEPOT);
    commands.push("survey");
    assert!(matches!(
        play(&mut w, &mut commands, Intent::BuyHerb, "buy"),
        Outcome::Traded { bought: true, .. }
    ));
    assert!(matches!(
        play(&mut w, &mut commands, Intent::SellHerb, "sell"),
        Outcome::Traded { bought: false, .. }
    ));
    travel(&mut w, &mut commands, GROVE);
    commands.push("survey");
    assert!(matches!(
        play(&mut w, &mut commands, Intent::Gather, "gather"),
        Outcome::Gathered
    ));
    commands.extend(["save", "reset", "load", "survey", "save", "quit"]);
    let expected = w.checkpoint();
    for lang in ["en", "id"] {
        let dir = root.path().join(lang);
        let out = run(
            &dir,
            &["--c41", "--seed", "7"],
            &format!("lang {lang};{}", commands.join(";")),
        );
        assert!(
            out.status.success() && out.stderr.is_empty(),
            "{:?}",
            out.stderr
        );
        let text = String::from_utf8(out.stdout).unwrap();
        if lang == "en" {
            assert!(
                text.contains("Pineford depot:")
                    && text.contains("Pineford grove:")
                    && text.contains("elevation")
                    && text.contains("Restored the settlement.")
            );
        } else {
            assert!(
                text.contains("Depot Pineford:")
                    && text.contains("Kebun Pineford:")
                    && text.contains("elevasi")
                    && text.contains("Desa berhasil dimuat.")
            );
        }
        assert!(!text.contains("living.expansion."));
        assert_eq!(save(&dir), expected);
    }
}

#[test]
fn separate_slot_preserves_c40_and_rejects_foreign_revision() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let legacy = World::generated_riverlands(7).checkpoint();
    let legacy_path = dir.join("checkpoint.c40");
    std::fs::write(&legacy_path, &legacy).unwrap();
    let out = run(root.path(), &["--c41"], "seed 42;wait;save;quit");
    assert!(out.status.success() && out.stderr.is_empty());
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy);
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.checkpoint_revision(), 28);
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    std::fs::write(dir.join("checkpoint.c41"), &legacy).unwrap();
    let bad = run(root.path(), &["--c41"], "load;save;quit");
    let text = String::from_utf8(bad.stdout).unwrap();
    assert!(text.contains("Load failed") && text.contains("Save failed"));
    assert_eq!(save(root.path()), legacy);
    assert_eq!(std::fs::read(legacy_path).unwrap(), legacy);
}
