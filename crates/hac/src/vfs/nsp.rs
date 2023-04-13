use crate::{backend::Backend, vfs::ticket::TitleKey};
use common::utils::ext_matches;
use eyre::{bail, Result};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{error, info};
use walkdir::WalkDir;

/// https://switchbrew.org/wiki/NCA#PFS0
///
/// Provides some methods relating to Pfs0, a file system.
#[derive(Debug, Default, Clone)]
pub struct Nsp {
    pub path: PathBuf,
    pub title_key: Option<TitleKey>,
}

impl Nsp {
    pub fn try_new<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().is_file() || !ext_matches(path.as_ref(), "nsp") {
            bail!("'{}' is not a NSP file", path.as_ref().display());
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
        program_id: &str,
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
            program_id.as_ref(),
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
        Nsp::try_new(outdir.as_ref().join(format!("{}.nsp", program_id)))
    }
    pub fn derive_title_key<P: AsRef<Path>>(&mut self, data_path: P) -> Result<()> {
        if self.title_key.is_none() {
            info!(nsp = ?self.path, "Deriving TitleKey");
            for entry in WalkDir::new(data_path.as_ref())
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if ext_matches(entry.path(), "tik") {
                    self.title_key = Some(TitleKey::try_new(entry.path())?);
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
