use crate::hac::{
    backend::Backend,
    rom::{Nca, NcaType},
};

use super::rom::Nsp;
use anyhow::{Context, Result};
use std::{env, ffi::OsStr, fs, path::PathBuf, process::Command};
use tempdir::TempDir;
use tracing::info;
use walkdir::WalkDir;

const TITLEID_SZ: u8 = 16;

pub fn patch_nsp_with_update(base: &mut Nsp, update: &mut Nsp) -> Result<Nsp> {
    let hactool = Backend::Hactool.path()?;
    let hacpack = Backend::Hacpack.path()?;

    base.derive_title_key()?; //? might need a change in future!? (err handling)
    update.derive_title_key()?;
    //* sadly, need to cleanup the dir/files created via this manually...
    //* need to look this up

    let switch_dir = dirs::home_dir()
        .context("failed to find home dir")?
        .join(".switch");
    fs::create_dir_all(&switch_dir)?;
    let title_keys_path = switch_dir.join("title.keys");

    info!("Saving TitleKeys in {:?}", title_keys_path.display());
    fs::write(
        &title_keys_path,
        format!(
            "{}\n{}",
            base.title_key.as_ref().unwrap().to_string(),
            update.title_key.as_ref().unwrap().to_string()
        ),
    )?;

    let base_data_path = base
        .extracted_data
        .as_ref()
        .context("failed to extract the base nsp")?;
    let update_data_path = update
        .extracted_data
        .as_ref()
        .context("failed to extract the update nsp")?;

    let mut base_nca: Option<Nca> = None;
    for entry in WalkDir::new(base_data_path)
        .sort_by(|a, b| {
            a.metadata()
                .expect(&format!("failed to read metadata for {:?}", a.path()))
                .len()
                .cmp(
                    &b.metadata()
                        .expect(&format!("failed to read metadata for {:?}", b.path()))
                        .len(),
                )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                let nca = Nca::from(entry.path())?;
                match nca.content_type {
                    NcaType::Program => {
                        base_nca = Some(nca); // this will be the biggest NCA of 'Program' type
                        break;
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }
    let mut base_nca = base_nca.expect("base NCA must exist");

    let mut control_nca: Option<Nca> = None;
    let mut update_nca: Option<Nca> = None;
    for entry in WalkDir::new(update_data_path)
        .sort_by(|a, b| {
            a.metadata()
                .expect(&format!("failed to read metadata for {:?}", a.path()))
                .len()
                .cmp(
                    &b.metadata()
                        .expect(&format!("failed to read metadata for {:?}", b.path()))
                        .len(),
                )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                let nca = Nca::from(entry.path())?;
                match nca.content_type {
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
                }
            }
            _ => {}
        }
    }
    let update_nca = update_nca.expect("update NCA must exist");
    let mut control_nca = control_nca.expect("control NCA must exist");

    let patch_dir = TempDir::new("patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    info!(
        "Extracting romfs & exefs of {:?} & {:?}",
        base_nca.path.display(),
        update_nca.path.display()
    );
    Command::new(&hactool)
        .args([
            "--basenca",
            &base_nca.path.to_string_lossy(),
            &update_nca.path.to_string_lossy(),
            "--romfsdir",
            &romfs_dir.to_string_lossy(),
            "--exefsdir",
            &exefs_dir.to_string_lossy(),
        ])
        .status()?
        .exit_ok()?;

    let nca_dir = patch_dir.path().join("nca");
    base_nca.title_id.truncate(TITLEID_SZ as _);
    info!("Packing romfs & exefs in a single NCA");
    Command::new(&hacpack)
        .args([
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
            &base_nca.title_id,
            "--outdir",
            &nca_dir.to_string_lossy(),
        ])
        .status()?
        .exit_ok()?;

    let mut pactched_nca: Option<Nca> = None;
    for entry in WalkDir::new(&nca_dir).into_iter().filter_map(|e| e.ok()) {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                pactched_nca = Some(Nca::from(dbg!(entry.path()))?);
                break;
            }
            _ => {}
        }
    }
    dbg!(fs::rename(
        dbg!(&control_nca.path),
        dbg!(&nca_dir.join(control_nca.path.file_name().expect("NCA file must exist")))
    )?);
    control_nca.path = nca_dir.join(control_nca.path.file_name().expect("NCA file must exist"));

    // cleanup
    info!("Cleaning up {:?}", base_data_path.to_string_lossy());
    fs::remove_dir_all(base_data_path)?;
    base.extracted_data = None;
    info!("Cleaning up {:?}", update_data_path.to_string_lossy());
    fs::remove_dir_all(update_data_path)?;
    update.extracted_data = None;

    info!("Generating Meta NCA from patched NCA & control NCA");
    Command::new(&hacpack)
        .args([
            "--type",
            "nca",
            "--ncatype",
            "meta",
            "--titletype",
            "application",
            "--programnca",
            &pactched_nca
                .expect("patched NCA must exist")
                .path
                .to_string_lossy(),
            "--controlnca",
            &control_nca.path.to_string_lossy(),
            "--titleid",
            &base_nca.title_id,
            "--outdir",
            &nca_dir.to_string_lossy(),
        ])
        .status()?
        .exit_ok()?;

    // TODO: need to rewrite this aswell, prolly just take outdir as an arg in the fn
    let outdir: PathBuf;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        outdir = env::current_exe()?
            .parent()
            .expect("can't access parent dir of yanu")
            .to_owned();
    }
    #[cfg(target_os = "android")]
    {
        outdir = dirs::home_dir()
            .context("couldn't access home dir")?
            .join("storage")
            .join("shared");
    }
    let patched_nsp_path = outdir.join(format!("{}.nsp", base_nca.title_id));

    info!(
        "Packing all 3 NCAs into a NSP as {:?}",
        patched_nsp_path.display()
    );
    Command::new(&hacpack)
        .args([
            "--type",
            "nsp",
            "--ncadir",
            &nca_dir.to_string_lossy(),
            "--titleid",
            &base_nca.title_id,
            "--outdir",
            &outdir.to_string_lossy(),
        ])
        .status()?
        .exit_ok()?;

    Ok(Nsp::from(patched_nsp_path)?)
}
