use sealed_sim::living::{
    Intent, World,
    road_history::{Method, REVISION},
};
use std::{
    path::Path,
    process::{Command, Output},
};
#[path = "support/c45_trace.rs"]
mod trace;
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
    std::fs::read(root.join("SealedHeaven/checkpoint.c45")).unwrap()
}

#[test]
fn real_cli_encounters_are_bilingual_witness_only_and_support_both_choices() {
    let root = tempfile::tempdir().unwrap();
    for (lang, entrust) in [("en", false), ("id", false), ("en", true), ("id", true)] {
        let (mut expected, commands) = trace::trace(entrust);
        let seed = expected.region().unwrap().seed.to_string();
        let action = if entrust {
            "entrustcache"
        } else {
            "recovercache"
        };
        let dir = root.path().join(format!("{lang}-{entrust}"));
        let p = run(
            &dir,
            &["--c45", "--seed", &seed],
            &format!(
                "lang {lang};history;encounters;historylog;{};encounters;history;{action};encounters;history;save;reset;load;history;save;quit",
                commands.join(";")
            ),
        );
        assert!(p.status.success() && p.stderr.is_empty(), "{:?}", p.stderr);
        let out = String::from_utf8(p.stdout).unwrap();
        assert!(out.contains(if lang == "en" {
            "No historical deposits discovered yet."
        } else {
            "Belum ada simpanan bersejarah yang ditemukan."
        }));
        assert!(out.contains(if lang == "en" {
            "Cache #1, Willowford to Qinghe"
        } else {
            "Simpanan #1, Willowford ke Qinghe"
        }));
        assert!(out.contains(if lang == "en" {
            "The cache is empty."
        } else {
            "Simpanan ini kosong."
        }));
        assert!(!out.contains("Deposit {") && !out.contains("witnessed_resolution:"));
        assert!(!out.contains("Cache #2,") && !out.contains("Simpanan #2,"));
        if entrust {
            assert!(
                out.contains(if lang == "en" {
                    "entrusted to Bao Jun"
                } else {
                    "diserahkan kepada Bao Jun"
                }),
                "{out}"
            );
        } else {
            assert!(out.contains(if lang == "en" {
                "took cache #1"
            } else {
                "mengambil simpanan #1"
            }));
        }
        expected
            .step(
                expected.tick(),
                if entrust {
                    Intent::EntrustCache(1)
                } else {
                    Intent::RecoverCache(1)
                },
            )
            .unwrap();
        let method = if entrust {
            Method::Entrusted
        } else {
            Method::Taken
        };
        assert_eq!(expected.known_cache_resolution(1).unwrap().method, method);
        assert_eq!(save(&dir), expected.checkpoint());
    }
    let dir = root.path().join("dev");
    let p = run(&dir, &["--c45", "--debug-world"], "historylog;save;quit");
    assert!(p.status.success() && p.stderr.is_empty());
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Deposit { id: 1") && out.contains("Deposit { id: 2"));
    assert_eq!(save(&dir), World::road_history_campaign(0).checkpoint());
}

#[test]
fn save_revision_and_strict_launch_remain_isolated() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("SealedHeaven");
    std::fs::create_dir(&dir).unwrap();
    let old = World::settlement_production_campaign(7).checkpoint();
    let path = dir.join("checkpoint.c44");
    std::fs::write(&path, &old).unwrap();
    let p = run(
        root.path(),
        &["--c45"],
        "seed 42;wait;history;encounters;save;quit",
    );
    assert!(p.status.success() && p.stderr.is_empty());
    let w = World::restore(&save(root.path())).unwrap();
    assert_eq!(w.checkpoint_revision(), REVISION);
    assert_eq!(w.region().unwrap().seed, 42);
    assert_eq!(w.tick(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), old);
    std::fs::write(dir.join("checkpoint.c45"), &old).unwrap();
    let p = run(root.path(), &["--c45"], "load;save;quit");
    let out = String::from_utf8(p.stdout).unwrap();
    assert!(out.contains("Load failed") && out.contains("Save failed"));
    assert_eq!(save(root.path()), old);
    for args in [
        vec!["--c45", "--c44"],
        vec!["--c45", "--c45"],
        vec!["--c45", "--seed"],
        vec!["--c45", "--seed", "4294967296"],
        vec!["--c45", "--pixel", "--debug-world"],
    ] {
        let isolated = root.path().join("invalid");
        let p = run(&isolated, &args, "wait;save;quit");
        assert_eq!(p.status.code(), Some(2), "{args:?}");
        assert!(!isolated.exists());
    }
}
