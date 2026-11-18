//! Exact canonical checkpoint persistence; this module never replaces live state.
//!
//! The default location is LOCALAPPDATA/SealedHeaven/checkpoint.c03, falling back
//! to HOME/.local/share/SealedHeaven/checkpoint.c03, then the OS temporary
//! directory's SealedHeaven/checkpoint.c03. Empty/relative environment paths are
//! ignored. No save is written into the application's fixture/source directory.
//!
//! Replacement is atomic where supported by the filesystem. File contents are
//! synced before replacement, but the directory entry is not synced: this is not
//! a guarantee of rename durability across power loss. Concurrent saves are
//! last-successful-replacement wins; there is no locking or migration.

use crate::view::{MAX_CHECKPOINT, View};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn default_path() -> PathBuf {
    let absolute_env = |name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let base = absolute_env("LOCALAPPDATA")
        .or_else(|| absolute_env("HOME").map(|home| home.join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("SealedHeaven").join("checkpoint.c03")
}

/// Validate before any filesystem mutation, then atomically replace using a
/// sibling tempfile. A failure never deliberately removes the previous save.
pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_CHECKPOINT {
        return Err("checkpoint.too_large".into());
    }
    View::validated(bytes)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".checkpoint-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|error| error.to_string())?;
    temporary
        .write_all(bytes)
        .map_err(|error| error.to_string())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    // NamedTempFile::persist documents atomic replacement of an existing file:
    // https://docs.rs/tempfile/3.27.0/tempfile/struct.NamedTempFile.html#method.persist
    // Its Windows backend uses MoveFileExW(MOVEFILE_REPLACE_EXISTING), not an
    // unlink-then-rename workaround. A sibling avoids cross-filesystem moves.
    // On error PersistError owns the tempfile; dropping it cleans that up too.
    temporary.persist(path).map_err(|error| error.to_string())?;
    Ok(())
}

/// Return validated candidate bytes only. Session must validate the candidate
/// before committing live state and applying pause/debt/event-log changes.
pub fn load(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_CHECKPOINT as u64 {
        return Err("checkpoint.too_large".into());
    }
    let bytes = read_bounded(file)?;
    View::validated(&bytes)?;
    Ok(bytes)
}

fn read_bounded(reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    // Metadata alone is insufficient: a file may grow after it was inspected.
    reader
        .take(MAX_CHECKPOINT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_CHECKPOINT {
        return Err("checkpoint.too_large".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sealed_sim::{
        ActorId, Command, CommandKey, FixtureConfig, Payload, ProcessingMode, Simulation,
    };

    fn entries(path: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        entries
    }

    fn commands(tick: u64) -> Vec<Command> {
        let payload = match tick {
            5 | 55 => Payload::SetMode(ProcessingMode::Scheduled),
            7 | 57 => Payload::SetMode(ProcessingMode::Active),
            8 | 58 => Payload::SetEnabled(0),
            10 | 60 => Payload::SetEnabled(1),
            _ => return Vec::new(),
        };
        vec![Command::new(
            CommandKey {
                target_tick: tick + 1,
                actor_id: ActorId::ONE,
                producer_id: 1,
                sequence: tick + 1,
            },
            payload,
        )]
    }

    #[test]
    fn c04_t05_save_replace_resume_at_ticks_2_and_50() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested/checkpoint.c03");
        let (mut baseline, _) = View::validated(crate::DEMO).unwrap();
        let mut checkpoints = vec![baseline.domain_checkpoint_bytes()];
        let mut outputs = Vec::new();
        for tick in 0..100 {
            outputs.push(baseline.drive(tick, &commands(tick)).unwrap());
            checkpoints.push(baseline.domain_checkpoint_bytes());
        }
        assert!(!View::validated(&checkpoints[2]).unwrap().1.tasks.is_empty());
        save(&path, crate::DEMO).unwrap();
        for split in [2, 50] {
            save(&path, &checkpoints[split]).unwrap();
            assert_eq!(fs::read(&path).unwrap(), checkpoints[split]);
            let bytes = load(&path).unwrap();
            assert_eq!(bytes, checkpoints[split]);
            let (mut resumed, _) = View::validated(&bytes).unwrap();
            for tick in split..100 {
                assert_eq!(
                    resumed.drive(tick as u64, &commands(tick as u64)).unwrap(),
                    outputs[tick]
                );
                assert_eq!(resumed.domain_checkpoint_bytes(), checkpoints[tick + 1]);
            }
            assert_eq!(entries(path.parent().unwrap()), vec![path.clone()]);
        }
    }

    #[test]
    fn c04_t05_invalid_candidates_preserve_previous_save() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint.c03");
        save(&path, crate::DEMO).unwrap();
        let mut checksum_bad = crate::DEMO.to_vec();
        *checksum_bad.last_mut().unwrap() ^= 1;
        let wrong_fixture = Simulation::new(FixtureConfig {
            seed: 100,
            actor_enabled: [true, true],
        })
        .domain_checkpoint_bytes();
        // This is a valid kernel checkpoint, but not this app's C0.3 fixture.
        assert!(Simulation::restore_domain_checkpoint(&wrong_fixture).is_ok());
        let mut unsupported = crate::DEMO.to_vec();
        unsupported[12..14].copy_from_slice(&u16::MAX.to_le_bytes());
        for candidate in [
            Vec::new(),
            b"not a checkpoint".to_vec(),
            crate::DEMO[..crate::DEMO.len() - 1].to_vec(),
            checksum_bad,
            wrong_fixture,
            unsupported,
            vec![0; MAX_CHECKPOINT + 1],
        ] {
            assert!(save(&path, &candidate).is_err());
            assert_eq!(load(&path).unwrap(), crate::DEMO);
            let invalid_path = directory.path().join("invalid.c03");
            fs::write(&invalid_path, &candidate).unwrap();
            assert!(load(&invalid_path).is_err());
            fs::remove_file(invalid_path).unwrap();
            assert_eq!(fs::read(&path).unwrap(), crate::DEMO);
            assert_eq!(entries(directory.path()), vec![path.clone()]);
        }
        assert!(load(&directory.path().join("missing.c03")).is_err());
        assert_eq!(load(&path).unwrap(), crate::DEMO);
    }

    #[test]
    fn c04_t05_failed_persist_preserves_destination_and_cleans_tempfile() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("destination");
        fs::create_dir(&destination).unwrap();
        let prior = destination.join("checkpoint.c03");
        save(&prior, crate::DEMO).unwrap();
        // A nonempty directory cannot be replaced by a checkpoint file. This
        // exercises persist failure after writing/syncing the sibling tempfile.
        assert!(save(&destination, crate::DEMO).is_err());
        assert_eq!(load(&prior).unwrap(), crate::DEMO);
        assert_eq!(entries(directory.path()), vec![destination.clone()]);
        assert_eq!(entries(&destination), vec![prior.clone()]);
        // Parent directory creation also fails without modifying the prior file.
        assert!(save(&prior.join("blocked.c03"), crate::DEMO).is_err());
        assert_eq!(load(&prior).unwrap(), crate::DEMO);
    }

    #[cfg(windows)]
    #[test]
    fn c04_t05_windows_locked_existing_save_is_preserved() {
        use std::os::windows::fs::OpenOptionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("checkpoint.c03");
        save(&path, crate::DEMO).unwrap();
        // Deny delete sharing so Windows refuses replacement of this exact file.
        let guard = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let (mut sim, _) = View::validated(crate::DEMO).unwrap();
        sim.drive(0, &[]).unwrap();
        let replacement = sim.domain_checkpoint_bytes();
        assert_ne!(replacement, crate::DEMO);
        assert!(save(&path, &replacement).is_err());
        assert_eq!(load(&path).unwrap(), crate::DEMO);
        assert_eq!(entries(directory.path()), vec![path.clone()]);
        drop(guard);
        save(&path, &replacement).unwrap();
        assert_eq!(load(&path).unwrap(), replacement);
    }

    #[test]
    fn c04_t05_read_bound_survives_growth_after_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("growing.c03");
        fs::write(&path, crate::DEMO).unwrap();
        let reader = File::open(&path).unwrap();
        assert!(reader.metadata().unwrap().len() <= MAX_CHECKPOINT as u64);
        let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
        writer.set_len(MAX_CHECKPOINT as u64 + 4096).unwrap();
        assert!(read_bounded(reader).is_err());
        // Even a reader that never reaches EOF is stopped after limit + 1.
        assert!(read_bounded(std::io::repeat(0)).is_err());
        assert_eq!(
            read_bounded(std::io::repeat(0).take(MAX_CHECKPOINT as u64))
                .unwrap()
                .len(),
            MAX_CHECKPOINT
        );
    }

    #[test]
    fn c04_t05_default_path_is_application_specific() {
        assert!(default_path().ends_with(Path::new("SealedHeaven/checkpoint.c03")));
    }
}
