use crate::{
    defines::DEFAULT_KEYFILE_PATH,
    hac::{
        backend::Backend,
        rom::{Nca, NcaType},
    },
    utils::move_file,
};

use super::rom::Nsp;
use console::style;
use eyre::{bail, eyre, Result};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::{
    cmp,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
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

fn fetch_ncas<P: AsRef<Path>>(extractor: &Backend, from: P) -> Vec<(PathBuf, Result<Nca>)> {
    let mut ncas = vec![];
    for entry in WalkDir::new(from.as_ref())
        .min_depth(1)
        // Sort by descending order of sizes
        .sort_by_key(|entry| {
            cmp::Reverse(
                entry
                    .metadata()
                    .expect(&format!("Failed to read metadata of {:?}", entry.path()))
                    .len(),
            )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        match entry.path().extension().and_then(OsStr::to_str) {
            Some("nca") => {
                ncas.push((entry.path().to_owned(), Nca::new(extractor, entry.path())));
            }
            _ => {}
        }
    }
    ncas
}

pub fn patch_nsp<O: AsRef<Path>>(base: &mut Nsp, update: &mut Nsp, outdir: O) -> Result<Nsp> {
    //* It's a mess, ik and I'm not sry ;-;
    let started = time::Instant::now();

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let extractor = Backend::new(Backend::HACTOOLNET)?;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    let fallback_extractor = Backend::new(Backend::HAC2L)?;
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

    base.unpack(&extractor, base_data_dir.path())?;
    update.unpack(&extractor, update_data_dir.path())?;

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
    'walk: for (path, mut nca) in fetch_ncas(&extractor, base_data_dir.path()) {
        let mut fallback: bool = false;
        loop {
            match nca {
                Ok(nca) => match nca.content_type {
                    NcaType::Program => {
                        base_nca = Some(nca);
                        break 'walk;
                    }
                    _ => {}
                },
                Err(err) => {
                    warn!("{}", err);
                    #[cfg(any(target_os = "windows", target_os = "linux"))]
                    {
                        if !fallback {
                            info!("Using fallback extractor {:?}", fallback_extractor.kind());
                            nca = Nca::new(&fallback_extractor, &path);
                            fallback = true;
                            continue;
                        }
                    }
                }
            }
            break;
        }
    }
    let base_nca = base_nca
        .ok_or_else(|| eyre!("Couldn't find a Base NCA (Program Type) in {:?}", base.path))?;
    debug!(?base_nca);

    let mut control_nca: Option<Nca> = None;
    let mut update_nca: Option<Nca> = None;
    for (path, mut nca) in fetch_ncas(&extractor, update_data_dir.path()) {
        let mut fallback = false;
        loop {
            match nca {
                Ok(nca) => match nca.content_type {
                    NcaType::Program => {
                        if update_nca.is_none() {
                            update_nca = Some(nca);
                        }
                    }
                    NcaType::Control => {
                        if control_nca.is_none() {
                            control_nca = Some(nca);
                        }
                    }
                    _ => {}
                },
                Err(err) => {
                    warn!("{}", err);
                    #[cfg(any(target_os = "windows", target_os = "linux"))]
                    {
                        if !fallback {
                            info!("Using fallback extractor {:?}", fallback_extractor.kind());
                            nca = Nca::new(&fallback_extractor, &path);
                            fallback = true;
                            continue;
                        }
                    }
                }
            }
            break;
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

    println!("{}", style("Unpacking NCAs...").yellow().bold());

    let patch_dir = TempDir::new_in(&temp_dir, "patch")?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    _ = base_nca.unpack(&extractor, &update_nca, &romfs_dir, &exefs_dir);

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
        style("Unpacked NCAs").green().bold(),
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim()
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
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Packing romfs/exefs to NCA...").yellow().bold()
    ));

    let keyfile_path = DEFAULT_KEYFILE_PATH.as_path();
    let mut title_id = base_nca
        .title_id
        .ok_or_else(|| eyre!("Base NCA ({:?}) should've a TitleID", base_nca.path))?
        .to_lowercase(); //* Important
    title_id.truncate(TITLEID_SZ as _);

    let patched_nca = Nca::pack(
        &extractor,
        &packer,
        &title_id,
        keyfile_path,
        &romfs_dir,
        &exefs_dir,
        &nca_dir,
    )?;

    sp.println(format!(
        "{} {}",
        style("Packed romfs/exefs to NCA").green().bold(),
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Generating Meta NCA...").yellow().bold()
    ));

    Nca::create_meta(
        &packer,
        &title_id,
        keyfile_path,
        &patched_nca,
        &control_nca,
        &nca_dir,
    )?;

    sp.println(format!(
        "{} {}",
        style("Generated Meta NCA").green().bold(),
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim(),
    ));
    sp.set_message(format!(
        "{}",
        style("Packing NCAs to NSP...").yellow().bold()
    ));

    let patched_nsp = Nsp::pack(&packer, &title_id, keyfile_path, &nca_dir, root_dir)?;

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-patched].nsp", title_id));
    info!(from = ?patched_nsp.path,to = ?dest,"Moving");
    move_file(&patched_nsp.path, &dest)?;

    sp.finish_and_clear();
    println!(
        "{} {}",
        style("Packed NCAs to NSP").green().bold(),
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim(),
    );
    println!(
        "{} {:?}",
        style("Patched NSP created at").cyan().bold(),
        dest
    );

    println!("{}", style("Cleaning up...").yellow().bold());
    Ok(Nsp::new(dest)?)
}
