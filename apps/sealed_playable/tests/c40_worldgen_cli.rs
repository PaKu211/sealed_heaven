use sealed_sim::living::{World, worldgen::GeneratedRegion};
use std::{
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c40")).unwrap()
}

#[test]
fn generated_manifest_is_deterministic_read_only_and_matches_runtime() {
    let root = tempfile::tempdir().unwrap();
    let untouched = root.path().join("untouched");
    let a = run(&untouched, &["--worldgen-info", "7"], "wait;save;quit");
    let b = run(&untouched, &["--worldgen-info", "7"], "quit");
    assert!(a.status.success() && a.stderr.is_empty());
    assert_eq!(a.stdout, b.stdout);
    let value: serde_json::Value = serde_json::from_slice(&a.stdout).unwrap();
    let map = GeneratedRegion::generate(7);
    let hash = map
        .digests()
        .world
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(value["digests"]["world"], hash);
    assert_eq!(value["checkpoint_revision"], 27);
    assert_eq!(value["generator_version"], 1);
    assert_eq!(value["terrain_rows"].as_array().unwrap().len(), 9);
    assert_eq!(value["tiles"].as_array().unwrap().len(), 9);
    assert!(!untouched.exists());
    for args in [
        vec!["--worldgen-info"],
        vec!["--worldgen-info", "-1"],
        vec!["--worldgen-info", "4294967296"],
        vec!["--worldgen-info", "7", "--c40"],
        vec!["--c40", "--seed"],
        vec!["--c40", "--seed", "1", "--seed", "2"],
        vec!["--c40", "--c40"],
        vec!["--c40", "--pixel", "--debug-world"],
        vec!["--c40", "--c35"],
    ] {
        let invalid = run(&untouched, &args, "wait;save;quit");
        assert_eq!(invalid.status.code(), Some(2), "{args:?}");
        assert!(!untouched.exists());
    }
}

#[test]
fn generated_campaign_surveys_and_locales_do_not_change_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let mut canonical = None;
    for (lang, args, script) in [
        (
            "en",
            vec!["--c40", "--seed", "7"],
            "lang en;survey;wait;survey;east;save;load;save;quit",
        ),
        (
            "id",
            vec!["--c40"],
            "lang id;seed 7;survey;wait;east;save;load;save;quit",
        ),
        ("plain", vec!["--c40", "--seed", "7"], "wait;east;save;quit"),
    ] {
        let dir = root.path().join(lang);
        let out = run(&dir, &args, script);
        assert!(
            out.status.success() && out.stderr.is_empty(),
            "{:?}",
            out.stderr
        );
        let text = String::from_utf8(out.stdout).unwrap();
        if lang == "en" {
            assert!(
                text.contains("elevation")
                    && text.contains("Qi")
                    && text.contains("Saved the settlement.")
            );
        }
        if lang == "id" {
            assert!(
                text.contains("elevasi")
                    && text.contains("kelembapan")
                    && text.contains("Desa tersimpan.")
            );
        }
        let bytes = save(&dir);
        let world = World::restore(&bytes).unwrap();
        assert_eq!(world.checkpoint_revision(), 27);
        assert_eq!(world.region().unwrap().seed, 7);
        assert_eq!(world.tick(), 2);
        assert_eq!(world.checkpoint(), bytes);
        if let Some(expected) = &canonical {
            assert_eq!(&bytes, expected);
        } else {
            canonical = Some(bytes);
        }
    }
}

#[test]
fn generated_slot_never_overwrites_legacy_save_or_accepts_foreign_revision() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let legacy = World::duty_coherence_campaign(0).checkpoint();
    let legacy_path = dir.join("checkpoint.c351");
    std::fs::write(&legacy_path, &legacy).unwrap();
    let out = run(root.path(), &["--c40", "--seed", "42"], "wait;save;quit");
    assert!(out.status.success() && out.stderr.is_empty());
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy);
    assert_eq!(
        World::restore(&save(root.path()))
            .unwrap()
            .region()
            .unwrap()
            .seed,
        42
    );
    std::fs::write(dir.join("checkpoint.c40"), &legacy).unwrap();
    let foreign = run(root.path(), &["--c40"], "load;save;quit");
    let text = String::from_utf8(foreign.stdout).unwrap();
    assert!(
        foreign.status.success() && text.contains("Load failed") && text.contains("Save failed")
    );
    assert_eq!(save(root.path()), legacy);
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy);
}
