use sealed_sim::living::{World, social_network::REVISION};
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
fn frozen_social_endpoints_match_in_both_locales_and_queries_are_read_only() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c50_social_network_golden_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let seed = row["seed"].as_u64().unwrap().to_string();
        let commands = row["commands"]
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
                &["--c50", "--seed", &seed],
                &format!(
                    "lang {lang};{commands};sociallog 48;socialdebug;save;reset;load;sociallog 48;save;quit"
                ),
            );
            assert!(
                !output.contains("SocialNetwork {")
                    && !output.contains("last_activity:")
                    && !output.contains("living.network.")
            );
            let b = std::fs::read(dir.join("SealedHeaven/checkpoint.c50")).unwrap();
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
            let witnessed = row["witnessed"].as_bool().unwrap();
            assert_eq!(w.witnessed_social().next().is_some(), witnessed);
            let marker = match (lang, witnessed) {
                ("en", true) => "greeted at",
                ("id", true) => "bertegur sapa di",
                ("en", false) => "No witnessed social acts yet.",
                _ => "Belum ada interaksi sosial yang disaksikan.",
            };
            assert!(output.contains(marker), "{name} {lang}: {marker}");
        }
    }
}

#[test]
fn debug_graph_is_explicit_and_save_slot_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let prior = World::encounter_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c49"), &prior).unwrap();
    let output = run(
        root.path(),
        &["--c50", "--debug-world"],
        "seed 7;socialdebug;save;quit",
    );
    assert!(output.contains("SocialNetwork {") && output.contains("last_activity:"));
    let file = dir.join("checkpoint.c50");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::social_network_campaign(7).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c49")).unwrap(), prior);
    std::fs::write(&file, &prior).unwrap();
    let output = run(root.path(), &["--c50"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), prior);
}
