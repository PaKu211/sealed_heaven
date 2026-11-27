use sealed_sim::living::{World, exploration::REVISION};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, process::Command};

fn run(root: &Path, flags: &[&str], commands: &str) -> String {
    let p = Command::new(env!("CARGO_BIN_EXE_sealed_agency"))
        .args(flags)
        .env("LOCALAPPDATA", root)
        .env("SEALED_ASCII_COMMANDS", commands)
        .env_remove("SEALED_LOCALE_DIR")
        .output()
        .unwrap();
    assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
    String::from_utf8(p.stdout).unwrap()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|n| format!("{n:02x}")).collect()
}
#[test]
fn vertical_slice_golden_replays_bilingual_save_and_restore() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c60_sandbox_golden_v1.json"
    ))
    .unwrap();
    assert_eq!(data["revision"].as_u64(), Some(u64::from(REVISION)));
    let rows = data["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 12);
    let temp = tempfile::tempdir().unwrap();
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let seed = row["seed"].as_u64().unwrap().to_string();
        let mut baseline = None;
        for lang in ["en", "id"] {
            let root = temp.path().join(format!("{name}-{lang}"));
            let mut cmds = vec![format!("lang {lang}")];
            cmds.extend(
                row["commands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_str().unwrap().to_owned()),
            );
            cmds.extend(["map", "save", "reset", "load", "map", "save", "quit"].map(str::to_owned));
            let out = run(&root, &["--c60", "--seed", &seed], &cmds.join(";"));
            assert!(
                !out.contains("DEVELOPER VIEW") && !out.contains("living.field."),
                "{name} {lang}"
            );
            let atlas = row["atlas"].as_array().unwrap();
            let expected = atlas
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(out.contains(&expected), "atlas {name} {lang}");
            for line in row[format!("player_lines_{lang}")].as_array().unwrap() {
                assert!(
                    out.contains(line.as_str().unwrap()),
                    "{name} {lang}: {line}"
                );
            }
            let b = std::fs::read(root.join("SealedHeaven/checkpoint.c60")).unwrap();
            assert_eq!(
                hex(&Sha256::digest(&b)),
                row["checkpoint_sha256"].as_str().unwrap(),
                "{name} {lang}"
            );
            assert_eq!(
                hex(&b[b.len() - 64..b.len() - 32]),
                row["state_sha256"].as_str().unwrap()
            );
            let w = World::restore(&b).unwrap();
            assert_eq!(w.checkpoint_revision(), REVISION);
            assert_eq!(w.tick(), row["tick"].as_u64().unwrap());
            assert_eq!(w.checkpoint(), b);
            if name == "enrollment" || name == "mapped_return" {
                assert!(w.region().unwrap().enrolled);
            }
            if name == "mapped_return" {
                assert!(w.exploration().unwrap().count() >= 570);
                let shrine = w.scenario().shrine;
                let pos = w.player().position;
                assert!(pos.x.abs_diff(shrine.x) + pos.y.abs_diff(shrine.y) <= 1);
            }
            if let Some(prev) = &baseline {
                assert_eq!(&b, prev, "{name}: locale must not alter state");
            } else {
                baseline = Some(b);
            }
        }
    }
}
#[test]
fn menu_and_unknown_route_are_read_only_and_c59_slot_is_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::governance_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c59"), &old).unwrap();
    let out = run(
        root.path(),
        &["--c60", "--seed", "3"],
        "field travel;route depot;go;field journal;close;map;close;save;quit",
    );
    assert!(!out.contains("route depot]") && !out.contains("DEVELOPER VIEW"));
    let file = dir.join("checkpoint.c60");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::sandbox_campaign(3).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c59")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let out = run(root.path(), &["--c60"], "load;save;quit");
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}
