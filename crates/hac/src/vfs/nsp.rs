use super::ticket::TitleKey;
use crate::{backend::Backend, vfs::ticket};
use eyre::{bail, eyre, Result};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{error, info};
use walkdir::WalkDir;

#[derive(Debug, Default, Clone)]
pub struct Nsp {
    pub path: PathBuf,
    pub title_key: Option<TitleKey>,
}

impl Nsp {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path
            .as_ref()
            .extension()
            .ok_or_else(|| eyre!("Failed to get file extension"))?
            != "nsp"
        {
            bail!(
                "'{}' is not a nsp file",
                path.as_ref()
                    .file_name()
                    .map(|ostr| ostr.to_string_lossy())
                    .ok_or_else(|| eyre!("Failed to get filename"))?
            );
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            ..Default::default()
        })
    }
    pub fn unpack<P: AsRef<Path>>(&self, extractor: &Backend, to: P) -> Result<()> {
        info!(?self.path, "Extracting");
        let mut cmd = Command::new(extractor.path());
        cmd.args([
            "-t".as_ref(),
            "pfs0".as_ref(),
            "--outdir".as_ref(),
            to.as_ref(),
            self.path.as_path(),
        ]);
        cmd.stdout(Stdio::inherit());
        let output = cmd.output()?;
        if !output.status.success() {
            error!(
                backend = ?extractor.kind(),
                stderr = %String::from_utf8(output.stderr)?,
                "Encountered an error while unpacking NSP"
            );
            bail!("Failed to extract '{}'", self.path.display());
        }

        info!(?self.path, to = ?to.as_ref(), "Extraction done!");
        Ok(())
    }
    pub fn pack<K, P, Q>(
        packer: &Backend,
        title_id: &str,
        keyfile: K,
        nca_dir: P,
        outdir: Q,
    ) -> Result<Nsp>
    where
        K: AsRef<Path>,
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        info!(nca_dir = ?nca_dir.as_ref(), "Packing NCAs to NSP");
        let mut cmd = Command::new(packer.path());
        cmd.args([
            "--keyset".as_ref(),
            keyfile.as_ref(),
            "--type".as_ref(),
            "nsp".as_ref(),
            "--ncadir".as_ref(),
            nca_dir.as_ref(),
            "--titleid".as_ref(),
            title_id.as_ref(),
            "--outdir".as_ref(),
            outdir.as_ref(),
        ]);
        cmd.stdout(Stdio::inherit());
        let output = cmd.output()?;
        if !output.status.success() {
            error!(
                backend = ?packer.kind(),
                code = ?output.status.code(),
                stderr = %String::from_utf8(output.stderr)?,
                "Encountered an error while packing NCAs to NSP"
            );
            bail!("Encountered an error while packing NCAs to NSP");
        }

        info!(outdir = ?outdir.as_ref(), "Packed NCAs to NSP");
        Nsp::new(outdir.as_ref().join(format!("{}.nsp", title_id)))
    }
    pub fn derive_title_key<P: AsRef<Path>>(&mut self, data_path: P) -> Result<()> {
        if self.title_key.is_none() {
            info!(nsp = ?self.path, "Deriving TitleKey");
            for entry in WalkDir::new(data_path.as_ref())
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.path().extension() == Some("tik".as_ref()) {
                    self.title_key = Some(ticket::get_title_key(entry.path())?);
                    break;
                }
                continue;
            }
            if self.title_key.is_none() {
                bail!(
                    "Couldn't derive TitleKey, '{}' doesn't have a .tik file",
                    self.path.display()
                );
            }
            info!("Derived TitleKey successfully!");
        } else {
            info!("TitleKey already exists");
        }

        Ok(())
    }
}
