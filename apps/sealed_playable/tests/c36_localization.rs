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

#[test]
fn adjacent_pack_supports_named_reordering_without_environment_override() {
    let root = tempfile::tempdir().unwrap();
    let exe = root.path().join(if cfg!(windows) {
        "sealed_agency.exe"
    } else {
        "sealed_agency"
    });
    fs::copy(env!("CARGO_BIN_EXE_sealed_agency"), &exe).unwrap();
    let pack = root.path().join("locales");
    fs::create_dir(&pack).unwrap();
    fs::write(pack.join("living_id.json"),r#"{
        "living.ascii.frame.template.scenario_name_day_clock_day_number":
        "C36-FRAME HP={player_hp} T={world_tick} H={player_herbs} {scenario_name} {hour}:{minute} D={clock_day_number_world_calendar_tick}{conditional}",
        "living.save.ok":"C36-SAVED"
    }"#).unwrap();
    let commands = "lang id;seed 7;wait;east;save;quit";
    let (_, before) = run(&root.path().join("baseline"), None, commands);
    let save = root.path().join("adjacent");
    let result = Command::new(exe)
        .env_remove("SEALED_LOCALE_DIR")
        .env("LOCALAPPDATA", &save)
        .env("SEALED_ASCII_COMMANDS", commands)
        .output()
        .unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("C36-FRAME HP="));
    assert!(text.contains(" T=2 H="));
    assert!(text.contains("C36-SAVED"));
    assert!(result.stderr.is_empty(), "{:?}", result.stderr);
    assert_eq!(
        fs::read(save.join("SealedHeaven/checkpoint.c351")).unwrap(),
        before
    );
}

#[test]
fn legacy_c09_catalog_overrides_are_runtime_only() {
    let root = tempfile::tempdir().unwrap();
    let pack = root.path().join("legacy");
    fs::create_dir(&pack).unwrap();
    fs::write(
        pack.join("ui_id.json"),
        r#"{"c09.healer_attitude":"C36-SIKAP","legacy.ascii.tick":"C36-TICK"}"#,
    )
    .unwrap();
    let invoke = |name: &str, path: Option<&Path>| {
        let save = root.path().join(name);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sealed_agency"));
        cmd.arg("--c09")
            .env("LOCALAPPDATA", &save)
            .env("SEALED_ASCII_COMMANDS", "t l n v q")
            .env_remove("SEALED_LOCALE_DIR");
        if let Some(path) = path {
            cmd.env("SEALED_LOCALE_DIR", path);
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success());
        let dir = save.join("SealedHeaven");
        let saves = fs::read_dir(dir)
            .unwrap()
            .map(|x| x.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| (p.file_name().unwrap().to_owned(), fs::read(&p).unwrap()))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert!(!saves.is_empty());
        (String::from_utf8(out.stdout).unwrap(), saves)
    };
    let (normal, expected) = invoke("normal", None);
    let (changed, actual) = invoke("override", Some(&pack));
    assert!(normal.contains("Sikap tabib"));
    assert!(changed.contains("C36-SIKAP"));
    assert!(changed.contains("C36-TICK"));
    assert_eq!(expected, actual);
}
