use std::{
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    str::FromStr,
};

use eyre::{bail, eyre, Result};
use strum_macros::EnumString;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::hac::backend::{Backend, BackendKind};

use super::ticket::{self, TitleKey};

#[derive(Debug, Default, Clone)]
pub struct Nsp {
    pub path: PathBuf,
    pub title_key: Option<TitleKey>,
}

#[derive(Debug, Clone, EnumString)]
pub enum NcaType {
    Control,
    Program,
    Meta,
    Manual,
}

impl fmt::Display for NcaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct Nca {
    pub path: PathBuf,
    pub title_id: Option<String>,
    pub content_type: NcaType,
}

impl Nsp {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path
            .as_ref()
            .extension()
            .ok_or_else(|| eyre!("Failed to get file extension"))?
            != "nsp"
        {
            bail!(
                "{:?} is not a nsp file",
                path.as_ref()
                    .file_name()
                    .ok_or_else(|| eyre!("Failed to get filename"))?
            );
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            ..Default::default()
        })
    }
    pub fn extract_data<P: AsRef<Path>>(&mut self, extractor: &Backend, to: P) -> Result<()> {
        info!(nsp = ?self.path, "Extracting");
        let mut cmd = Command::new(extractor.path());
        cmd.args([
            "-t".as_ref(),
            "pfs0".as_ref(),
            "--outdir".as_ref(),
            to.as_ref(),
            self.path.as_path(),
        ]);
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        cmd.stdout(Stdio::inherit());
        let output = cmd.output()?;
        if !output.status.success() {
            error!(stderr = %String::from_utf8(output.stderr)?);
            bail!("Failed to extract {:?}", self.path);
        }

        info!(nsp = ?self.path, data_dir = ?to.as_ref(), "Extraction done!");
        Ok(())
    }
    pub fn derive_title_key<P: AsRef<Path>>(&mut self, data_path: P) -> Result<()> {
        if self.title_key.is_none() {
            info!(nsp = ?self.path, "Deriving TitleKey");
            for entry in WalkDir::new(data_path.as_ref())
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                match entry.path().extension().and_then(OsStr::to_str) {
                    Some("tik") => {
                        self.title_key = Some(ticket::get_title_key(&entry.path())?);
                        break;
                    }
                    _ => continue,
                }
            }
            if self.title_key.is_none() {
                bail!(
                    "Couldn't derive TitleKey, {:?} doesn't have a .tik file",
                    self.path
                );
            }
            info!("Derived TitleKey successfully!");
        } else {
            info!("TitleKey already exists");
        }

        Ok(())
    }
    pub fn get_title_key(&self) -> String {
        match self.title_key {
            Some(ref key) => key.to_string(),
            None => "=".to_string(),
        }
    }
}

impl Nca {
    pub fn new<P: AsRef<Path>>(extractor: &Backend, path: P) -> Result<Self> {
        if path.as_ref().is_file()
            && path
                .as_ref()
                .extension()
                .ok_or_else(|| eyre!("Failed to get file extension"))?
                != "nca"
        {
            bail!(
                "{:?} is not a nca file",
                path.as_ref()
                    .file_name()
                    .ok_or_else(|| eyre!("Failed to get filename"))?
            );
        }

        info!(
            nca = ?path.as_ref(),
            "Identifying TitleID and ContentType",
        );

        let output = Command::new(extractor.path())
            .args([path.as_ref()])
            .output()?; // Capture stdout aswell :-)
        if !output.status.success() {
            warn!(
                nca = ?path.as_ref(),
                stderr = %String::from_utf8(output.stderr)?,
                "An error occured while trying to view info",
            );
        }

        let stdout = String::from_utf8(output.stdout)?.to_owned();
        let mut title_id: Option<String> = None;
        let title_id_pat = match extractor.kind() {
            BackendKind::Hactool => "Title ID:",
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            BackendKind::Hactoolnet => "TitleID:",
            _ => unreachable!(),
        };
        for line in stdout.lines() {
            if line.find(title_id_pat).is_some() {
                title_id = Some(
                    line.trim()
                        .split(' ')
                        .last()
                        .ok_or_else(|| eyre!("TitleID line should've an item"))?
                        .into(),
                );
                debug!(?title_id);
                break;
            }
        }

        let mut content_type: Option<NcaType> = None;
        for line in stdout.lines() {
            if line.find("Content Type:").is_some() {
                content_type = Some(NcaType::from_str(
                    line.trim()
                        .split(' ')
                        .last()
                        .ok_or_else(|| eyre!("ContentType line should've an item"))?,
                )?);
                debug!(?content_type);
                break;
            }
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            title_id,
            content_type: content_type
                .ok_or_else(|| eyre!("Failed to identify ContentType of {:?}", path.as_ref()))?,
        })
    }
}
