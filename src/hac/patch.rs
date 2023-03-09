use crate::{
    defines::get_default_keyfile_path,
    hac::{
        backend::Backend,
        rom::{Nca, NcaType},
    },
    utils::move_file,
};

use super::rom::Nsp;
use console::style;
use eyre::{bail, eyre, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    cmp,
    ffi::OsStr,
    fs, io,
    path::Path,
    process::{Command, Stdio},
    time::{self, Duration},
};
use tempdir::TempDir;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

fn default_spinner() -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.enable_steady_tick(Duration::from_millis(80));
    sp.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    sp
}

const TITLEID_SZ: u8 = 16;

pub fn patch_nsp_with_update<O: AsRef<Path>>(
    base: &mut Nsp,
    update: &mut Nsp,
    outdir: O,
) -> Result<Nsp> {
    //* It's a mess, ik and I'm not sry ;-;
    let started = time::Instant::now();

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let extractor = Backend::new(Backend::HACTOOLNET)?;
    #[cfg(target_os = "android")]
    let extractor = Backend::new(Backend::HACTOOL)?;
    let packer = Backend::new(Backend::HACPACK)?;

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

    let exe_path = std::env::current_exe()?;
    let root_dir = exe_path
        .parent()
        .ok_or_else(|| eyre!("Failed to get parent of {:?}", exe_path))?;
    let temp_dir = TempDir::new_in(&root_dir, "yanu")?;
    let base_data_dir = TempDir::new_in(&temp_dir, "basedata")?;
    let update_data_dir = TempDir::new_in(&temp_dir, "updatedata")?;
    fs::create_dir_all(base_data_dir.path())?;
    fs::create_dir_all(update_data_dir.path())?;

    println!("{}", style("Extracting NSP data...").yellow().bold());

    base.extract_data(&extractor, base_data_dir.path())?;
    update.extract_data(&extractor, update_data_dir.path())?;

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
                match Nca::new(&extractor, entry.path()) {
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
            Some("nca") => match Nca::new(&extractor, entry.path()) {
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

    println!("{}", style("Extracting romfs/exefs...").yellow().bold());

    let patch_dir = TempDir::new_in(&temp_dir, "patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    info!(?base_nca.path, ?update_nca.path, "Extracting romfs/exefs");
    let mut cmd = Command::new(extractor.path());
    cmd.args([
        "--basenca".as_ref(),
        base_nca.path.as_path(),
        update_nca.path.as_path(),
        "--romfsdir".as_ref(),
        romfs_dir.as_ref(), // ! hacshit seems to fail if the outdirs are in different mount places -_-
        "--exefsdir".as_ref(),
        exefs_dir.as_ref(),
    ]);
    cmd.stdout(Stdio::inherit());
    let output = cmd.output()?;
    if !output.status.success() {
        warn!(
            exit_code = ?output.status.code(),
            stderr = %String::from_utf8(output.stderr)?,
            "The process responsible for extracting romfs/exefs terminated improperly"
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

    println!(
        "{} {}",
        style("Extracted romfs/exefs").green().bold(),
        style(format!("({:?})", started.elapsed())).bold().dim()
    );
    let sp = default_spinner().with_message(format!(
        "{}",
        style("Cleaning up extracted NSPs data...").yellow().bold()
    ));

    // Early cleanup
    info!(dir = ?base_data_dir.path(), "Cleaning up");
    drop(base_data_dir);
    info!(dir = ?update_data_dir.path(), "Cleaning up");
    drop(update_data_dir);

    sp.println(format!(
        "{} {}",
        style("Cleaned up extracted NSPs data").green().bold(),
        style(format!("({:?})", started.elapsed())).bold().dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Packing romfs/exefs to NCA...").yellow().bold()
    ));

    let keyset_path = get_default_keyfile_path()?;
    let mut title_id = base_nca
        .title_id
        .ok_or_else(|| eyre!("Base NCA ({:?}) should've a TitleID", base_nca.path))?
        .to_lowercase(); //* Important
    title_id.truncate(TITLEID_SZ as _);
    info!("Packing romfs/exefs into a NCA");
    let mut cmd = Command::new(packer.path());
    cmd.args([
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
    ]);
    let output = cmd.output()?;
    if !output.status.success() {
        error!(exit_code = ?output.status.code(), stderr = %String::from_utf8(output.stderr)?);
        bail!("Failed to pack romfs/exefs into a NCA");
    }

    let mut patched_nca: Option<Nca> = None;
    for entry in WalkDir::new(&nca_dir)
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                patched_nca = Some(Nca::new(&extractor, entry.path())?);
                break;
            }
            _ => {}
        }
    }
    let patched_nca = patched_nca.ok_or_else(|| eyre!("Failed to pack romfs/exefs into a NCA"))?;

    sp.println(format!(
        "{} {}",
        style("Packed romfs/exefs to NCA").green().bold(),
        style(format!("({:?})", started.elapsed())).bold().dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Generating Meta NCA...").yellow().bold()
    ));

    info!("Generating Meta NCA from patched NCA & control NCA");
    let mut cmd = Command::new(packer.path());
    cmd.args([
        "--keyset".as_ref(),
        keyset_path.as_path(),
        "--type".as_ref(),
        "nca".as_ref(),
        "--ncatype".as_ref(),
        "meta".as_ref(),
        "--titletype".as_ref(),
        "application".as_ref(),
        "--programnca".as_ref(),
        patched_nca.path.as_path(),
        "--controlnca".as_ref(),
        control_nca.path.as_path(),
        "--titleid".as_ref(),
        title_id.as_ref(),
        "--outdir".as_ref(),
        nca_dir.as_path(),
    ]);
    let output = cmd.output()?;
    if !output.status.success() {
        error!(exit_code = ?output.status.code(), stderr = %String::from_utf8(output.stderr)?);
        bail!("Failed to generate Meta NCA from patched NCA & control NCA");
    }

    let patched_nsp_path = root_dir.join(format!("{}.nsp", title_id));

    sp.println(format!(
        "{} {}",
        style("Created Meta NCA").green().bold(),
        style(format!("({:?})", started.elapsed())).bold().dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Packing all NCAs to NSP...").yellow().bold()
    ));

    info!(
        patched_nsp = ?patched_nsp_path,
        "Packing all 3 NCAs into a NSP"
    );
    let mut cmd = Command::new(packer.path());
    cmd.args([
        "--keyset".as_ref(),
        keyset_path.as_path(),
        "--type".as_ref(),
        "nsp".as_ref(),
        "--ncadir".as_ref(),
        nca_dir.as_path(),
        "--titleid".as_ref(),
        title_id.as_ref(),
        "--outdir".as_ref(),
        root_dir.as_ref(),
    ]);
    let output = cmd.output()?;
    if !output.status.success() {
        error!(exit_code = ?output.status.code(), stderr = %String::from_utf8(output.stderr)?);
        bail!("Failed to Pack all 3 NCAs into a NSP");
    }

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-patched].nsp", title_id));
    info!(from = ?patched_nsp_path,to = ?dest,"Moving");
    move_file(patched_nsp_path, &dest)?;

    sp.finish_and_clear();
    println!(
        "{} {}",
        style("Packed all NCAs to NSP").green().bold(),
        style(format!("({:?})", started.elapsed())).bold().dim(),
    );
    println!(
        "{} {:?}",
        style("Patched NSP created at").cyan().bold(),
        dest
    );

    Ok(Nsp::from(dest)?)
}
