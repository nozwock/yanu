use crate::{defines::APP_CACHE_DIR, utils::move_file};
use eyre::{bail, Result};
use fs_err as fs;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct Cache {}

impl Cache {
    pub fn store_path<P: AsRef<Path>>(from: P) -> Result<PathBuf> {
        Ok(Self::store_path_in(from.as_ref(), APP_CACHE_DIR.as_path())?)
    }
    pub fn store_bytes(slice: &[u8], filename: &str) -> Result<PathBuf> {
        Ok(Self::store_bytes_as(slice, APP_CACHE_DIR.join(filename))?)
    }
    pub fn store_path_in<P: AsRef<Path>, Q: AsRef<Path>>(from: P, dir: Q) -> Result<PathBuf> {
        info!(dir = ?dir.as_ref(), "Caching {:?}", from.as_ref());
        fs::create_dir_all(dir.as_ref())?;
        let dest = dir.as_ref().join(
            from.as_ref()
                .file_name()
                .ok_or_else(|| eyre::eyre!("Failed to get filename of {:?}", from.as_ref()))?,
        );
        if from.as_ref() != dest {
            move_file(from.as_ref(), &dest)?;
        }
        Ok(dest)
    }
    pub fn store_bytes_as<P: AsRef<Path>>(slice: &[u8], path: P) -> Result<PathBuf> {
        info!(path = ?path.as_ref(), "Storing given bytes");
        fs::create_dir_all(
            path.as_ref()
                .parent()
                .ok_or_else(|| eyre::eyre!("Failed to find parent of {:?}", path.as_ref()))?,
        )?;
        let mut file = fs::File::create(path.as_ref())?;
        file.write_all(slice)?;
        Ok(file.path().into())
    }
    pub fn path(filename: &str) -> Result<PathBuf> {
        for entry in walkdir::WalkDir::new(APP_CACHE_DIR.as_path())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == filename {
                return Ok(entry.path().into());
            }
        }
        bail!("Failed to find {:?} in cache", filename);
    }
    pub fn is_cached(filename: &str) -> bool {
        Self::path(filename).is_ok()
    }
}
