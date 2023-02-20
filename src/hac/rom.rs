use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use tempdir::TempDir;
use tracing::{debug, info};

use crate::cache::CacheEmbedded;

use super::ticket::{self, TitleKey};

#[derive(Debug, Default, Clone)]
pub struct Nsp {
    path: PathBuf,
    title_key: Option<TitleKey>,
    pub extracted_data: Option<PathBuf>,
}

impl Nsp {
    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().extension().context("no file found")? != "nsp" {
            bail!(
                "{:?} is not an nsp file",
                path.as_ref()
                    .file_name()
                    .context("no file found")?
                    .to_string_lossy()
            );
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            ..Default::default()
        })
    }
    pub fn extract_data_to<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let hactool = CacheEmbedded::Hactool.load()?;

        debug!(
            "hactool extract stdout:\n{}",
            std::str::from_utf8(
                Command::new(&hactool)
                    .args([
                        "-t",
                        "pfs0",
                        "--pfs0dir",
                        &path.as_ref().to_string_lossy(),
                        &self.path.to_string_lossy(),
                    ])
                    .output()?
                    .stdout
                    .as_slice(),
            )?
        );
        self.extracted_data = Some(path.as_ref().to_owned());

        info!(
            "{} has been extracted in \"{}\"",
            self.path
                .file_name()
                .context("no file found")?
                .to_string_lossy(),
            path.as_ref().to_string_lossy()
        );

        Ok(())
    }
    pub fn extract_title_key(&mut self) -> Result<()> {
        let temp_dir: PathBuf;
        info!("Extracting title key for {:?}", self.path.to_string_lossy());

        if self.extracted_data.is_none() {
            temp_dir = TempDir::new("nspdata")?.into_path();
            fs::create_dir_all(&temp_dir)?;
            self.extract_data_to(&temp_dir)?;
        } else {
            temp_dir = self
                .extracted_data
                .as_ref()
                .expect("data must've been extracted")
                .to_path_buf();
        }

        if self.title_key.is_none() {
            for entry in fs::read_dir(temp_dir)? {
                let entry = entry?.path();
                if entry.is_file()
                    && entry
                        .extension()
                        .expect("check for file has already being done")
                        == "tik"
                {
                    self.title_key = Some(ticket::get_title_key(&entry)?);
                }
            }
        } else {
            info!("TitleKey has already being extracted!");
        }

        Ok(())
    }
}
