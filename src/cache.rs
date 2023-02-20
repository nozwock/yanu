use anyhow::Result;
use std::{fmt, fs, io::Write, path::PathBuf};

use crate::defines::{app_cache_dir, HACPACK, HACTOOL};

#[derive(Debug)]
pub enum CacheEmbedded {
    Hacpack,
    Hactool,
}

impl fmt::Display for CacheEmbedded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "windows")]
            CacheEmbedded::Hacpack => write!(f, "hacpack.exe"),
            #[cfg(target_os = "windows")]
            CacheEmbedded::Hactool => write!(f, "hactool.exe"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            CacheEmbedded::Hacpack => write!(f, "hacpack"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            CacheEmbedded::Hactool => write!(f, "hactool"),
        }
    }
}

impl CacheEmbedded {
    /// Returns the path to the embedded resource.
    ///
    /// Cache is used if it exists else the embedded data is written to a file
    /// and the path is returned.
    pub fn load(self) -> Result<PathBuf> {
        let cache_dir = app_cache_dir();
        fs::create_dir_all(&cache_dir)?;

        let file_name = self.to_string();
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy() == file_name {
                // return cache if exists
                return Ok(entry.path());
            }
        }

        let path = cache_dir.join(file_name);
        let mut file = fs::File::create(&path)?;
        file.write_all(self.get_item())?;

        Ok(path)
    }
    fn get_item(self) -> &'static [u8] {
        match self {
            CacheEmbedded::Hacpack => HACPACK,
            CacheEmbedded::Hactool => HACTOOL,
        }
    }
}
