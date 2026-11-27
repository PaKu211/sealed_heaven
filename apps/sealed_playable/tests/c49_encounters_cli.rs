use sealed_sim::living::{ActorId, World, encounters::REVISION};
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

#[test]
fn actual_road_conversations_are_persistent_and_read_only_in_both_languages() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c48_branch_trace_v1.json"
    ))
    .unwrap();
    let route = fixture["rows"][0]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .take(30)
        .map(|s| s.as_str().unwrap())
        .collect::<Vec<_>>()
        .join(";");
    let root = tempfile::tempdir().unwrap();
    let mut expected = None;
    for lang in ["en", "id"] {
        let dir = root.path().join(lang);
        let s = run(
            &dir,
            &["--c49"],
            &format!(
                "lang {lang};contacts;{route};save;contacts;contacts 1;help;save;reset;load;contacts;save;quit"
            ),
        );
        assert!(
            s.contains(if lang == "en" {
                "No confirmed encounters yet."
            } else {
                "Belum ada pertemuan yang terkonfirmasi."
            }),
            "{s}"
        );
        assert!(
            s.contains(if lang == "en" {
                "last met t30"
            } else {
                "terakhir ditemui t30"
            }),
            "{s}"
        );
        assert!(!s.contains("living.contact."));
        let bytes = std::fs::read(dir.join("SealedHeaven/checkpoint.c49")).unwrap();
        let w = World::restore(&bytes).unwrap();
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert_eq!(w.tick(), 30);
        let e = w.encounters().unwrap().contact(ActorId::new(22)).unwrap();
        assert_eq!(
            (e.first_at, e.last_at, e.visits, e.distinct_talks),
            (26, 30, 2, 1)
        );
        if let Some(ref first) = expected {
            assert_eq!(first, &bytes);
        } else {
            expected = Some(bytes);
        }
    }
}

#[test]
fn c48_slot_is_unchanged_and_foreign_save_is_not_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::reciprocal_bonds_campaign(7).checkpoint();
    std::fs::write(dir.join("checkpoint.c48"), &old).unwrap();
    run(root.path(), &["--c49"], "seed 42;contacts;save;quit");
    let file = dir.join("checkpoint.c49");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::encounter_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c48")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let s = run(root.path(), &["--c49"], "load;save;quit");
    assert!(s.contains("Load failed") && s.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}

#[test]
fn four_frozen_endpoints_are_exact_across_languages_and_replay() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c49_encounter_golden_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let seed = row["seed"].as_u64().unwrap().to_string();
        let cmds = row["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(";");
        for lang in ["en", "id"] {
            let dir = root.path().join(format!("{name}-{lang}"));
            run(
                &dir,
                &["--c49", "--seed", &seed],
                &format!("lang {lang};{cmds};contacts;save;reset;load;contacts;save;quit"),
            );
            let b = std::fs::read(dir.join("SealedHeaven/checkpoint.c49")).unwrap();
            let hex = |bytes: &[u8]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
            assert_eq!(
                hex(&Sha256::digest(&b)),
                row["checkpoint_sha256"].as_str().unwrap(),
                "{name} {lang}"
            );
            let world = World::restore(&b).unwrap();
            assert_eq!(world.tick(), row["tick"].as_u64().unwrap());
            assert_eq!(
                hex(&world.state_digest()),
                row["state_sha256"].as_str().unwrap(),
                "{name} {lang}"
            );
        }
    }
}
