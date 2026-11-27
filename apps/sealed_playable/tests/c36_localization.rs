//! A genuine executable-level localization test. No process-global environment
//! changes, no real user save writes, no simulation rule changes.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(save: &Path, pack: Option<&Path>, commands: &str) -> (Output, Vec<u8>) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sealed_agency"));
    cmd.env("LOCALAPPDATA", save)
        .env("SEALED_ASCII_COMMANDS", commands)
        .env_remove("SEALED_LOCALE_DIR");
    if let Some(pack) = pack {
        cmd.env("SEALED_LOCALE_DIR", pack);
    }
    let result = cmd.output().expect("launch actual client");
    assert!(result.status.success(), "{:?}", result);
    let checkpoint = fs::read(save.join("SealedHeaven").join("checkpoint.c351")).unwrap();
    (result, checkpoint)
}

#[test]
fn changing_external_catalog_changes_output_but_not_a_single_checkpoint_byte() {
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("pack");
    fs::create_dir(&pack).unwrap();
    fs::write(
        pack.join("living_id.json"),
        r#"{"living.save.ok":"C36-SAVE-TERSIMPAN","living.load.ok":"C36-LOAD-PULIH"}"#,
    )
    .unwrap();
    let commands = "lang id;seed 1;east;wait;save;reset;load;wait;save;quit";
    let (normal, before) = run(&root.path().join("normal"), None, commands);
    let (translated, after) = run(&root.path().join("external"), Some(&pack), commands);
    let output = String::from_utf8(translated.stdout).unwrap();
    assert!(output.contains("C36-SAVE-TERSIMPAN"));
    assert!(output.contains("C36-LOAD-PULIH"));
    assert!(
        output.contains("Started a new settlement."),
        "missing localized key must fall back to English"
    );
    assert!(
        String::from_utf8(normal.stdout)
            .unwrap()
            .contains("Desa tersimpan.")
    );
    assert!(translated.stderr.is_empty());
    assert_eq!(before, after);
    assert_eq!(u16::from_le_bytes(after[12..14].try_into().unwrap()), 26);
    assert_eq!(u32::from_le_bytes(after[46..50].try_into().unwrap()), 3);
}

#[test]
fn malformed_catalog_is_diagnosed_and_embedded_fallback_keeps_valid_saves() {
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("bad");
    fs::create_dir(&pack).unwrap();
    fs::write(
        pack.join("living_id.json"),
        r#"{"living.save.ok":"SHOULD-NOT-PUBLISH","living.save.error":"lost placeholder"}"#,
    )
    .unwrap();
    let commands = "lang id;wait;save;quit";
    let (_, expected) = run(&root.path().join("normal"), None, commands);
    let (invalid, actual) = run(&root.path().join("invalid"), Some(&pack), commands);
    let stdout = String::from_utf8(invalid.stdout).unwrap();
    let stderr = String::from_utf8(invalid.stderr).unwrap();
    assert!(stderr.contains("Invalid living locale pack"));
    assert!(stderr.contains("parameter contract mismatch"));
    assert!(stdout.contains("Desa tersimpan."));
    assert!(!stdout.contains("SHOULD-NOT-PUBLISH"));
    assert_eq!(expected, actual);
}
