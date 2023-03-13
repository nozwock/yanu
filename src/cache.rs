use crate::{defines::APP_CACHE_DIR, utils::move_file};
use eyre::{bail, Result};
use fs_err as fs;
use std::{
    fmt,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::info;

#[derive(Debug, Clone, Copy)]
pub enum Cache {
    Hacpack,
    Hactool,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    Hactoolnet,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    Hac2l,
}

impl fmt::Display for Cache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(target_os = "windows")]
            Cache::Hacpack => write!(f, "hacpack.exe"),
            #[cfg(target_os = "windows")]
            Cache::Hactool => write!(f, "hactool.exe"),
            #[cfg(target_os = "windows")]
            Cache::Hactoolnet => write!(f, "hactoolnet.exe"),
            #[cfg(target_os = "windows")]
            Cache::Hac2l => write!(f, "hac2l.exe"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Cache::Hacpack => write!(f, "hacpack"),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Cache::Hactool => write!(f, "hactool"),
            #[cfg(target_os = "linux")]
            Cache::Hactoolnet => write!(f, "hactoolnet"),
            #[cfg(target_os = "linux")]
            Cache::Hac2l => write!(f, "hac2l"),
        }
    }
}

impl Cache {
    pub fn new<P: AsRef<Path>>(self, path: P) -> Result<Self> {
        info!(?self, "Caching {:?}", path.as_ref());

        let cache_dir = APP_CACHE_DIR.as_path();
        fs::create_dir_all(cache_dir)?;
        let dest = cache_dir.join(self.to_string());
        if path.as_ref() != dest {
            move_file(path.as_ref(), dest)?;
        }

        Ok(self)
    }
    pub fn from_bytes(self, slice: &[u8]) -> Result<Self> {
        info!(?self, "Caching from given bytes");

        let cache_dir = APP_CACHE_DIR.as_path();
        fs::create_dir_all(cache_dir)?;
        let mut file = fs::File::create(cache_dir.join(self.to_string()))?;
        file.write_all(slice)?;

        Ok(self)
    }
    pub fn path(&self) -> Result<PathBuf> {
        let file_name = self.to_string();
        for entry in walkdir::WalkDir::new(APP_CACHE_DIR.as_path())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == file_name.as_str() {
                return Ok(entry.path().into());
            }
        }
        bail!("Failed to find {:?} in cache", self);
    }
    pub fn is_cached(&self) -> bool {
        self.path().is_ok()
    }
    #[cfg(target_family = "unix")]
    pub fn with_executable_bit(self, on: bool) -> Result<Self> {
        use crate::utils::set_executable_bit;
        set_executable_bit(self.path()?, on)?;
        Ok(self)
    }
}
