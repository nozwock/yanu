use crate::hac::{
    backend::Backend,
    rom::{Nca, NcaType},
};

use super::rom::Nsp;
use anyhow::{Context, Result};
use std::{fs, process::Command};
use tempdir::TempDir;
use tracing::info;
use walkdir::WalkDir;

const TITLEID_SZ: u8 = 16;

/// `title_key_path` is the Path where TitleKeys will be stored (optional).
pub fn patch_nsp_with_update(base: &mut Nsp, update: &mut Nsp) -> Result<Nsp> {
    let hactool = Backend::Hactool.path()?;
    let hacpack = Backend::Hacpack.path()?;

    base.extract_title_key()?; //? might need a change in future!? (err handling)
    update.extract_title_key()?;
    //* sadly, need to cleanup the dir/files created via this manually...
    //* need to look this up

    let switch_dir = dirs::home_dir()
        .context("failed to find home dir")?
        .join(".switch");
    let title_keys_path = switch_dir.join("title.keys");
    fs::create_dir_all(&switch_dir)?;
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
        let nca = Nca::from(entry.path())?;
        match nca.content_type {
            NcaType::Program => {
                base_nca = Some(nca); // this will be the biggest NCA of 'Program' type
                break;
            }
            _ => {}
        };
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
    let mut update_nca = update_nca.expect("update NCA must exist");
    let mut control_nca = control_nca.expect("control NCA must exist");

    // cleanup
    info!("Cleaning up {:?}", base_data_path.to_string_lossy());
    fs::remove_dir_all(base_data_path)?;
    base.extracted_data = None;
    info!("Cleaning up {:?}", update_data_path.to_string_lossy());
    fs::remove_dir_all(update_data_path)?;
    update.extracted_data = None;

    let patch_dir = TempDir::new("patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
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
        .output()?;

    // pack romfs & exefs into one NCA
    let nca_dir = patch_dir.path().join("nca");
    base_nca.title_id.truncate(TITLEID_SZ as _);
    Command::new(&hacpack)
        .args([
            "--type",
            "nca",
            "--ncatype",
            "program",
            "--plaintext",
            "--exefsdir",
            &exefs_dir.to_string_lossy(),
            "--romfsdoor",
            &romfs_dir.to_string_lossy(),
            "--titleid",
            &base_nca.title_id,
            "--outdir",
            &nca_dir.to_string_lossy(),
        ])
        .output()?;

    let mut pactched_nca: Option<Nca> = None;
    for entry in WalkDir::new(&nca_dir).into_iter().filter_map(|e| e.ok()) {
        pactched_nca = Some(Nca::from(entry.path())?);
        break;
    }

    fs::rename(&control_nca.path, &nca_dir)?;
    control_nca.path = nca_dir.clone();

    // generate meta NCA from patched NCA and control NCa
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
        .output()?;

    let patched_dir = TempDir::new("patched")?.into_path();

    // pack all 3 NCAs into a single NSP
    Command::new(&hacpack)
        .args([
            "--type",
            "nsp",
            "--ncadir",
            &nca_dir.to_string_lossy(),
            "--titleid",
            &base_nca.title_id,
            "--outdir",
            &patched_dir.to_string_lossy(),
        ])
        .output()?;

    Ok(Nsp::from(
        patched_dir.join(format!("{}.nsp", base_nca.title_id)),
    )?)
}
