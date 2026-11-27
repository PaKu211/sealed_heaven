use sealed_sim::living::{Need, World, governance::REVISION};
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
fn bilingual_governance_is_read_only_and_deterministic() {
    let root = tempfile::tempdir().unwrap();
    for seed in [0u32, 3, 7, 42] {
        let mut baseline = World::governance_campaign(seed);
        for _ in 0..200 {
            baseline
                .step(baseline.tick(), sealed_sim::living::Intent::Wait)
                .unwrap();
        }
        let seed_text = seed.to_string();
        for lang in ["en", "id"] {
            let d = root.path().join(format!("{seed}-{lang}"));
            let mut commands = vec![format!("lang {lang}"), "governance".to_string()];
            commands.extend((0..200).map(|_| "wait".to_string()));
            commands.extend(
                [
                    "governance",
                    "governancenext",
                    "governancedebug",
                    "save",
                    "reset",
                    "load",
                    "governance",
                    "save",
                    "quit",
                ]
                .map(str::to_owned),
            );
            let out = run(&d, &["--c59", "--seed", &seed_text], &commands.join(";"));
            assert!(
                !out.contains("Governance {")
                    && !out.contains("ledgers:")
                    && !out.contains("living.governance.")
            );
            let b = std::fs::read(d.join("SealedHeaven/checkpoint.c59")).unwrap();
            assert_eq!(b, baseline.checkpoint());
            let w = World::restore(&b).unwrap();
            assert_eq!(w.tick(), 200);
            assert_eq!(w.checkpoint_revision(), REVISION);
            if w.known_board(Need::ShrineMedicine).is_none()
                && w.known_board(Need::HallOfferings).is_none()
            {
                assert!(out.contains(if lang == "en" {
                    "No institutional board read yet."
                } else {
                    "Belum ada papan institusi yang dibaca."
                }));
            }
        }
    }
}
#[test]
fn explicit_debug_and_separate_save_slot() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::diplomacy_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c58"), &old).unwrap();
    let out = run(
        root.path(),
        &["--c59", "--debug-world"],
        "seed 42;governancedebug;save;quit",
    );
    assert!(out.contains("Governance {") && out.contains("ledgers:"));
    let file = dir.join("checkpoint.c59");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::governance_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c58")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let out = run(root.path(), &["--c59"], "load;save;quit");
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}
