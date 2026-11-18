//! C05 saves use a different path and validator; never implicitly import C03.
use sealed_sim::player5::World;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_SAVE: usize = 16 * 1024 * 1024;

pub fn default_path() -> PathBuf {
    crate::persistence::default_path().with_file_name("checkpoint.c05")
}

pub fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_SAVE {
        return Err("checkpoint.too_large".into());
    }
    World::restore(bytes).map_err(|error| format!("{error:?}"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".c05-")
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
    temporary.persist(path).map_err(|error| error.to_string())?;
    Ok(())
}

pub fn load(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(MAX_SAVE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_SAVE {
        return Err("checkpoint.too_large".into());
    }
    World::restore(&bytes).map_err(|error| format!("{error:?}"))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_is_separate_bounded_and_preserves_prior_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("checkpoint.c05");
        let bytes = World::settlement().checkpoint();
        save(&path, &bytes).unwrap();
        assert_eq!(load(&path).unwrap(), bytes);
        for invalid in [crate::DEMO.to_vec(), vec![], vec![0; MAX_SAVE + 1]] {
            assert!(save(&path, &invalid).is_err());
            assert_eq!(load(&path).unwrap(), bytes);
        }
        assert!(save(&dir.path().join("checkpoint.c05/child"), &bytes).is_err());
        assert_eq!(load(&path).unwrap(), bytes);
        assert!(default_path().ends_with("SealedHeaven/checkpoint.c05"));
    }
}
