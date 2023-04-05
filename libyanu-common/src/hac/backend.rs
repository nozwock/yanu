use eyre::Result;
#[cfg(unix)]
use once_cell::sync::Lazy;
#[cfg(all(target_arch = "x86_64", unix))]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use tempfile::tempdir;

#[cfg(target_family = "unix")]
use crate::utils::set_executable_bit;
use crate::{cache::Cache, defines};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Hacpack,
    Hactool,
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    Hactoolnet,
    Hac2l,
}

impl BackendKind {
    fn to_filename(&self) -> String {
        #[cfg(unix)]
        {
            format!("{:?}", self).to_lowercase()
        }
        #[cfg(windows)]
        {
            format!("{:?}.exe", self).to_lowercase()
        }
    }
}

pub struct Backend {
    kind: BackendKind,
    path: PathBuf,
}

impl Backend {
    pub fn new(kind: BackendKind) -> Result<Self> {
        let filename = kind.to_filename();
        let cache = Cache::default();
        let path = if let Ok(cached_path) = cache.path(&filename) {
            cached_path
        } else {
            #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
            {
                match kind {
                    BackendKind::Hacpack => cache.store_bytes(defines::HACPACK, &filename)?,
                    BackendKind::Hactool => cache.store_bytes(defines::HACTOOL, &filename)?,
                    BackendKind::Hactoolnet => cache.store_bytes(defines::HACTOOLNET, &filename)?,
                    BackendKind::Hac2l => cache.store_bytes(defines::HAC2L, &filename)?,
                }
            }
            #[cfg(unix)]
            {
                let cached_path = match kind {
                    BackendKind::Hacpack => cache.store_path(make_hacpack()?)?,
                    BackendKind::Hactool => cache.store_path(make_hactool()?)?,
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hactoolnet => cache.store_bytes(defines::HACTOOLNET, &filename)?,
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hac2l => cache.store_path(make_hac2l(["linux_x64_release"])?)?,
                    #[cfg(feature = "android-proot")]
                    BackendKind::Hac2l => cache.store_bytes(defines::HAC2L, &filename)?,
                };
                set_executable_bit(&cached_path, true)?;
                cached_path
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
}

#[cfg(unix)]
static NPROC: Lazy<Result<u8>> = Lazy::new(|| {
    Ok(String::from_utf8(Command::new("nproc").output()?.stdout)?
        .trim()
        .parse()?)
});

#[cfg(unix)]
pub fn make_hacpack() -> Result<PathBuf> {
    use crate::{config::Config, defines::APP_CACHE_DIR, utils::move_file};
    use eyre::{bail, eyre};
    use fs_err as fs;
    use tracing::info;

    let config = Config::load()?;
    let name = format!("{:?}", BackendKind::Hacpack).to_lowercase();
    info!("Building {}", name);
    let src_dir = tempdir()?;

    if !Command::new("git")
        .args(["clone", "https://github.com/The-4n/hacPack"])
        .arg(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to clone {} repo", name);
    }

    if !Command::new("git")
        .args(["checkout", &config.hacpack_rev])
        .current_dir(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to checkout");
    }

    info!("Renaming config file");
    fs::rename(
        src_dir.path().join("config.mk.template"),
        src_dir.path().join("config.mk"),
    )?;

    info!("Running make");
    if !Command::new("make")
        .args([
            "-j",
            &(NPROC.as_ref().map_err(|err| eyre!(err))? / 2).to_string(),
        ])
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("Failed to build {}", name);
    }

    //* Moving bin from temp dir to cache dir
    let filename = BackendKind::Hacpack.to_filename();
    fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
    let dest = APP_CACHE_DIR.join(&filename);
    move_file(src_dir.path().join(&filename), &dest)?;

    Ok(dest)
}

#[cfg(unix)]
pub fn make_hactool() -> Result<PathBuf> {
    use crate::{config::Config, defines::APP_CACHE_DIR, utils::move_file};
    use eyre::{bail, eyre};
    use fs_err as fs;
    use tracing::info;

    let config = Config::load()?;
    let name = format!("{:?}", BackendKind::Hactool).to_lowercase();
    info!("Building {}", name);
    let src_dir = tempdir()?;

    if !Command::new("git")
        .args(["clone", "https://github.com/SciresM/hactool"])
        .arg(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to clone {} repo", name);
    }

    if !Command::new("git")
        .args(["checkout", &config.hactool_rev])
        .current_dir(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to checkout");
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
        .args([
            "-j",
            &(NPROC.as_ref().map_err(|err| eyre!(err))? / 2).to_string(),
        ])
        .current_dir(&src_dir)
        .status()?
        .success()
    {
        bail!("Failed to build {}", name);
    }

    //* Moving bin from temp dir to cache dir
    let filename = BackendKind::Hactool.to_filename();
    fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
    let dest = APP_CACHE_DIR.join(&filename);
    move_file(src_dir.path().join(&filename), &dest)?;

    Ok(dest)
}

#[cfg(all(target_arch = "x86_64", unix))]
pub fn make_hac2l<I, S>(args: I) -> Result<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    use crate::{config::Config, defines::APP_CACHE_DIR, utils::move_file};
    use eyre::{bail, eyre};
    use tracing::{debug, info};

    let config = Config::load()?;

    let name = format!("{:?}", BackendKind::Hac2l).to_lowercase();
    info!("Building {}", name);
    let src_dir = tempdir()?;

    if !Command::new("git")
        .args(["clone", "https://github.com/Atmosphere-NX/Atmosphere.git"])
        .arg(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to clone Atmosphere repo");
    }

    if !Command::new("git")
        .args(["checkout", &config.atmosphere_rev])
        .current_dir(src_dir.path())
        .status()?
        .success()
    {
        bail!("Failed to checkout");
    }

    let hac2l_src_dir = src_dir.path().join("tools/hac2l");
    if !Command::new("git")
        .args(["clone", "https://github.com/Atmosphere-NX/hac2l.git"])
        .arg(&hac2l_src_dir)
        .status()?
        .success()
    {
        bail!("Failed to clone {} repo", name);
    }

    if !Command::new("git")
        .args(["checkout", &config.hac2l_rev])
        .current_dir(&hac2l_src_dir)
        .status()?
        .success()
    {
        bail!("Failed to checkout");
    }

    info!("Running make");

    if !Command::new("make")
        .args([
            "-j",
            &(NPROC.as_ref().map_err(|err| eyre!(err))? / 2).to_string(),
        ])
        .args(args)
        .current_dir(&hac2l_src_dir)
        .status()?
        .success()
    {
        bail!("Failed to build {}", name);
    }

    //* Moving bin from temp dir to cache dir
    let filename = BackendKind::Hac2l.to_filename();
    fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
    let dest = APP_CACHE_DIR.join(&filename);
    for entry in walkdir::WalkDir::new(hac2l_src_dir.join("out"))
        .min_depth(1)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        debug!(
            ?entry,
            is_file = entry.file_type().is_file(),
            parent_is_release = entry.path().parent().unwrap().ends_with("release")
        );
        if entry.file_type().is_file() && entry.path().parent().unwrap().ends_with("release") {
            move_file(entry.path(), &dest)?;
            return Ok(dest);
        }
    }

    bail!("Failed to build {}", name);
}
