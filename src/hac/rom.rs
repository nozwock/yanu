use std::{
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use strum_macros::EnumString;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::hac::backend::Backend;

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
            .context("Failed to get file extension")?
            != "nsp"
        {
            bail!(
                "{:?} is not a nsp file",
                path.as_ref()
                    .file_name()
                    .context("Failed to get filename")?
            );
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            ..Default::default()
        })
    }
    pub fn extract_data_to<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let hactool = Backend::Hactool.path()?;

        info!(nsp = ?self.path, "Extracting");
        if !Command::new(hactool)
            .args([
                "-t",
                "pfs0",
                "--pfs0dir",
                &path.as_ref().to_string_lossy(),
                &self.path.to_string_lossy(),
            ])
            .status()?
            .success()
        {
            bail!("Failed to extract {:?}", path.as_ref());
        }

        info!(nsp = ?self.path, data_dir = ?path.as_ref(), "Extraction done!");
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
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().is_file()
            && path
                .as_ref()
                .extension()
                .context("Failed to get file extension")?
                != "nca"
        {
            bail!(
                "{:?} is not a nca file",
                path.as_ref()
                    .file_name()
                    .context("Failed to get filename")?
            );
        }

        info!(
            nca = ?path.as_ref(),
            "Identifying TitleID and ContentType",
        );

        let hactool = Backend::Hactool.path()?;

        let output = Command::new(&hactool).args([path.as_ref()]).output()?;
        if !output.status.success() {
            bail!("Hactool failed to view info of {:?}", path.as_ref());
        }

        let stdout = std::str::from_utf8(output.stdout.as_slice())?.to_owned();
        let mut title_id: Option<String> = None;
        for line in stdout.lines() {
            if line.find("Title ID:").is_some() {
                title_id = Some(
                    line.trim()
                        .split(' ')
                        .last()
                        .context("TitleID line should've an item")?
                        .into(),
                );
                debug!(?title_id);
                break;
            }
        }

        let mut content_type: Option<NcaType> = None;
        for line in stdout.lines() {
            if line.find("Content Type:").is_some() {
                content_type = Some(
                    NcaType::from_str(
                        line.trim()
                            .split(' ')
                            .last()
                            .context("ContentType line should've an item")?,
                    )
                    .context("Failed to identify NCA content type")?,
                );
                debug!(?content_type);
                break;
            }
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            title_id,
            content_type: content_type.with_context(|| {
                format!("Failed to identify ContentType of {:?}", path.as_ref())
            })?,
        })
    }
}
