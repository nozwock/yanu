use anyhow::Result;
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
    use anyhow::bail;
    use std::fs;
    use tracing::info;

    info!("Building hacPack");
    let src_dir = TempDir::new("hacpack")?.into_path();

    if !Command::new("git")
        .args(["clone", "https://github.com/The-4n/hacPack"])
        .arg(&src_dir)
        .status()?
        .success()
    {
        bail!("failed to clone hacPack repo");
    }

    info!("Renaming config file");
    fs::rename(
        src_dir.join("config.mk.template"),
        src_dir.join("config.mk"),
    )?;

    info!("Running make");
    if !Command::new("make")
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("failed to build hacPack");
    }

    Ok(src_dir.join("hacpack"))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn make_hactool() -> Result<PathBuf> {
    use anyhow::bail;
    use std::fs;
    use tracing::info;

    info!("Building hactool");
    let src_dir = TempDir::new("hactool")?.into_path();

    if !Command::new("git")
        .args(["clone", "https://github.com/SciresM/hactool"])
        .arg(&src_dir)
        .status()?
        .success()
    {
        bail!("failed to clone hactool repo");
    }

    info!("Renaming config file");
    fs::rename(
        src_dir.join("config.mk.template"),
        src_dir.join("config.mk"),
    )?;

    // removing line 372 as it causes build to fail on android
    #[cfg(target_os = "android")]
    {
        use std::io::{BufRead, BufReader};

        info!("Removing line 372 from `main.c`");
        let reader = BufReader::new(fs::File::open(src_dir.join("main.c"))?);
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
        fs::write(src_dir.join("main.c"), fixed_main.as_bytes())?;
    }

    info!("Running make");
    if !Command::new("make")
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("failed to build hactool");
    }

    Ok(src_dir.join("hactool"))
}
