use common::defines;
use eyre::Result;
#[cfg(unix)]
use once_cell::sync::Lazy;
#[cfg(all(target_arch = "x86_64", unix))]
use std::ffi::OsStr;
#[cfg(unix)]
use std::process::Command;
use std::{
    fmt,
    path::{Path, PathBuf},
};
#[cfg(unix)]
use tempfile::tempdir;

use cache::{self, Cache};
#[cfg(target_family = "unix")]
use common::utils::set_executable_bit;

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
    FourNXCI,
}

#[cfg(not(feature = "android-proot"))]
impl From<config::NspExtractor> for BackendKind {
    fn from(value: config::NspExtractor) -> Self {
        use config::NspExtractor;
        match value {
            NspExtractor::Hactoolnet => Self::Hactoolnet,
            NspExtractor::Hactool => Self::Hactool,
        }
    }
}

#[cfg(not(feature = "android-proot"))]
impl From<config::NcaExtractor> for BackendKind {
    fn from(value: config::NcaExtractor) -> Self {
        use config::NcaExtractor;
        match value {
            NcaExtractor::Hactoolnet => Self::Hactoolnet,
            NcaExtractor::Hac2l => Self::Hac2l,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl BackendKind {
    // This is important, don't remove it again!
    fn filename(&self) -> String {
        #[cfg(unix)]
        {
            format!("{}", self).to_lowercase()
        }
        #[cfg(windows)]
        {
            format!("{}.exe", self).to_lowercase()
        }
    }
}

pub struct Backend {
    kind: BackendKind,
    path: PathBuf,
}

// TODO?: Some method in which backends are builded no matter what

impl Backend {
    /// Prefers embedded, builds only if binary not available.
    pub fn try_new(kind: BackendKind) -> Result<Self> {
        let filename = kind.filename();
        let cache = Cache::default();
        let cached_path = if let Ok(cached_path) = cache.get(&filename) {
            cached_path
        } else {
            #[cfg(windows)]
            {
                match kind {
                    BackendKind::Hacpack => cache.store_bytes(defines::HACPACK, &filename)?,
                    BackendKind::Hactool => cache.store_bytes(defines::HACTOOL, &filename)?,
                    BackendKind::Hactoolnet => cache.store_bytes(defines::HACTOOLNET, &filename)?,
                    BackendKind::Hac2l => cache.store_bytes(defines::HAC2L, &filename)?,
                    BackendKind::FourNXCI => cache.store_bytes(defines::FOURNXCI, &filename)?,
                }
            }
            #[cfg(unix)]
            {
                let cached_path = match kind {
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hacpack => Backend::build(kind)?.path,
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hactool => Backend::build(kind)?.path,
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hactoolnet => cache.store_bytes(defines::HACTOOLNET, &filename)?,
                    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
                    BackendKind::Hac2l => Backend::build(kind)?.path,
                    #[cfg(feature = "android-proot")]
                    BackendKind::Hacpack => cache.store_bytes(defines::HACPACK, &filename)?,
                    #[cfg(feature = "android-proot")]
                    BackendKind::Hactool => cache.store_bytes(defines::HACTOOL, &filename)?,
                    #[cfg(feature = "android-proot")]
                    BackendKind::Hac2l => cache.store_bytes(defines::HAC2L, &filename)?,
                    BackendKind::FourNXCI => cache.store_bytes(defines::FOURNXCI, &filename)?,
                };
                set_executable_bit(&cached_path, true)?;
                cached_path
            }
        };

        Ok(Self {
            kind,
            path: cached_path,
        })
    }
    #[cfg(unix)]
    /// Opposite of `try_new`.
    pub fn build(kind: BackendKind) -> Result<Self> {
        let cache = Cache::default();
        let cached_path = match kind {
            BackendKind::Hacpack => cache.store_path(build::hacpack()?)?,
            BackendKind::Hactool => cache.store_path(build::hactool()?)?,
            #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
            BackendKind::Hactoolnet => Backend::try_new(kind)?.path,
            #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
            BackendKind::Hac2l => cache.store_path(build::hac2l(["linux_x64_release"])?)?,
            #[cfg(feature = "android-proot")]
            BackendKind::Hac2l => Backend::try_new(kind)?.path,
            BackendKind::FourNXCI => cache.store_path(build::four_nxci()?)?,
        };
        set_executable_bit(&cached_path, true)?;

        Ok(Self {
            kind,
            path: cached_path,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn kind(&self) -> BackendKind {
        self.kind
    }
}

#[cfg(unix)]
pub mod build {
    use common::{defines::APP_CACHE_DIR, utils::move_file};
    use config::Config;
    use eyre::{bail, eyre};
    use fs_err as fs;
    use tracing::info;

    use super::*;

    static NPROC: Lazy<Result<u8>> = Lazy::new(|| {
        Ok(String::from_utf8(Command::new("nproc").output()?.stdout)?
            .trim()
            .parse()?)
    });

    pub fn hacpack() -> Result<PathBuf> {
        let config = Config::load()?;
        let kind = BackendKind::Hacpack;
        info!("Building {}", kind);
        let src_dir = tempdir()?;

        if !Command::new("git")
            .args(["clone", "https://github.com/The-4n/hacPack"])
            .arg(src_dir.path())
            .status()?
            .success()
        {
            bail!("Failed to clone {} repo", kind);
        }

        git_checkout(src_dir.path(), &config.hacpack_rev)?;

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
            bail!("Failed to build {}", kind);
        }

        //* Moving bin from temp dir to cache dir
        let filename = kind.filename();
        fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
        let dest = APP_CACHE_DIR.join(&filename);
        move_file(src_dir.path().join(&filename), &dest)?;

        Ok(dest)
    }

    pub fn hactool() -> Result<PathBuf> {
        let config = Config::load()?;
        let kind = BackendKind::Hactool;
        info!("Building {}", kind);
        let src_dir = tempdir()?;

        if !Command::new("git")
            .args(["clone", "https://github.com/SciresM/hactool"])
            .arg(src_dir.path())
            .status()?
            .success()
        {
            bail!("Failed to clone {} repo", kind);
        }

        git_checkout(src_dir.path(), &config.hactool_rev)?;

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
            bail!("Failed to build {}", kind);
        }

        //* Moving bin from temp dir to cache dir
        let filename = kind.filename();
        fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
        let dest = APP_CACHE_DIR.join(&filename);
        move_file(src_dir.path().join(&filename), &dest)?;

        Ok(dest)
    }

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    pub fn hac2l<I, S>(args: I) -> Result<PathBuf>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        use tracing::debug;

        let config = Config::load()?;

        let kind = BackendKind::Hac2l;
        info!("Building {}", kind);
        let src_dir = tempdir()?;

        if !Command::new("git")
            .args(["clone", "https://github.com/Atmosphere-NX/Atmosphere.git"])
            .arg(src_dir.path())
            .status()?
            .success()
        {
            bail!("Failed to clone Atmosphere repo");
        }

        git_checkout(src_dir.path(), &config.atmosphere_rev)?;

        let hac2l_src_dir = src_dir.path().join("tools/hac2l");
        if !Command::new("git")
            .args(["clone", "https://github.com/Atmosphere-NX/hac2l.git"])
            .arg(&hac2l_src_dir)
            .status()?
            .success()
        {
            bail!("Failed to clone {} repo", kind);
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
            bail!("Failed to build {}", kind);
        }

        //* Moving bin from temp dir to cache dir
        let filename = kind.filename();
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

        bail!("Failed to build {}", kind);
    }

    pub fn four_nxci() -> Result<PathBuf> {
        let config = Config::load()?;
        let kind = BackendKind::FourNXCI;
        info!("Building {}", kind);
        let src_dir = tempdir()?;

        if !Command::new("git")
            .args(["clone", "https://github.com/The-4n/4NXCI.git"])
            .arg(src_dir.path())
            .status()?
            .success()
        {
            bail!("Failed to clone {} repo", kind)
        }

        git_checkout(src_dir.path(), &config.four_nxci_rev)?;

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
            bail!("Failed to build {}", kind);
        }

        //* Moving bin from temp dir to cache dir
        let filename = kind.filename();
        fs_err::create_dir_all(APP_CACHE_DIR.as_path())?;
        let dest = APP_CACHE_DIR.join(&filename);
        move_file(src_dir.path().join(&filename), &dest)?;

        Ok(dest)
    }

    fn git_checkout<P: AsRef<Path>>(repo: P, rev: &str) -> Result<()> {
        if Command::new("git")
            .args(["checkout", rev])
            .current_dir(repo)
            .status()?
            .success()
        {
            return Ok(());
        }
        bail!("Failed to checkout");
    }
}
