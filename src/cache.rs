use anyhow::{bail, Result};
use std::{
    fmt, fs,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::debug;

use crate::defines::app_cache_dir;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::defines::{HACPACK, HACTOOL};

#[derive(Debug, Clone, Copy)]
pub enum Cache {
    Hacpack,
    Hactool,
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "windows")]
            Cache::Hacpack => write!(f, "hacpack.exe"),
            #[cfg(target_os = "windows")]
            Cache::Hactool => write!(f, "hactool.exe"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Cache::Hacpack => write!(f, "hacpack"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Cache::Hactool => write!(f, "hactool"),
        }
    }
}

impl Cache {
    /// Saves the given file as a cache for `self`.
    ///
    /// Overwrited the previous cache in the process if any.
    pub fn from<P: AsRef<Path>>(self, path: P) -> Result<Self> {
        debug!("Copying {:?} as cache for {:?}", path.as_ref(), self);

        let cache_dir = app_cache_dir();
        fs::create_dir_all(&cache_dir)?;
        fs::copy(path.as_ref(), cache_dir.join(self.to_string()))?;

        Ok(self)
    }
    /// Extracts the embedded files to the cache dir
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn from_embed(self) -> Result<Self> {
        let cache_dir = app_cache_dir();
        fs::create_dir_all(&cache_dir)?;
        let mut file = fs::File::create(cache_dir.join(self.to_string()))?;
        file.write_all(self.as_bytes())?;

        Ok(self)
    }
    /// Returns the path to the embedded resource.
    ///
    /// Cache is used if it exists else the embedded data is written to a file
    /// and the path is returned.
    pub fn path(&self) -> Result<PathBuf> {
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

        bail!("failed to find {:?} in cache", self);
    }
    /// chmod +x
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pub fn make_executable(self) -> Result<Self> {
        use std::process::Command;

        let cache_dir = app_cache_dir();
        fs::create_dir_all(&cache_dir)?;

        let file_path = cache_dir.join(self.to_string());
        if self.is_cached() {
            if Command::new("chmod")
                .arg("+x")
                .arg(&file_path)
                .status()?
                .success()
            {
                return Ok(self);
            }
        }

        bail!("failed to give executable permission to {:?}", file_path);
    }
    pub fn is_cached(&self) -> bool {
        if self._exists().is_ok() {
            return true;
        }
        false
    }
    fn _exists(&self) -> Result<()> {
        let cache_dir = app_cache_dir();
        fs::create_dir_all(&cache_dir)?;

        let file_name = self.to_string();
        for entry in fs::read_dir(&cache_dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy() == file_name {
                return Ok(());
            }
        }

        bail!("{:?} isn't cached", file_name);
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn as_bytes(&self) -> &'static [u8] {
        match self {
            Cache::Hacpack => HACPACK,
            Cache::Hactool => HACTOOL,
        }
    }
}
