use std::{
    ffi::OsStr,
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{bail, Context, Result};
use strum_macros::EnumString;
use tempdir::TempDir;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::hac::backend::Backend;

use super::ticket::{self, TitleKey};

#[derive(Debug, Default, Clone)]
pub struct Nsp {
    pub path: PathBuf,
    pub title_key: Option<TitleKey>,
    pub extracted_data: Option<PathBuf>,
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
    pub title_id: String,
    pub content_type: NcaType,
}

impl Nsp {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().extension().context("no file found")? != "nsp" {
            bail!(
                "{:?} is not a nsp file",
                path.as_ref().file_name().context("no file found")?
            );
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            ..Default::default()
        })
    }
    pub fn extract_data_to<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let hactool = Backend::Hactool.path()?;

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
            bail!("hactool failed to extract {:?}", path.as_ref());
        }
        self.extracted_data = Some(path.as_ref().to_owned());

        info!(
            "{:?} has been extracted in {:?}",
            self.path.file_name().context("no file found")?,
            path.as_ref()
        );

        Ok(())
    }
    pub fn derive_title_key(&mut self) -> Result<()> {
        let temp_dir: PathBuf;

        if self.extracted_data.is_none() {
            temp_dir = TempDir::new("nspdata")?.into_path();
            fs::create_dir_all(&temp_dir)?;
            dbg!(self.extract_data_to(&temp_dir)?);
        } else {
            temp_dir = self
                .extracted_data
                .as_ref()
                .expect("data must've been extracted")
                .to_path_buf();
        }

        if dbg!(self.title_key.is_none()) {
            info!("Deriving title key for {:?}", self.path.display());
            for entry in WalkDir::new(&temp_dir) {
                let entry = entry?;
                match dbg!(entry.path().extension().and_then(OsStr::to_str)) {
                    Some("tik") => {
                        self.title_key = Some(ticket::get_title_key(&entry.path())?);
                        break;
                    }
                    _ => continue,
                }
            }
            if self.title_key.is_none() {
                bail!("failed to derive title key for {:?}", self.path);
            }
        } else {
            info!("TitleKey has already being derived!");
        }

        Ok(())
    }
}

impl Nca {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().extension().context("no file found")? != "nca" {
            bail!(
                "{:?} is not a nca file",
                path.as_ref().file_name().context("no file found")?
            );
        }

        info!(
            "Identifying title ID and content type for {:?}",
            path.as_ref()
        );

        let hactool = Backend::Hactool.path()?;

        let output = Command::new(&hactool).args([path.as_ref()]).output()?;
        if !output.status.success() {
            bail!("hactool failed to view info of {:?}", path.as_ref());
        }

        let stdout = std::str::from_utf8(output.stdout.as_slice())?.to_owned();
        let mut title_id: Option<String> = None;
        for line in stdout.lines() {
            if line.find("Title ID:").is_some() {
                title_id = Some(
                    line.trim()
                        .split(' ')
                        .last()
                        .expect("line must've had an item")
                        .into(),
                );
                debug!("Title ID: {:?}", title_id);
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
                            .expect("line must've had an item"),
                    )
                    .context("failed to identify nca content type")?,
                );
                debug!("Content Type: {:?}", content_type);
                break;
            }
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            title_id: title_id.expect("title id should've been found"),
            content_type: content_type.expect("content type should've been found"),
        })
    }
}
