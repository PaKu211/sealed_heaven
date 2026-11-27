use sealed_sim::living::{
    ActorId, World,
    diplomacy::{Kind, REVISION},
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
fn diplomatic_observations_and_dated_disclosures_replay_in_both_languages() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c58_diplomacy_golden_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let kinds = [
        Kind::Accord,
        Kind::Disputed,
        Kind::Received,
        Kind::Confirmed,
        Kind::Superseded,
        Kind::Lapsed,
        Kind::Reconciled,
        Kind::Petition,
        Kind::Withdrawn,
        Kind::Expired,
        Kind::Returned,
    ];
    let mut dated = None;
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
                &["--c58", "--seed", &seed],
                &format!(
                    "lang {lang};{cmds};diplomacy;diplomacynext;diplomacydebug;save;reset;load;diplomacy;diplomacynext;save;quit"
                ),
            );
            assert!(
                !output.contains("Diplomacy {")
                    && !output.contains("assessments:")
                    && !output.contains("exclusive_ticks:")
                    && !output.contains("living.diplomacy.")
            );
            let b = std::fs::read(dir.join("SealedHeaven/checkpoint.c58")).unwrap();
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
            let d = w.diplomacy().unwrap();
            for (k, n) in kinds.into_iter().zip(row["counts"].as_array().unwrap()) {
                assert_eq!(u64::from(d.count(k)), n.as_u64().unwrap());
            }
            assert_eq!(
                u64::from(d.effort_ticks()),
                row["effort_ticks"].as_u64().unwrap()
            );
            assert_eq!(
                u64::from(d.exclusive_ticks()),
                row["exclusive_ticks"].as_u64().unwrap()
            );
            for m in row[format!("player_lines_{lang}")].as_array().unwrap() {
                assert!(output.contains(m.as_str().unwrap()), "{name} {lang} {m}");
            }
            if matches!(
                name,
                "initial" | "private-accord" | "seed7-private" | "seed42-private" | "seed0-passive"
            ) {
                assert_eq!(
                    w.known_diplomacy().count() + w.witnessed_diplomacy().count(),
                    0
                );
                assert!(
                    !output.contains("agreed to share reports")
                        && !output.contains("sepakat bertukar kabar")
                );
            }
            if name == "witnessed-disagreement" {
                assert!(w.witnessed_diplomacy().any(|e| e.kind == Kind::Disputed));
                assert_eq!(w.known_diplomacy().count(), 0);
            }
            if name == "witnessed-negotiations" {
                assert!(w.witnessed_diplomacy().any(|e| e.kind == Kind::Accord));
            }
            if name == "member-disclosure" {
                let actor = ActorId::new(
                    row["commands"]
                        .as_array()
                        .unwrap()
                        .last()
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .strip_prefix("talk ")
                        .unwrap()
                        .parse()
                        .unwrap(),
                );
                let info = d.known()[&actor];
                assert_eq!(info.heard, w.tick());
                assert!(info.event.at <= info.heard);
                if let Some(old) = dated {
                    assert_eq!(info, old);
                } else {
                    dated = Some(info);
                }
            }
            if name == "dated-disclosure" {
                let info = dated.expect("earlier disclosure fixture");
                assert_eq!(d.known()[&info.speaker], info);
            }
        }
    }
}
#[test]
fn explicit_debug_and_separate_save_slot() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::circles_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c57"), &old).unwrap();
    let output = run(
        root.path(),
        &["--c58", "--debug-world"],
        "seed 42;diplomacydebug;save;quit",
    );
    assert!(output.contains("Diplomacy {") && output.contains("assessments:"));
    let file = dir.join("checkpoint.c58");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::diplomacy_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c57")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let output = run(root.path(), &["--c58"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}
