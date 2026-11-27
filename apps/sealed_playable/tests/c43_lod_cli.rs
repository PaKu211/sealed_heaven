use sealed_sim::living::{World, lod::REVISION};
use std::{
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c43")).unwrap()
}
#[test]
fn inspection_is_developer_only_and_read_only_in_both_locales() {
    let root = tempfile::tempdir().unwrap();
    let mut expected = None;
    for (lang, developer) in [("en", false), ("id", false), ("en", true), ("id", true)] {
        let dir = root.path().join(format!("{lang}-{developer}"));
        let mut flags = vec!["--c43", "--seed", "7"];
        if developer {
            flags.push("--debug-world");
        }
        let commands = format!(
            "lang {lang};lod;{}lod;news;survey;save;reset;load;lod;save;quit",
            "wait;".repeat(180)
        );
        let p = run(&dir, &flags, &commands);
        assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
        let stdout = String::from_utf8(p.stdout).unwrap();
        assert_eq!(stdout.contains("LOD tick=180"), developer);
        assert_eq!(stdout.contains("Scheduled wake="), developer);
        let bytes = save(&dir);
        let w = World::restore(&bytes).unwrap();
        assert_eq!(w.tick(), 180);
        assert_eq!(w.checkpoint_revision(), REVISION);
        assert!(w.scheduler().unwrap().evaluation_totals().1 > 0);
        if let Some(old) = &expected {
            assert_eq!(&bytes, old);
        } else {
            expected = Some(bytes);
        }
    }
}
#[test]
fn launch_and_save_are_isolated_from_all_previous_revisions() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::regional_knowledge_campaign(7).checkpoint();
    let path = dir.join("checkpoint.c42");
    std::fs::write(&path, &old).unwrap();
    let p = run(root.path(), &["--c43"], "seed 42;wait;save;quit");
    assert!(p.status.success() && p.stderr.is_empty());
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c43"), &old).unwrap();
    let p = run(root.path(), &["--c43"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    for args in [
        vec!["--c43", "--c42"],
        vec!["--c43", "--c43"],
        vec!["--c43", "--seed"],
        vec!["--c43", "--seed", "4294967296"],
        vec!["--c43", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &args, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{args:?}");
        assert!(!isolated.exists());
    }
}
