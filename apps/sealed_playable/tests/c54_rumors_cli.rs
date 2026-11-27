use sealed_sim::living::{
    World,
    regional_news::Site,
    rumors::{REVISION, Status},
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
fn frozen_player_accounts_and_private_network_replay_in_both_languages() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c54_rumors_golden_v1.json"
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
                &["--c54", "--seed", &seed],
                &format!(
                    "lang {lang};{commands};rumors;rumorsnext;rumordebug;save;reset;load;rumors;rumorsnext;save;quit"
                ),
            );
            assert!(
                !output.contains("Rumors {")
                    && !output.contains("last_exchange:")
                    && !output.contains("living.rumor.")
            );
            let bytes = std::fs::read(dir.join("SealedHeaven/checkpoint.c54")).unwrap();
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
                w.rumors().unwrap().counts(),
                (counts[0], counts[1], counts[2])
            );
            let known = Site::ALL
                .into_iter()
                .any(|s| w.known_rumor(s).unwrap().status != Status::Unknown);
            assert_eq!(known, matches!(name, "local-shrine" | "heard-account"));
            if lang == "en" {
                for line in row["player_lines_en"].as_array().unwrap() {
                    assert!(output.contains(line.as_str().unwrap()), "{name} {}", line);
                }
            } else {
                let marker = match name {
                    "local-shrine" => "diperiksa langsung (bertanggal)",
                    "heard-account" => {
                        "sumber Shen Wei, melalui Shen Wei, keyakinan 67/100, perantara 1"
                    }
                    _ => "Belum ada pengamatan lokal atau kabar yang masih segar.",
                };
                assert!(output.contains(marker), "{name} {marker}");
            }
            if !known {
                assert!(
                    !output.contains("[reported, not verified]")
                        && !output.contains("[competing dated accounts]")
                        && !output.contains("[locally checked (dated)]")
                );
                assert!(
                    !output.contains("[kabar, belum terverifikasi]")
                        && !output.contains("[kesaksian berbeda waktu]")
                        && !output.contains("[diperiksa langsung (bertanggal)]")
                );
            }
        }
    }
}

#[test]
fn explicit_debug_and_isolated_save_slot() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let prior = World::communication_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c53"), &prior).unwrap();
    let output = run(
        root.path(),
        &["--c54", "--debug-world"],
        "seed 42;rumordebug;save;quit",
    );
    assert!(output.contains("Rumors {") && output.contains("last_exchange:"));
    let file = dir.join("checkpoint.c54");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::rumor_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c53")).unwrap(), prior);
    std::fs::write(&file, &prior).unwrap();
    let output = run(root.path(), &["--c54"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), prior);
}
