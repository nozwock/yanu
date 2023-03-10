use eyre::Result;
use std::path::{Path, PathBuf};
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "android"))]
use tempdir::TempDir;

use crate::cache::Cache;

#[derive(Debug, Clone, Copy)]
pub enum BackendKind {
    Hacpack,
    Hactool,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    Hactoolnet,
}

pub struct Backend {
    kind: BackendKind,
    path: PathBuf,
}

impl Backend {
    pub const HACPACK: BackendKind = BackendKind::Hacpack;
    pub const HACTOOL: BackendKind = BackendKind::Hactool;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub const HACTOOLNET: BackendKind = BackendKind::Hactoolnet;

    pub fn new(kind: BackendKind) -> Result<Self> {
        let tool = Backend::map_to_cache(kind);
        let path = if tool.is_cached() {
            tool.path()?
        } else {
            #[cfg(target_os = "windows")]
            {
                tool.from_embed()?.path()?
            }
            #[cfg(any(target_os = "linux", target_os = "android"))]
            {
                match tool {
                    Cache::Hacpack => tool.from(make_hacpack()?)?.make_executable()?.path()?,
                    Cache::Hactool => tool.from(make_hactool()?)?.make_executable()?.path()?,
                    #[cfg(target_os = "linux")]
                    Cache::Hactoolnet => tool.from_embed()?.make_executable()?.path()?,
                }
            }
        };

        Ok(Self { kind, path })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn kind(&self) -> BackendKind {
        self.kind
    }
    //* there's prob a better way to do this mapping
    fn map_to_cache(tool: BackendKind) -> Cache {
        match tool {
            BackendKind::Hacpack => Cache::Hacpack,
            BackendKind::Hactool => Cache::Hactool,
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            BackendKind::Hactoolnet => Cache::Hactoolnet,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn make_hacpack() -> Result<PathBuf> {
    use crate::{defines::APP_CACHE_DIR, utils::move_file};
    use eyre::bail;
    use std::fs;
    use tracing::info;

    let name = format!("{:?}", Backend::HACPACK).to_lowercase();
    info!("Building {}", name);
    let src_dir = TempDir::new(&name)?;

    if !Command::new("git")
        .args(["clone", "https://github.com/The-4n/hacPack"])
        .arg(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to clone {} repo", name);
    }

    info!("Renaming config file");
    fs::rename(
        src_dir.path().join("config.mk.template"),
        src_dir.path().join("config.mk"),
    )?;

    info!("Running make");
    if !Command::new("make")
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("Failed to build {}", name);
    }

    //* Moving bin from temp dir to cache dir
    let dest = APP_CACHE_DIR.join(&name);
    move_file(src_dir.path().join(&name), &dest)?;

    Ok(dest)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn make_hactool() -> Result<PathBuf> {
    use crate::{defines::APP_CACHE_DIR, utils::move_file};
    use eyre::bail;
    use std::fs;
    use tracing::info;

    let name = format!("{:?}", Backend::HACTOOL).to_lowercase();
    info!("Building {}", name);
    let src_dir = TempDir::new(&name)?;

    if !Command::new("git")
        .args(["clone", "https://github.com/SciresM/hactool"])
        .arg(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to clone {} repo", name);
    }

    info!("Renaming config file");
    fs::rename(
        src_dir.path().join("config.mk.template"),
        src_dir.path().join("config.mk"),
    )?;

    // removing line 372 as it causes build to fail on android
    #[cfg(target_os = "android")]
    {
        use std::io::{BufRead, BufReader};

        info!("Removing line 372 from `main.c`");
        let reader = BufReader::new(fs::File::open(src_dir.path().join("main.c"))?);
        //* can't use advance_by yet
        let fixed_main = reader
            .lines()
            .enumerate()
            .filter_map(|(i, ln)| {
                if i != 371 {
                    // i.e ln 372
                    return Some(ln);
                }
                None
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("\n");
        fs::write(src_dir.path().join("main.c"), fixed_main.as_bytes())?;
    }

    info!("Running make");
    if !Command::new("make")
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("Failed to build {}", name);
    }

    //* Moving bin from temp dir to cache dir
    let dest = APP_CACHE_DIR.join(&name);
    move_file(src_dir.path().join(&name), &dest)?;

    Ok(dest)
}
