use crate::{
    defines::{app_cache_dir, get_default_keyfile_path},
    hac::{
        backend::Backend,
        rom::{Nca, NcaType},
    },
    utils::move_file,
};

use super::rom::Nsp;
use eyre::{bail, eyre, Result};
use std::{cmp, ffi::OsStr, fs, io, path::Path, process::Command};
use tempdir::TempDir;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

const TITLEID_SZ: u8 = 16;

pub fn patch_nsp_with_update<O: AsRef<Path>>(
    base: &mut Nsp,
    update: &mut Nsp,
    outdir: O,
) -> Result<Nsp> {
    let hactool = Backend::Hactool.path()?;
    let hacpack = Backend::Hacpack.path()?;

    let switch_dir = dirs::home_dir()
        .ok_or_else(|| eyre!("Failed to find home dir"))?
        .join(".switch");
    fs::create_dir_all(&switch_dir)?;
    let title_keys_path = switch_dir.join("title.keys");
    match fs::remove_file(&title_keys_path) {
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => {
            bail!("{}", err);
        }
        _ => {}
    }

    let cache_dir = app_cache_dir();
    let temp_dir = TempDir::new_in(&cache_dir, "yanu")?;
    let base_data_dir = TempDir::new_in(&temp_dir, "basedata")?;
    let update_data_dir = TempDir::new_in(&temp_dir, "updatedata")?;
    fs::create_dir_all(base_data_dir.path())?;
    fs::create_dir_all(update_data_dir.path())?;

    base.extract_data_to(base_data_dir.path())?;
    update.extract_data_to(update_data_dir.path())?;

    if let Err(err) = base.derive_title_key(base_data_dir.path()) {
        warn!(?err, "This error is not being handeled right away!",);
    }
    if let Err(err) = update.derive_title_key(update_data_dir.path()) {
        warn!(?err, "This error is not being handeled right away!");
    }

    info!(keyfile = ?title_keys_path, "Storing TitleKeys");
    fs::write(
        &title_keys_path,
        format!("{}\n{}", base.get_title_key(), update.get_title_key()),
    )?;

    let mut base_nca: Option<Nca> = None;
    for entry in WalkDir::new(base_data_dir.path())
        .min_depth(1)
        .sort_by_key(|a| {
            cmp::Reverse(
                a.metadata()
                    .expect(&format!("Failed to read metadata of {:?}", a.path()))
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
        .ok_or_else(|| eyre!("Couldn't find a Base NCA (Program Type) in {:?}", base.path))?;
    debug!(?base_nca);

    let mut control_nca: Option<Nca> = None;
    let mut update_nca: Option<Nca> = None;
    for entry in WalkDir::new(update_data_dir.path())
        .min_depth(1)
        .sort_by_key(|a| {
            cmp::Reverse(
                a.metadata()
                    .expect(&format!("Failed to read metadata of {:?}", a.path()))
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
    let update_nca = update_nca.ok_or_else(|| {
        eyre!(
            "Couldn't find a Update NCA (Program Type) in {:?}",
            update.path
        )
    })?;
    debug!(?update_nca);
    let mut control_nca = control_nca.ok_or_else(|| {
        eyre!(
            "Couldn't find a Control NCA (Control Type) in {:?}",
            update.path
        )
    })?;
    debug!(?control_nca);

    let patch_dir = TempDir::new_in(&temp_dir, "patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    info!(?base_nca.path, ?update_nca.path, "Extracting romfs/exefs");
    let status = Command::new(&hactool)
        .args([
            "--basenca".as_ref(),
            base_nca.path.as_path(),
            update_nca.path.as_path(),
            "--romfsdir".as_ref(),
            romfs_dir.as_path(), // ! hacshit seems to fail if the outdirs are in different mount places -_-
            "--exefsdir".as_ref(),
            exefs_dir.as_path(),
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
    info!(dir = ?base_data_dir.path(), "Cleaning up");
    drop(base_data_dir);
    info!(dir = ?update_data_dir.path(), "Cleaning up");
    drop(update_data_dir);

    let keyset_path = get_default_keyfile_path()?;
    let mut title_id = base_nca
        .title_id
        .ok_or_else(|| eyre!("Base NCA ({:?}) should've a TitleID", base_nca.path))?;
    title_id.truncate(TITLEID_SZ as _);
    info!("Packing romfs/exefs into a single NCA");
    if !Command::new(&hacpack)
        .args([
            "--keyset".as_ref(),
            keyset_path.as_path(),
            "--type".as_ref(),
            "nca".as_ref(),
            "--ncatype".as_ref(),
            "program".as_ref(),
            "--plaintext".as_ref(),
            "--exefsdir".as_ref(),
            exefs_dir.as_path(),
            "--romfsdir".as_ref(),
            romfs_dir.as_path(),
            "--titleid".as_ref(),
            title_id.as_ref(),
            "--outdir".as_ref(),
            nca_dir.as_path(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to pack romfs/exefs into a single NCA");
    }

    let mut pactched_nca: Option<Nca> = None;
    for entry in WalkDir::new(&nca_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
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
            "--keyset".as_ref(),
            keyset_path.as_path(),
            "--type".as_ref(),
            "nca".as_ref(),
            "--ncatype".as_ref(),
            "meta".as_ref(),
            "--titletype".as_ref(),
            "application".as_ref(),
            "--programnca".as_ref(),
            pactched_nca
                .ok_or_else(|| eyre!("Couldn't find the patched NCA"))?
                .path
                .as_path(),
            "--controlnca".as_ref(),
            control_nca.path.as_path(),
            "--titleid".as_ref(),
            title_id.as_ref(),
            "--outdir".as_ref(),
            nca_dir.as_path(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to generate Meta NCA from patched NCA & control NCA");
    }

    let patched_nsp_path = cache_dir.join(format!("{}.nsp", title_id));

    info!(
        patched_nsp = ?patched_nsp_path,
        "Packing all 3 NCAs into a NSP"
    );
    if !Command::new(&hacpack)
        .args([
            "--keyset".as_ref(),
            keyset_path.as_path(),
            "--type".as_ref(),
            "nsp".as_ref(),
            "--ncadir".as_ref(),
            nca_dir.as_path(),
            "--titleid".as_ref(),
            title_id.as_ref(),
            "--outdir".as_ref(),
            cache_dir.as_ref(),
        ])
        .status()?
        .success()
    {
        bail!("Failed to Pack all 3 NCAs into a NSP");
    }

    // Moving patched NSP to outdir
    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-patched].nsp", title_id));
    move_file(patched_nsp_path, &dest)?;

    Ok(Nsp::from(dest)?)
}
