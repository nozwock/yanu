use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use eyre::{bail, eyre, Result};
use strum_macros::EnumString;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::backend::{Backend, BackendKind};

#[derive(Debug, Clone, Copy, EnumString, PartialEq, Eq, Hash)]
pub enum ContentType {
    Program,
    Meta,
    Control,
    Manual,
    Data,
    PublicData,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct Nca {
    pub path: PathBuf,
    pub title_id: Option<String>, //? does every NCA have TittleID?
    pub content_type: ContentType,
}

// TODO: add the stdout to the logs in case an error is catches in main

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
                "'{}' is not a nca file",
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
        let title_id = stdout
            .lines()
            .find(|line| line.contains(title_id_pat))
            .map(|line| line.trim().split(' ').last())
            .flatten()
            .map(|id| id.into());
        debug!(?title_id);

        let content_type = match stdout
            .lines()
            .find(|line| line.contains("Content Type:"))
            .map(|line| {
                line.trim()
                    .split(' ')
                    .last()
                    .and_then(|kind| Some(ContentType::from_str(kind)))
            })
            .flatten()
            .transpose()
        {
            Ok(kind) => kind.ok_or_else(|| {
                eyre!(
                    "Failed to identify ContentType of '{}'",
                    path.as_ref().display()
                )
            })?,
            Err(err) => {
                // Unknown ContentType
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

        Ok(Self {
            path: path.as_ref().to_owned(),
            title_id,
            content_type,
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
            if entry.path().is_file() && entry.path().extension() == Some("nca".as_ref()) {
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

/// Returns filtered NCA(s) in descending order of size.
///
/// For eg-
/// ```
/// // This'll return the largest Control type NCA in "."
/// nca_with_filters(
///     Backend::new(BackendKind::Hactoolnet),
///     ".",
///     HashSet::from([NcaType::Control]),
/// )
/// .get(&NcaType::Control)
/// .unwrap()[0];
/// ```
pub fn nca_with_filters<P>(
    reader: &Backend,
    from: P,
    filters: &HashSet<ContentType>,
) -> HashMap<ContentType, Vec<Nca>>
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

#[allow(unused)]
pub fn nca_with_kind<P>(reader: &Backend, from: P, kind: ContentType) -> Option<Vec<Nca>>
where
    P: AsRef<Path>,
{
    nca_with_filters(reader, from, &HashSet::from([kind])).remove(&kind)
}
