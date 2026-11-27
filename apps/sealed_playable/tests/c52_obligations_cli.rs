use sealed_sim::living::{World, obligations::REVISION};
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
fn frozen_obligation_endpoints_and_private_acknowledgements_replay_in_both_languages() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c52_obligations_golden_v1.json"
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
                &["--c52", "--seed", &seed],
                &format!(
                    "lang {lang};{commands};obligations 48;obligationdebug;teams 48;save;reset;load;obligations 48;teams 48;save;quit"
                ),
            );
            assert!(
                !output.contains("Obligations {")
                    && !output.contains("regard_change:")
                    && !output.contains("living.obligation.")
            );
            let bytes = std::fs::read(dir.join("SealedHeaven/checkpoint.c52")).unwrap();
            let hex = |v: &[u8]| v.iter().map(|n| format!("{n:02x}")).collect::<String>();
            assert_eq!(
                hex(&Sha256::digest(&bytes)),
                row["checkpoint_sha256"].as_str().unwrap(),
                "{name} {lang}"
            );
            let w = World::restore(&bytes).unwrap();
            assert_eq!(w.tick(), row["tick"].as_u64().unwrap());
            assert_eq!(w.checkpoint_revision(), REVISION);
            assert_eq!(
                hex(&w.state_digest()),
                row["state_sha256"].as_str().unwrap()
            );
            let counts = row["counts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_u64().unwrap() as u32)
                .collect::<Vec<_>>();
            assert_eq!(
                w.obligations().unwrap().counts(),
                (counts[0], counts[1], counts[2], counts[3])
            );
            assert!(w.witnessed_obligations().next().is_none());
            let known = row["known_receipt"].as_bool().unwrap();
            assert_eq!(w.known_cooperation().next().is_some(), known);
            assert!(output.contains(if lang == "en" {
                "No witnessed promise acknowledgements yet."
            } else {
                "Belum ada konfirmasi janji yang disaksikan."
            }));
            assert!(
                !output.contains("confirmed delivery to")
                    && !output.contains("mengonfirmasi pengiriman kepada")
            );
            if known {
                assert!(output.contains(if lang == "en" {
                    "Receipt t37 #0:"
                } else {
                    "Catatan t37 #0:"
                }));
            }
        }
    }
}

#[test]
fn developer_inspection_and_legacy_save_isolation() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let prior = World::cooperation_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c51"), &prior).unwrap();
    let output = run(
        root.path(),
        &["--c52", "--debug-world"],
        "seed 42;obligationdebug;save;quit",
    );
    assert!(output.contains("Obligations {") && output.contains("reliability:"));
    let file = dir.join("checkpoint.c52");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::obligations_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c51")).unwrap(), prior);
    std::fs::write(&file, &prior).unwrap();
    let output = run(root.path(), &["--c52"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), prior);
}
