use sealed_sim::living::{
    ActorId, World,
    development::{Focus, REVISION},
};
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
fn dated_disclosures_and_private_learning_replay_in_both_languages() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c56_development_golden_v1.json"
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
            let output = run(
                &dir,
                &["--c56", "--seed", &seed],
                &format!(
                    "lang {lang};{cmds};profiles;profilesnext;profiledebug;save;reset;load;profiles;profilesnext;save;quit"
                ),
            );
            assert!(
                !output.contains("Development {")
                    && !output.contains("inquiry_points:")
                    && !output.contains("partners:")
                    && !output.contains("living.profile.")
            );
            let b = std::fs::read(dir.join("SealedHeaven/checkpoint.c56")).unwrap();
            let hex = |v: &[u8]| v.iter().map(|n| format!("{n:02x}")).collect::<String>();
            assert_eq!(
                hex(&Sha256::digest(&b)),
                row["checkpoint_sha256"].as_str().unwrap(),
                "{name} {lang}"
            );
            let w = World::restore(&b).unwrap();
            assert_eq!(w.tick(), row["tick"].as_u64().unwrap());
            assert_eq!(w.checkpoint_revision(), REVISION);
            assert_eq!(
                hex(&w.state_digest()),
                row["state_sha256"].as_str().unwrap()
            );
            let d = w.development().unwrap();
            assert_eq!(u64::from(d.changes()), row["changes"].as_u64().unwrap());
            for m in row[format!("player_lines_{lang}")].as_array().unwrap() {
                assert!(output.contains(m.as_str().unwrap()), "{name} {lang} {m}");
            }
            if matches!(
                name,
                "initial"
                    | "private-world"
                    | "private-own-outcome"
                    | "natural-verification"
                    | "seed42-passive"
            ) {
                assert_eq!(w.known_pursuits().count(), 0);
                assert!(
                    !output.contains(", said at t") && !output.contains(", menyampaikan pada t")
                );
            }
            let mei = ActorId::new(31);
            if name == "private-own-outcome" {
                assert_eq!(d.profiles()[&ActorId::new(32)].focus, Focus::Partnership);
                assert_eq!(d.profiles()[&mei].focus, Focus::Vocation);
            }
            if name == "early-partner-disclosure" {
                assert_eq!(d.known()[&mei].focus, Focus::Vocation);
                assert_eq!(d.known()[&mei].at, 41);
            }
            if name == "stale-partner-disclosure" {
                assert_eq!(d.profiles()[&mei].focus, Focus::Partnership);
                assert_eq!(d.known()[&mei].focus, Focus::Vocation);
                assert_eq!(d.known()[&mei].at, 41);
                assert!(
                    !output.contains("working toward dependable partnerships")
                        && !output.contains("mengupayakan kerja sama yang andal")
                );
            }
            if name == "updated-partner-disclosure" {
                assert_eq!(d.known()[&mei].focus, Focus::Partnership);
                assert_eq!(d.known()[&mei].at, 213);
            }
            if name == "natural-verification" {
                assert!(d.profiles().values().any(|p| p.extended_ticks > 0));
            }
        }
    }
}

#[test]
fn explicit_debug_and_separate_save_slot() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::narrative_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c55"), &old).unwrap();
    let output = run(
        root.path(),
        &["--c56", "--debug-world"],
        "seed 42;profiledebug;save;quit",
    );
    assert!(output.contains("Development {") && output.contains("inquiry_points:"));
    let file = dir.join("checkpoint.c56");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::development_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c55")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let output = run(root.path(), &["--c56"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}
