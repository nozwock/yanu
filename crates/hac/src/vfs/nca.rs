use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
};

use common::utils::{ext_matches, get_size_as_string, move_file};
use derivative::Derivative;
use eyre::{bail, eyre, Result};
use strum_macros::EnumString;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

use crate::{
    backend::{Backend, BackendKind},
    vfs::filter_out_key_mismatches,
};

#[derive(Debug, Clone, Copy, EnumString, PartialEq, Eq, Hash)]
pub enum ContentType {
    Program = 0x00,
    Meta = 0x01,
    Control = 0x02,
    Manual = 0x03,
    Data = 0x04,
    PublicData = 0x05,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

type ProgramID = [u8; 8];

/// https://switchbrew.org/wiki/NCA\
/// Provides some methods relating to Nca, an encrypted content archive.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Nca {
    pub path: PathBuf,
    #[derivative(Debug(format_with = "program_id_fmt"))]
    pub program_id: ProgramID,
    pub content_type: ContentType,
}

fn program_id_fmt(program_id: &ProgramID, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
    fmt.write_fmt(format_args!("{:?}", hex::encode(program_id)))
}

// TODO?: Add the stdout to the logs in case an error is catched in main

impl Nca {
    pub fn try_new<P: AsRef<Path>>(reader: &Backend, file_path: P) -> Result<Self> {
        // Can't rely on Backend tools to check for NCA file because they're
        // pretty bad cli tools (don't even have non zero exit status on failure)
        // excluding Hactoolnet.
        if !file_path.as_ref().is_file() || !ext_matches(file_path.as_ref(), "nca") {
            bail!("'{}' is not a NCA file", file_path.as_ref().display())
        }

        info!(
            nca = %file_path.as_ref().display(),
            size = %get_size_as_string(file_path.as_ref()).unwrap_or_default(),
            "Identifying TitleID and ContentType",
        );

        let output = Command::new(reader.path())
            .args([file_path.as_ref()])
            .output()?; // All streams are piped
        let stderr = filter_out_key_mismatches(&output.stderr);
        if !output.status.success() {
            warn!(
                nca = %file_path.as_ref().display(),
                backend = ?reader.kind(),
                %stderr,
                "Encountered an error while viewing info",
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);

        let program_id_pat = match reader.kind() {
            #[cfg(all(
                target_arch = "x86_64",
                any(target_os = "windows", target_os = "linux")
            ))]
            BackendKind::Hactoolnet => "TitleID:",
            // On all supported platforms
            BackendKind::Hactool => "Title ID:",
            BackendKind::Hac2l => "Program Id:",
            _ => unimplemented!(),
        };
        let mut program_id = [0u8; 8];
        stdout
            .lines()
            .find(|line| line.contains(program_id_pat))
            .map(|line| line.trim().split(' ').last())
            .flatten()
            .map(|id_str| hex::decode_to_slice(id_str, program_id.as_mut()))
            .ok_or_else(|| {
                eyre!(
                    "Failed to process ProgramID of '{}'",
                    file_path.as_ref().display()
                )
            })??;
        debug!(program_id = ?hex::encode(program_id));

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
            Ok(content_type) => content_type.ok_or_else(|| {
                eyre!(
                    "Failed to process ContentType of '{}'",
                    file_path.as_ref().display()
                )
            })?,
            Err(err) => {
                // Unknown ContentType
                warn!(
                    nca = %file_path.as_ref().display(),
                    backend = ?reader.kind(),
                    stdout = %stdout,
                    "Dumping stdout"
                );
                bail!(err);
            }
        };
        debug!(?content_type);

        Ok(Self {
            path: file_path.as_ref().to_owned(),
            program_id,
            content_type,
        })
    }
    pub fn get_program_id(&self) -> String {
        hex::encode(self.program_id)
    }
    pub fn unpack_romfs<P: AsRef<Path>>(&self, extractor: &Backend, romfs_dir: P) -> Result<()> {
        info!(nca = %self.path.display(), "Unpacking RomFS from NCA");
        let output = Command::new(extractor.path())
            .args([
                self.path.as_path(),
                "--romfsdir".as_ref(),
                romfs_dir.as_ref(),
            ])
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()?;
        let stderr = filter_out_key_mismatches(&output.stderr);
        eprint!("{}", stderr);
        if !output.status.success() {
            warn!(
                nca = %self.path.display(),
                backend = ?extractor.kind(),
                %stderr,
                "Encountered an error while unpacking RomFS from NCA",
            );
            bail!("Encountered an error while unpacking RomFS from NCA");
        }

        info!(
            nca = %self.path.display(),
            romfs = %romfs_dir.as_ref().display(),
            "Unpacked RomFS from NCA"
        );

        Ok(())
    }
    pub fn unpack_all<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        extractor: &Backend,
        aux: &Nca,
        romfs_dir: P,
        exefs_dir: Q,
    ) -> Result<()> {
        info!(basenca = %self.path.display(), nca = %aux.path.display(), "Unpacking RomFS/ExeFS from NCAs");
        let mut cmd = Command::new(extractor.path());
        cmd.args([
            "--basenca".as_ref(),
            self.path.as_path(),
            aux.path.as_path(),
            "--romfsdir".as_ref(),
            romfs_dir.as_ref(),
            "--exefsdir".as_ref(),
            exefs_dir.as_ref(),
        ])
        .stderr(Stdio::piped());
        let output = cmd.spawn()?.wait_with_output()?;
        let stderr = filter_out_key_mismatches(&output.stderr);
        eprint!("{}", stderr);
        if !output.status.success() {
            error!(
                backend = ?extractor.kind(),
                code = ?output.status.code(),
                %stderr,
                "Encountered an error while unpacking RomFS/ExeFS from NCAs"
            );
            bail!("Encountered an error while unpacking RomFS/ExeFS from NCAs");
        }

        info!(
            basenca = %self.path.display(),
            nca = %aux.path.display(),
            romfs = %romfs_dir.as_ref().display(),
            exefs = %exefs_dir.as_ref().display(),
            "Unpacked RomFS/ExeFS from NCAs"
        );
        Ok(())
    }
    pub fn pack_program<'a, P, Q, R, K, I>(
        readers: I,
        packer: &Backend,
        program_id: &str,
        keyfile: K,
        romfs_dir: P,
        exefs_dir: Q,
        outdir: R,
    ) -> Result<Nca>
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
        R: AsRef<Path>,
        K: AsRef<Path>,
        I: IntoIterator<Item = &'a Backend>,
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
            program_id.as_ref(),
            "--outdir".as_ref(),
            outdir.as_ref(),
        ])
        .stderr(Stdio::piped());
        let output = cmd.spawn()?.wait_with_output()?;
        let stderr = filter_out_key_mismatches(&output.stderr);
        eprint!("{}", stderr);
        if !output.status.success() {
            warn!(
                backend = ?packer.kind(),
                exit_code = ?output.status.code(),
                %stderr,
                "Encountered an error while packing FS files to NCA"
            );
        }

        let patched_nca = readers
            .into_iter()
            .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
            .map(|reader| nca_with_kind(reader, outdir.as_ref(), ContentType::Program))
            .find(|filtered| filtered.is_some())
            .flatten()
            .ok_or_else(|| eyre!("Failed to pack FS files to NCA"))?
            .remove(0);
        info!(
            nca = %patched_nca.path.display(),
            outdir = %outdir.as_ref().display(),
            "Packing done! Should be Program Type NCA"
        );
        Ok(patched_nca)
    }
    pub fn create_meta<K, O, T>(
        packer: &Backend,
        program_id: &str,
        keyfile: K,
        program: &Nca,
        control: &Nca,
        outdir: O,
        tempdir_in: T,
    ) -> Result<()>
    where
        K: AsRef<Path>,
        O: AsRef<Path>,
        T: AsRef<Path>,
    {
        // figure out a solution for this
        info!(?program.path, ?control.path, "Generating Meta NCA");

        let temp_outdir = tempfile::tempdir_in(tempdir_in.as_ref())?;
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
            program_id.as_ref(),
            "--outdir".as_ref(),
            temp_outdir.path(),
        ])
        .stderr(Stdio::piped());
        let output = cmd.spawn()?.wait_with_output()?;
        let stderr = filter_out_key_mismatches(&output.stderr);
        eprint!("{}", stderr);
        if !output.status.success() {
            warn!(
                backend = ?packer.kind(),
                code = ?output.status.code(),
                %stderr,
                "Encountered an error while generating Meta NCA"
            );
        }

        for entry in WalkDir::new(temp_outdir.path())
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file() && ext_matches(entry.path(), "nca") {
                // Moving NCA file from temp outdir to outdir
                move_file(entry.path(), outdir.as_ref().join(entry.file_name()))?;
                info!(outdir = ?outdir.as_ref(), "Generated Meta NCA");
                return Ok(());
            }
        }

        bail!("Failed to generate Meta NCA");
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
                if entry.path().is_file() && ext_matches(entry.path(), "nca") {
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
        match Nca::try_new(reader, entry.path()) {
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
