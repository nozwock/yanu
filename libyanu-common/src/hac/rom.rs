use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
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

#[derive(Debug, Clone, Copy, EnumString, PartialEq, Eq, Hash)]
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
    pub title_id: Option<String>, //? does every NCA have TittleID?
    pub content_type: NcaType,
}

// TODO: instead of this, add the stdout to logs at the end

impl Nsp {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path
            .as_ref()
            .extension()
            .ok_or_else(|| eyre!("Failed to get file extension"))?
            != "nsp"
        {
            bail!(
                "\"{}\" is not a nsp file",
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
            bail!("Failed to extract \"{}\"", self.path.display());
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
                match entry.path().extension().and_then(OsStr::to_str) {
                    Some("tik") => {
                        self.title_key = Some(ticket::get_title_key(entry.path())?);
                        break;
                    }
                    _ => continue,
                }
            }
            if self.title_key.is_none() {
                bail!(
                    "Couldn't derive TitleKey, \"{}\" doesn't have a .tik file",
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

impl Nca {
    pub fn new<P: AsRef<Path>>(reader: &Backend, path: P) -> Result<Self> {
        if path.as_ref().is_file()
            && path
                .as_ref()
                .extension()
                .ok_or_else(|| eyre!("Failed to get file extension"))?
                != "nca"
        {
            bail!(
                "\"{}\" is not a nca file",
                path.as_ref()
                    .file_name()
                    .map(|ostr| ostr.to_string_lossy())
                    .ok_or_else(|| eyre!("Failed to get filename"))?
            );
        }

        info!(
            nca = %path.as_ref().display(),
            "Identifying TitleID and ContentType",
        );

        let output = Command::new(reader.path()).args([path.as_ref()]).output()?; // Capture stdout aswell :-)
        if !output.status.success() {
            warn!(
                nca = %path.as_ref().display(),
                backend = ?reader.kind(),
                stderr = %String::from_utf8(output.stderr)?,
                "Encountered an error while viewing info",
            );
        } else {
            let stderr = std::str::from_utf8(output.stderr.as_slice())?
                .lines()
                .filter(|line| !line.to_lowercase().contains("failed to match key"))
                .collect::<Vec<_>>()
                .join("\n");
            if !stderr.trim().is_empty() {
                warn!(backend = ?reader.kind(), %stderr);
            }
        }

        let stdout = String::from_utf8(output.stdout)?;
        let mut title_id: Option<String> = None;
        let title_id_pat = match reader.kind() {
            #[cfg(all(
                target_arch = "x86_64",
                any(target_os = "windows", target_os = "linux")
            ))]
            BackendKind::Hactoolnet => "TitleID:",
            // On all supported platforms
            BackendKind::Hactool => "Title ID:",
            BackendKind::Hac2l => "Program Id:",
            _ => unreachable!(),
        };
        for line in stdout.lines() {
            if line.contains(title_id_pat) {
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
            if line.contains("Content Type:") {
                content_type = match NcaType::from_str(
                    line.trim()
                        .split(' ')
                        .last()
                        .ok_or_else(|| eyre!("ContentType line should've an item"))?,
                ) {
                    Ok(content_type) => Some(content_type),
                    Err(err) => {
                        warn!(
                            nca = %path.as_ref().display(),
                            backend = ?reader.kind(),
                            stdout = %stdout,
                            "Dumping stdout"
                        );
                        bail!(err);
                    }
                };
                debug!(?content_type);
                break;
            }
        }

        Ok(Self {
            path: path.as_ref().to_owned(),
            title_id,
            content_type: content_type.ok_or_else(|| {
                eyre!(
                    "Failed to identify ContentType of \"{}\"",
                    path.as_ref().display()
                )
            })?,
        })
    }
    pub fn unpack<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        extractor: &Backend,
        aux: &Nca,
        romfs_dir: P,
        exefs_dir: Q,
    ) -> Result<()> {
        info!(?self.path, ?aux.path, "Extracting");
        let mut cmd = Command::new(extractor.path());
        cmd.args([
            "--basenca".as_ref(),
            self.path.as_path(),
            aux.path.as_path(),
            "--romfsdir".as_ref(),
            romfs_dir.as_ref(),
            "--exefsdir".as_ref(),
            exefs_dir.as_ref(),
        ]);
        cmd.stdout(Stdio::inherit());
        let output = cmd.output()?;
        if !output.status.success() {
            error!(
                backend = ?extractor.kind(),
                code = ?output.status.code(),
                stderr = %String::from_utf8(output.stderr)?,
                "Encountered an error while unpacking NCAs"
            );
            bail!("Encountered an error while unpacking NCAs");
        }

        info!(
            ?self.path,
            romfs = ?romfs_dir.as_ref(),
            exefs = ?exefs_dir.as_ref(),
            "Extraction done"
        );
        Ok(())
    }
    pub fn pack<P, Q, R, K>(
        packer: &Backend,
        title_id: &str,
        keyfile: K,
        romfs_dir: P,
        exefs_dir: Q,
        outdir: R,
    ) -> Result<PathBuf>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
        K: AsRef<Path>,
    {
        info!(
            romfs = ?romfs_dir.as_ref(),
            exefs = ?exefs_dir.as_ref(),
            to = ?outdir.as_ref(),
            "Packing"
        );
        let mut cmd = Command::new(packer.path());
        cmd.args([
            "--keyset".as_ref(),
            keyfile.as_ref(),
            "--type".as_ref(),
            "nca".as_ref(),
            "--ncatype".as_ref(),
            "program".as_ref(),
            "--plaintext".as_ref(),
            "--exefsdir".as_ref(),
            exefs_dir.as_ref(),
            "--romfsdir".as_ref(),
            romfs_dir.as_ref(),
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
                exit_code = ?output.status.code(),
                stderr = %String::from_utf8(output.stderr)?,
                "Encountered an error while packing FS files to NCA"
            );
            bail!("Encountered an error while packing FS files to NCA");
        }

        for entry in WalkDir::new(outdir.as_ref())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension() == Some("nca".as_ref()) {
                info!(outdir = %outdir.as_ref().display(), "Packing done");
                info!(nca = %entry.path().display(), "Should be the Patched NCA");
                return Ok(entry.into_path());
            }
        }
        bail!("Failed to pack romfs/exefs to NCA");
    }
    pub fn create_meta<K, O>(
        packer: &Backend,
        title_id: &str,
        keyfile: K,
        program: &Nca,
        control: &Nca,
        outdir: O,
    ) -> Result<()>
    where
        K: AsRef<Path>,
        O: AsRef<Path>,
    {
        info!(?program.path, ?control.path, "Generating Meta NCA");
        let mut cmd = Command::new(packer.path());
        cmd.args([
            "--keyset".as_ref(),
            keyfile.as_ref(),
            "--type".as_ref(),
            "nca".as_ref(),
            "--ncatype".as_ref(),
            "meta".as_ref(),
            "--titletype".as_ref(),
            "application".as_ref(),
            "--programnca".as_ref(),
            program.path.as_path(),
            "--controlnca".as_ref(),
            control.path.as_path(),
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
                "Encountered an error while generating Meta NCA"
            );
            bail!("Encountered an error while generating Meta NCA");
        }

        info!(outdir = ?outdir.as_ref(), "Generated Meta NCA");
        Ok(())
    }
}

