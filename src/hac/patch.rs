use crate::{
    defines::get_default_keyfile_path,
    hac::{
        backend::Backend,
        rom::{Nca, NcaType},
    },
};

use super::rom::Nsp;
use anyhow::{bail, Context, Result};
use std::{ffi::OsStr, fs, path::Path, process::Command};
use tempdir::TempDir;
use tracing::{info, warn};
use walkdir::WalkDir;

const TITLEID_SZ: u8 = 16;

pub fn patch_nsp_with_update<O: AsRef<Path>>(
    base: &mut Nsp,
    update: &mut Nsp,
    outdir: O,
) -> Result<Nsp> {
    let hactool = Backend::Hactool.path()?;
    let hacpack = Backend::Hacpack.path()?;

    let temp_dir = TempDir::new("yanu")?;
    let base_data_path = TempDir::new_in(&temp_dir, "basedata")?;
    let update_data_path = TempDir::new_in(&temp_dir, "updatedata")?;
    fs::create_dir_all(base_data_path.path())?;
    fs::create_dir_all(update_data_path.path())?;

    base.extract_data_to(base_data_path.path())?;
    update.extract_data_to(update_data_path.path())?;

    if let Err(err) = base.derive_title_key(base_data_path.path()) {
        warn!(?err, "This error is not being handeled right away!",);
    }
    if let Err(err) = update.derive_title_key(update_data_path.path()) {
        warn!(?err, "This error is not being handeled right away!");
    }

    let switch_dir = dirs::home_dir()
        .context("Failed to find home dir")?
        .join(".switch");
    fs::create_dir_all(&switch_dir)?;
    let title_keys_path = switch_dir.join("title.keys");

    info!(keyfile = ?title_keys_path, "Storing TitleKeys");
    fs::write(
        &title_keys_path,
        format!("{}\n{}", base.get_title_key(), update.get_title_key()),
    )?;

    let mut base_nca: Option<Nca> = None;
    for entry in WalkDir::new(base_data_path.path())
        .sort_by(|a, b| {
            a.metadata()
                .expect(&format!("Failed to read metadata of {:?}", a.path()))
                .len()
                .cmp(
                    &b.metadata()
                        .expect(&format!("Failed to read metadata of {:?}", b.path()))
                        .len(),
                )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                match Nca::from(entry.path()) {
                    Ok(nca) => {
                        match nca.content_type {
                            NcaType::Program => {
                                base_nca = Some(nca); // this will be the biggest NCA of 'Program' type
                                break;
                            }
                            _ => {}
                        };
                    }
                    Err(err) => {
                        warn!("{}", err);
                    }
                }
            }
            _ => {}
        }
    }
    let base_nca = base_nca
        .with_context(|| format!("Couldn't find a Base NCA (Program Type) in {:?}", base.path))?;

    let mut control_nca: Option<Nca> = None;
    let mut update_nca: Option<Nca> = None;
    for entry in WalkDir::new(update_data_path.path())
        .sort_by(|a, b| {
            a.metadata()
                .expect(&format!("Failed to read metadata of {:?}", a.path()))
                .len()
                .cmp(
                    &b.metadata()
                        .expect(&format!("Failed to read metadata of {:?}", b.path()))
                        .len(),
                )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => match Nca::from(entry.path()) {
                Ok(nca) => match nca.content_type {
                    NcaType::Control => {
                        if control_nca.is_none() {
                            control_nca = Some(nca);
                        }
                    }
                    NcaType::Program => {
                        if update_nca.is_none() {
                            update_nca = Some(nca);
                        }
                    }
                    _ => {}
                },
                Err(err) => {
                    warn!("{}", err);
                }
            },
            _ => {}
        }
    }
    let update_nca = update_nca.with_context(|| {
        format!(
            "Couldn't find a Update NCA (Program Type) in {:?}",
            update.path
        )
    })?;
    let mut control_nca = control_nca.with_context(|| {
        format!(
            "Couldn't find a Control NCA (Control Type) in {:?}",
            update.path
        )
    })?;

    let patch_dir = TempDir::new_in(&temp_dir, "patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    info!(?base_nca.path, ?update_nca.path, "Extracting romfs/exefs");
    let status = Command::new(&hactool)
        .args([
            "--basenca",
            &base_nca.path.to_string_lossy(),
            &update_nca.path.to_string_lossy(),
            "--romfsdir",
            &romfs_dir.to_string_lossy(),
            "--exefsdir",
            &exefs_dir.to_string_lossy(),
        ])
        .status()?;
    if !status.success() {
        warn!(
            exit_code = ?status.code(),
            "The proccess responsible for extracting romfs/exefs terminated improperly (This might result in a crash!)",
        );
    }

    let nca_dir = patch_dir.path().join("nca");
    fs::create_dir_all(&nca_dir)?;
    let control_nca_filename = control_nca
        .path
        .file_name()
        .expect("File should've a filename");
    fs::rename(&control_nca.path, &nca_dir.join(control_nca_filename))?;
    control_nca.path = nca_dir.join(control_nca_filename);

    // Early cleanup
    info!(dir = ?base_data_path.path(), "Cleaning up");
    drop(base_data_path);
    info!(dir = ?update_data_path.path(), "Cleaning up");
    drop(update_data_path);

    let keyset_path = get_default_keyfile_path()?;
    let mut title_id = base_nca
        .title_id
        .with_context(|| format!("Base NCA ({:?}) should've a TitleID", base_nca.path))?;
    title_id.truncate(TITLEID_SZ as _);
    info!("Packing romfs/exefs into a single NCA");
    if !Command::new(&hacpack)
        .args([
            "--keyset",
            &keyset_path.to_string_lossy(),
            "--type",
            "nca",
            "--ncatype",
            "program",
            "--plaintext",
            "--exefsdir",
            &exefs_dir.to_string_lossy(),
            "--romfsdir",
            &romfs_dir.to_string_lossy(),
            "--titleid",
            &title_id,
            "--outdir",
            &nca_dir.to_string_lossy(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to pack romfs/exefs into a single NCA");
    }

    let mut pactched_nca: Option<Nca> = None;
    for entry in WalkDir::new(&nca_dir).into_iter().filter_map(|e| e.ok()) {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                pactched_nca = Some(Nca::from(entry.path())?);
                break;
            }
            _ => {}
        }
    }

    info!("Generating Meta NCA from patched NCA & control NCA");
    if !Command::new(&hacpack)
        .args([
            "--keyset",
            &keyset_path.to_string_lossy(),
            "--type",
            "nca",
            "--ncatype",
            "meta",
            "--titletype",
            "application",
            "--programnca",
            &pactched_nca
                .context("Couldn't find the patched NCA")?
                .path
                .to_string_lossy(),
            "--controlnca",
            &control_nca.path.to_string_lossy(),
            "--titleid",
            &title_id,
            "--outdir",
            &nca_dir.to_string_lossy(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to generate Meta NCA from patched NCA & control NCA");
    }

    let patched_nsp_path = outdir.as_ref().join(format!("{}.nsp", title_id));

    info!(
        patched_nsp = ?patched_nsp_path.display(),
        "Packing all 3 NCAs into a NSP"
    );
    if !Command::new(&hacpack)
        .args([
            "--keyset",
            &keyset_path.to_string_lossy(),
            "--type",
            "nsp",
            "--ncadir",
            &nca_dir.to_string_lossy(),
            "--titleid",
            &title_id,
            "--outdir",
            &outdir.as_ref().to_string_lossy(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to Pack all 3 NCAs into a NSP");
    }

    Ok(Nsp::from(patched_nsp_path)?)
}
