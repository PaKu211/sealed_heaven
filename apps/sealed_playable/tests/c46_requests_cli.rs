use sealed_sim::living::{World, requests::REVISION};
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c46")).unwrap()
}
fn hash(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
fn six_real_request_choices_replay_in_both_locales_and_save_load_exactly() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c46_branch_trace_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for r in fixture["rows"].as_array().unwrap() {
        let site = r["site"].as_str().unwrap();
        let choice = r["choice"].as_str().unwrap();
        let seed = r["seed"].as_u64().unwrap().to_string();
        let commands = r["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        let first = commands
            .iter()
            .position(|c| c.starts_with("acceptquest") || c.starts_with("declinequest"))
            .unwrap();
        for lang in ["en", "id"] {
            let dir = root.path().join(format!("{site}-{choice}-{lang}"));
            let script = format!(
                "lang {lang};quests;questlog;questdebug;{};quests;save;{};quests;questlog;save;reset;load;questlog;save;quit",
                commands[..first].join(";"),
                commands[first..].join(";")
            );
            let p = run(&dir, &["--c46", "--seed", &seed], &script);
            assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
            let out = String::from_utf8(p.stdout).unwrap();
            assert!(out.contains(if lang == "en" {
                "No known requests."
            } else {
                "Belum ada permintaan diketahui."
            }));
            assert!(out.contains(if lang == "en" {
                "Offered"
            } else {
                "Ditawarkan"
            }));
            assert!(out.contains(match (lang, choice) {
                ("en", "complete") => "Completed",
                ("en", "decline") => "Declined",
                ("en", _) => "Withdrawn",
                ("id", "complete") => "Selesai",
                ("id", "decline") => "Ditolak",
                _ => "Dibatalkan",
            }));
            assert!(!out.contains("Requests {") && !out.contains("next_id:"));
            if site == "market" && choice == "complete" {
                assert!(out.contains(if lang == "en" {
                    "bonus 1 paid"
                } else {
                    "bonus 1 dibayar"
                }));
            }
            assert!(out.contains(if lang == "en" {
                "Restored the settlement."
            } else {
                "Desa berhasil dimuat."
            }));
            let bytes = save(&dir);
            assert_eq!(hash(&bytes), r["checkpoint_sha256"].as_str().unwrap());
            assert_eq!(
                World::restore(&bytes).unwrap().checkpoint_revision(),
                REVISION
            );
        }
    }
}
#[test]
fn new_launch_save_slots_and_developer_inspection_are_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::road_history_campaign(7).checkpoint();
    std::fs::write(dir.join("checkpoint.c45"), &old).unwrap();
    let p = run(
        root.path(),
        &["--c46", "--debug-world"],
        "seed 42;questdebug;wait;quests;save;quit",
    );
    assert!(p.status.success() && p.stderr.is_empty());
    assert!(String::from_utf8(p.stdout).unwrap().contains("Requests {"));
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.checkpoint_revision(), REVISION);
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    assert_eq!(std::fs::read(dir.join("checkpoint.c45")).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c46"), &old).unwrap();
    let p = run(root.path(), &["--c46"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    for flags in [
        vec!["--c46", "--c45"],
        vec!["--c46", "--c46"],
        vec!["--c46", "--seed"],
        vec!["--c46", "--seed", "4294967296"],
        vec!["--c46", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &flags, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{flags:?}");
        assert!(!isolated.exists());
    }
}