/// Returns filtered NCAs in descending order of size.
///
/// So for eg-
/// ```
/// // This'll return the largest Control type NCA in ""
/// get_filtered_ncas(
///     Backend::new(BackendKind::Hactoolnet),
///     "",
///     HashSet::from([NcaType::Control]),
/// )
/// .get(&NcaType::Control)
/// .unwrap()[0];
/// ```
pub fn get_filtered_ncas<P>(
    reader: &Backend,
    from: P,
    filters: &HashSet<NcaType>,
) -> HashMap<NcaType, Vec<Nca>>
where
    P: AsRef<Path>,
{
    let mut filtered_ncas = HashMap::new();

    for entry in WalkDir::new(from.as_ref())
        .min_depth(1)
        // Sort by descending order of size
        .sort_by_key(|entry| {
            std::cmp::Reverse(entry.metadata().map_or_else(|_e| 0, |meta| meta.len()))
        })
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry) => {
                if entry.path().extension() == Some("nca".as_ref()) {
                    Some(entry)
                } else {
                    None
                }
            }
            Err(err) => {
                warn!(%err);
                None
            }
        })
    {
        match Nca::new(reader, entry.path()) {
            Ok(nca) => {
                if filters.contains(&nca.content_type) {
                    filtered_ncas
                        .entry(nca.content_type)
                        .or_insert(vec![])
                        .push(nca);
                }
            }
            Err(err) => {
                warn!(%err);
            }
        }
    }

    filtered_ncas
}
