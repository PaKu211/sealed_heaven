use sealed_sim::living::{ActorId, Intent, PLAYER, World, favors::REVISION};
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c47")).unwrap()
}
fn hash(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
fn five_real_personal_branches_replay_bilingually_and_preserve_social_state() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c47_branch_trace_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for r in fixture["rows"].as_array().unwrap() {
        let choice = r["choice"].as_str().unwrap();
        let seed = r["seed"].as_u64().unwrap().to_string();
        let actor = ActorId::new(r["actor"].as_u64().unwrap().try_into().unwrap());
        let commands = r["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        let first = r["offer_tick"].as_u64().unwrap() as usize;
        for lang in ["en", "id"] {
            let dir = root.path().join(format!("{choice}-{lang}"));
            let script = format!(
                "lang {lang};favors;favorlog;favordebug;{};favors;save;{};favors;favorlog;save;reset;load;favorlog;save;quit",
                commands[..first].join(";"),
                commands[first..].join(";")
            );
            let p = run(&dir, &["--c47", "--seed", &seed], &script);
            assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
            let out = String::from_utf8(p.stdout).unwrap();
            assert!(out.contains(if lang == "en" {
                "No known personal requests."
            } else {
                "Belum ada permintaan personal diketahui."
            }));
            assert!(out.contains(if lang == "en" {
                "1 herb for travel kit"
            } else {
                "1 herba bekal"
            }));
            assert!(out.contains(match (lang, choice) {
                ("en", "fulfill") => "Help given",
                ("en", "decline") => "Declined",
                ("en", "withdraw") => "Withdrawn",
                ("en", _) => "Circumstances changed",
                ("id", "fulfill") => "Bantuan diberikan",
                ("id", "decline") => "Ditolak",
                ("id", "withdraw") => "Dibatalkan",
                _ => "Keadaan berubah",
            }));
            assert!(
                !out.contains("Favors {")
                    && !out.contains("next_id:")
                    && !out.contains("living.favor.")
            );
            if choice == "fulfill" {
                assert!(out.contains(if lang == "en" {
                    "trust change +1"
                } else {
                    "perubahan percaya +1"
                }));
            }
            assert!(out.contains(if lang == "en" {
                "Restored the settlement."
            } else {
                "Desa berhasil dimuat."
            }));
            let bytes = save(&dir);
            assert_eq!(hash(&bytes), r["checkpoint_sha256"].as_str().unwrap());
            let w = World::restore(&bytes).unwrap();
            assert_eq!(w.checkpoint_revision(), REVISION);
            assert_eq!(
                i64::from(w.social().relationship(actor, PLAYER).regard),
                r["regard"].as_i64().unwrap()
            );
            assert_eq!(
                std::fs::read_dir(dir.join("SealedHeaven")).unwrap().count(),
                1
            );
        }
    }
}
#[test]
fn launch_slots_debug_and_funding_command_are_revision_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::emergent_requests_campaign(7).checkpoint();
    std::fs::write(dir.join("checkpoint.c46"), &old).unwrap();
    let p = run(
        root.path(),
        &["--c47", "--debug-world"],
        "seed 42;favordebug;fundfavor 4294967295;favors;save;quit",
    );
    assert!(p.status.success() && p.stderr.is_empty());
    assert!(String::from_utf8(p.stdout).unwrap().contains("Favors {"));
    let mut expected = World::personal_favors_campaign(42);
    expected.step(0, Intent::FundFavor(u32::MAX)).unwrap();
    assert_eq!(save(root.path()), expected.checkpoint());
    assert_eq!(std::fs::read(dir.join("checkpoint.c46")).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c47"), &old).unwrap();
    let p = run(root.path(), &["--c47"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    for flags in [
        vec!["--c47", "--c46"],
        vec!["--c47", "--c47"],
        vec!["--c47", "--seed"],
        vec!["--c47", "--seed", "4294967296"],
        vec!["--c47", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &flags, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{flags:?}");
        assert!(!isolated.exists());
    }
}
