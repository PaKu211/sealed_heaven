use sealed_sim::living::{ActorId, Intent, PLAYER, World, reciprocity::REVISION};
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c48")).unwrap()
}
fn hash(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[test]
fn seven_canonical_reciprocity_endpoints_replay_bilingually_and_save_exactly() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c48_branch_trace_v1.json"
    ))
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let choice = row["choice"].as_str().unwrap();
        let seed = row["seed"].as_u64().unwrap().to_string();
        let actor = ActorId::new(row["actor"].as_u64().unwrap().try_into().unwrap());
        let commands = row["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        let last = commands.len() - 1;
        for lang in ["en", "id"] {
            let dir = root.path().join(format!("{choice}-{lang}"));
            let script = format!(
                "lang {lang};bonds;aidlog;bonddebug;{};bonds;save;{};bonds;aidlog;save;reset;load;aidlog;save;quit",
                commands[..last].join(";"),
                commands[last]
            );
            let p = run(&dir, &["--c48", "--seed", &seed], &script);
            assert!(
                p.status.success() && p.stderr.is_empty(),
                "{choice} {lang}: {:?}",
                p.stderr
            );
            let out = String::from_utf8(p.stdout).unwrap();
            assert!(out.contains(if lang == "en" {
                "No outstanding goodwill."
            } else {
                "Tidak ada utang budi tersisa."
            }));
            assert!(out.contains(if lang == "en" {
                "No witnessed acts of aid."
            } else {
                "Belum ada bantuan yang disaksikan."
            }));
            assert!(
                !out.contains("Reciprocity {")
                    && !out.contains("next_event:")
                    && !out.contains("living.bond.")
            );
            let marker = match (lang, choice) {
                ("en", "return") => "returned an herb.",
                ("id", "return") => "membalas dengan satu herba.",
                ("en", "waive") => "released Road outrider",
                ("id", "waive") => "melepaskan utang budi Road outrider",
                ("en", "repay") => "You returned a coin to",
                ("id", "repay") => "Kamu membalas satu koin kepada",
                ("en", _) => "Packed an herb",
                _ => "Memberi bekal herba",
            };
            assert!(out.contains(marker), "{choice} {lang}: missing {marker}");
            assert!(out.contains(if lang == "en" {
                "Restored the settlement."
            } else {
                "Desa berhasil dimuat."
            }));
            let bytes = save(&dir);
            assert_eq!(
                hash(&bytes),
                row["checkpoint_sha256"].as_str().unwrap(),
                "{choice} {lang}"
            );
            let w = World::restore(&bytes).unwrap();
            assert_eq!(w.checkpoint_revision(), REVISION);
            assert_eq!(
                i64::from(w.reciprocity().unwrap().net(PLAYER, actor)),
                row["balance"].as_i64().unwrap()
            );
            assert_eq!(
                std::fs::read_dir(dir.join("SealedHeaven")).unwrap().count(),
                1
            );
        }
    }
}
#[test]
fn new_launch_slots_dev_view_and_intents_are_revision_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::personal_favors_campaign(7).checkpoint();
    std::fs::write(dir.join("checkpoint.c47"), &old).unwrap();
    let p = run(
        root.path(),
        &["--c48", "--debug-world"],
        "seed 42;bonddebug;save;quit",
    );
    assert!(p.status.success() && p.stderr.is_empty());
    assert!(
        String::from_utf8(p.stdout)
            .unwrap()
            .contains("Reciprocity {")
    );
    assert_eq!(
        save(root.path()),
        World::reciprocal_bonds_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c47")).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c48"), &old).unwrap();
    let p = run(root.path(), &["--c48"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    let mut previous = World::personal_favors_campaign(0);
    for i in [
        Intent::RequestAid(ActorId::new(22)),
        Intent::RepayAid(ActorId::new(22)),
        Intent::WaiveAid(ActorId::new(22)),
    ] {
        assert!(previous.step(0, i).is_err());
        assert_eq!(previous.tick(), 0);
    }
    for flags in [
        vec!["--c48", "--c47"],
        vec!["--c48", "--c48"],
        vec!["--c48", "--seed"],
        vec!["--c48", "--seed", "4294967296"],
        vec!["--c48", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &flags, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{flags:?}");
        assert!(!isolated.exists());
    }
}
