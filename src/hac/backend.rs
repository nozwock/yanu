use eyre::Result;
use std::path::PathBuf;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::process::Command;
#[cfg(any(target_os = "linux", target_os = "android"))]
use tempdir::TempDir;

use crate::cache::Cache;

#[derive(Debug, Clone, Copy)]
pub enum Backend {
    Hacpack,
    Hactool,
}

impl Backend {
    pub fn path(&self) -> Result<PathBuf> {
        match self {
            Backend::Hacpack => {
                let hacpack = Cache::Hacpack;
                #[cfg(target_os = "windows")]
                {
                    if hacpack.is_cached() {
                        return Ok(hacpack.path()?);
                    } else {
                        return Ok(hacpack.from_embed()?.path()?);
                    }
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    if hacpack.is_cached() {
                        return Ok(hacpack.path()?);
                    } else {
                        return Ok(hacpack.from(make_hacpack()?)?.make_executable()?.path()?);
                    }
                }
            }
            Backend::Hactool => {
                let hactool = Cache::Hactool;
                #[cfg(target_os = "windows")]
                {
                    if hactool.is_cached() {
                        return Ok(hactool.path()?);
                    } else {
                        return Ok(hactool.from_embed()?.path()?);
                    }
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    if hactool.is_cached() {
                        return Ok(hactool.path()?);
                    } else {
                        return Ok(hactool.from(make_hactool()?)?.make_executable()?.path()?);
                    }
                }
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn make_hacpack() -> Result<PathBuf> {
    use crate::{defines::app_cache_dir, utils::move_file};
    use eyre::bail;
    use std::fs;
    use tracing::info;

    let name = format!("{:?}", Backend::Hacpack).to_lowercase();
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
    let dest = app_cache_dir().join(&name);
    move_file(src_dir.path().join(&name), &dest)?;

    Ok(dest)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn make_hactool() -> Result<PathBuf> {
    use crate::{defines::app_cache_dir, utils::move_file};
    use eyre::bail;
    use std::fs;
    use tracing::info;

    let name = format!("{:?}", Backend::Hactool).to_lowercase();
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
    let dest = app_cache_dir().join(&name);
    move_file(src_dir.path().join(&name), &dest)?;

    Ok(dest)
}
