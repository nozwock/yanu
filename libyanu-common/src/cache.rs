use crate::{defines::APP_CACHE_DIR, utils::move_file};
use eyre::{bail, Result};
use fs_err as fs;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub struct Cache<'a> {
    pub dir: &'a Path,
}

impl Default for Cache<'_> {
    fn default() -> Self {
        Self {
            dir: APP_CACHE_DIR.as_path(),
        }
    }
}

impl Cache<'_> {
    /// Moves the file pointed by the given `file_path` to the cache dir.
    pub fn store_path<P: AsRef<Path>>(&self, file_path: P) -> Result<PathBuf> {
        info!(dir = ?self.dir, "Caching \"{}\"", file_path.as_ref().display());
        fs::create_dir_all(self.dir)?;
        let dst = self.dir.join(file_path.as_ref().file_name().ok_or_else(|| {
            eyre::eyre!(
                "Failed to get filename of \"{}\"",
                file_path.as_ref().display()
            )
        })?);
        if file_path.as_ref() != dst {
            move_file(file_path.as_ref(), &dst)?;
        }
        Ok(dst)
    }
    /// Stores the given `slice` in the cache dir with `filename`.
    pub fn store_bytes(&self, slice: &[u8], filename: &str) -> Result<PathBuf> {
        let dst = self.dir.join(filename);
        info!(to = ?dst, "Storing given bytes");
        fs::create_dir_all(self.dir)?;
        let mut file = fs::File::create(dst)?;
        file.write_all(slice)?;
        Ok(file.path().into())
    }
    /// Looks for a file with `filename` in the cache dir and returns its path.
    pub fn path(&self, filename: &str) -> Result<PathBuf> {
        for entry in walkdir::WalkDir::new(self.dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == filename {
                return Ok(entry.path().into());
            }
        }
        bail!("Failed to find \"{}\" in cache", filename);
    }
}
