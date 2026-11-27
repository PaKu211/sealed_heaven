use sealed_sim::living::{World, narrative::REVISION};
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
fn natural_episodes_and_private_world_replay_in_both_languages() {
    use sha2::{Digest, Sha256};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/living/c55_narrative_golden_v1.json"
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
                &["--c55", "--seed", &seed],
                &format!(
                    "lang {lang};{cmds};stories;storiesnext;storydebug;save;reset;load;stories;storiesnext;save;quit"
                ),
            );
            assert!(
                !output.contains("Narrative {")
                    && !output.contains("active_projects:")
                    && !output.contains("careers:")
                    && !output.contains("living.story.")
            );
            let b = std::fs::read(dir.join("SealedHeaven/checkpoint.c55")).unwrap();
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
            let c = row["counts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>();
            assert_eq!(
                w.narrative().unwrap().counts(),
                (c[0], c[1], c[2], c[3], c[4])
            );
            let markers = row[format!("player_lines_{lang}")].as_array().unwrap();
            for m in markers {
                assert!(output.contains(m.as_str().unwrap()), "{name} {lang} {m}");
            }
            if matches!(
                name,
                "initial" | "private-world" | "private-in-flight" | "seed42-passive"
            ) {
                assert_eq!(
                    w.known_story_projects().count() + w.known_story_inquiries().count(),
                    0
                );
                assert!(
                    !output.contains("Project #")
                        && !output.contains("Inquiry #")
                        && !output.contains("Proyek #")
                        && !output.contains("Penyelidikan #")
                );
            }
            if name == "project-report" {
                let p = w.known_story_projects().next().unwrap();
                assert!(p.dispatch.is_none() && p.delivery.is_none() && p.report.is_some());
                assert!(!output.contains("Project #0: t37") && !output.contains("Proyek #0: t37"));
            }
            if name == "project-register" {
                let p = w.known_story_projects().next().unwrap();
                assert!(p.dispatch.is_none() && p.delivery.is_some() && p.report.is_none());
                assert!(!output.contains("t71 #0:"));
            }
            if matches!(name, "checked-inquiry" | "lapsed-inquiry") {
                assert!(w.known_story_inquiries().all(|i| i.end.is_some()));
            }
        }
    }
}

#[test]
fn explicit_debug_and_separate_save_slot() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::rumor_campaign(9).checkpoint();
    std::fs::write(dir.join("checkpoint.c54"), &old).unwrap();
    let output = run(
        root.path(),
        &["--c55", "--debug-world"],
        "seed 42;storydebug;save;quit",
    );
    assert!(output.contains("Narrative {") && output.contains("active_projects:"));
    let file = dir.join("checkpoint.c55");
    assert_eq!(
        std::fs::read(&file).unwrap(),
        World::narrative_campaign(42).checkpoint()
    );
    assert_eq!(std::fs::read(dir.join("checkpoint.c54")).unwrap(), old);
    std::fs::write(&file, &old).unwrap();
    let output = run(root.path(), &["--c55"], "load;save;quit");
    assert!(output.contains("Load failed") && output.contains("Save failed"));
    assert_eq!(std::fs::read(file).unwrap(), old);
}
