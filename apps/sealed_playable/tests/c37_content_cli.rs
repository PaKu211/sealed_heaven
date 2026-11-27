//! Executable-level content identity query must not create a world or save.
use std::process::Command;
#[test]
fn content_info_is_a_read_only_query_and_conflicting_flags_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let save = dir.path().join("save-root");
    let invoke = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_sealed_agency"))
            .args(args)
            .env("LOCALAPPDATA", &save)
            .env("SEALED_ASCII_COMMANDS", "wait;save;quit")
            .env_remove("SEALED_LOCALE_DIR")
            .output()
            .unwrap()
    };
    let out = invoke(&["--content-info"]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let manifest: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        manifest["content_sha256"],
        sealed_sim::living::content::CONTENT_SHA256
    );
    assert_eq!(manifest["definitions"].as_array().unwrap().len(), 43);
    assert_eq!(
        manifest["pack_order"],
        serde_json::json!(["core", "sealed_heaven"])
    );
    assert!(!save.exists());
    for flag in ["--pixel", "--c35", "--debug-world"] {
        let out = invoke(&["--content-info", flag]);
        assert_eq!(out.status.code(), Some(2));
        assert!(!save.exists());
    }
}
