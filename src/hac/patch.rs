use crate::{
    defines::{DEFAULT_PRODKEYS_PATH, DEFAULT_TITLEKEYS_PATH, TEMP_DIR_IN},
    hac::{
        backend::{Backend, BackendKind},
        rom::{Nca, NcaType},
    },
    utils::move_file,
};

use super::rom::Nsp;
use console::style;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::{
    cmp,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    time::{self, Duration},
};
use tempfile::tempdir_in;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

#[allow(unused)]
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
                    .unwrap_or_else(|_| {
                        panic!("Failed to read metadata of \"{}\"", entry.path().display())
                    })
                    .len(),
            )
        })
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Some("nca") = entry.path().extension().and_then(OsStr::to_str) {
            ncas.push((entry.path().to_owned(), Nca::new(extractor, entry.path())));
        }
    }
    ncas
}

pub fn repack_to_nsp<N, E, R, O>(
    control_path: N,
    romfs_dir: R,
    exefs_dir: E,
    outdir: O,
) -> Result<Nsp>
where
    N: AsRef<Path>,
    E: AsRef<Path>,
    R: AsRef<Path>,
    O: AsRef<Path>,
{
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let extractor = Backend::new(BackendKind::Hactoolnet)?;
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let fallback_extractor = Backend::new(BackendKind::Hac2l)?;
    #[cfg(feature = "android-proot")]
    let extractor = Backend::new(BackendKind::Hac2l)?;
    let packer = Backend::new(BackendKind::Hacpack)?;

    let control = match Nca::new(&extractor, control_path.as_ref()) {
        Ok(control) => match control.content_type {
            NcaType::Control => Some(control),
            _ => None,
        },
        Err(err) => {
            warn!("{}", err);
            #[cfg(all(
                target_arch = "x86_64",
                any(target_os = "windows", target_os = "linux")
            ))]
            {
                info!("Using fallback extractor {:?}", fallback_extractor.kind());
                match Nca::new(&fallback_extractor, control_path.as_ref()) {
                    Ok(control) => match control.content_type {
                        NcaType::Control => Some(control),
                        _ => None,
                    },
                    Err(err) => {
                        warn!("{}", err);
                        None
                    }
                }
            }
            #[cfg(feature = "android-proot")]
            {
                None
            }
        }
    };
    let control = control.ok_or_else(|| {
        eyre!(
            "\"{}\" is not a Control Type NCA",
            control_path.as_ref().display()
        )
    })?;

    let mut title_id = control
        .title_id
        .as_ref()
        .ok_or_else(|| eyre!("Failed to find TitleID in \"{}\"", control.path.display()))?
        .to_lowercase();
    title_id.truncate(TITLEID_SZ as _);

    let temp_dir = tempdir_in(TEMP_DIR_IN.as_path())?;

    let patched = Nca::pack(
        &extractor,
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        romfs_dir.as_ref(),
        exefs_dir.as_ref(),
        temp_dir.path(),
    )?;

    Nca::create_meta(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &patched,
        &control,
        temp_dir.path(),
    )?;

    let control_filename = control.path.file_name().expect("File should've a filename");
    fs::copy(&control.path, temp_dir.path().join(control_filename))?;

    let mut packed = Nsp::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        temp_dir.path(),
        outdir.as_ref(),
    )?;

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-repacked].nsp", title_id));
    info!(from = ?packed.path,to = ?dest,"Moving");
    move_file(&packed.path, &dest)?;
    packed.path = dest;

    Ok(packed)
}

pub fn unpack_to_fs<O>(mut base: Nsp, mut patch: Option<Nsp>, outdir: O) -> Result<()>
where
    O: AsRef<Path>,
{
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let extractor = Backend::new(BackendKind::Hactoolnet)?;
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let fallback_extractor = Backend::new(BackendKind::Hac2l)?;
    #[cfg(feature = "android-proot")]
    let extractor = Backend::new(BackendKind::Hactool)?;

    fs::create_dir_all(DEFAULT_TITLEKEYS_PATH.parent().unwrap())?;
    match fs::remove_file(DEFAULT_TITLEKEYS_PATH.as_path()) {
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => {
            bail!("{}", err);
        }
        _ => {}
    }

    let base_data_dir = outdir.as_ref().join("basedata");
    let patch_data_dir = outdir.as_ref().join("patchdata");

    base.unpack(&extractor, &base_data_dir)?;
    if let Err(err) = base.derive_title_key(&base_data_dir) {
        warn!(?err);
    }

    if let Some(patch) = patch.as_mut() {
        patch.unpack(&extractor, &patch_data_dir)?;
        if let Err(err) = patch.derive_title_key(&patch_data_dir) {
            warn!(?err);
        }
    }

    // switching to hac2l for NCAs
    #[cfg(feature = "android-proot")]
    let extractor = Backend::new(BackendKind::Hac2l)?;

    info!(keyfile = ?DEFAULT_TITLEKEYS_PATH.as_path(), "Storing TitleKeys");
    let mut contents = String::new();
    if let Some(key) = &base.title_key {
        contents.push_str(&key.to_string());
        contents.push('\n');
    }
    if let Some(patch) = patch.as_ref() {
        if let Some(key) = &patch.title_key {
            contents.push_str(&key.to_string());
            contents.push('\n');
        }
    }
    fs::write(DEFAULT_TITLEKEYS_PATH.as_path(), contents)?;

    let mut base_nca: Option<Nca> = None;
    'walk: for (path, mut nca) in fetch_ncas(&extractor, &base_data_dir) {
        let mut fallback: bool = false;
        loop {
            match nca {
                Ok(nca) => {
                    if nca.content_type == NcaType::Program {
                        base_nca = Some(nca);
                        break 'walk;
                    }
                }
                Err(err) => {
                    warn!("{}", err);
                    #[cfg(all(
                        target_arch = "x86_64",
                        any(target_os = "windows", target_os = "linux")
                    ))]
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
    let base_nca = base_nca.ok_or_else(|| {
        eyre!(
            "Couldn't find a Base NCA (Program Type) in \"{}\"",
            base_data_dir.display()
        )
    })?;
    debug!(?base_nca);

    let mut patch_nca: Option<Nca> = None;
    if patch.is_some() {
        'walk: for (path, mut nca) in fetch_ncas(&extractor, &patch_data_dir) {
            let mut fallback: bool = false;
            loop {
                match nca {
                    Ok(nca) => {
                        if nca.content_type == NcaType::Program {
                            patch_nca = Some(nca);
                            break 'walk;
                        }
                    }
                    Err(err) => {
                        warn!("{}", err);
                        #[cfg(all(
                            target_arch = "x86_64",
                            any(target_os = "windows", target_os = "linux")
                        ))]
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
        let patch_nca = patch_nca.ok_or_else(|| {
            eyre!(
                "Couldn't find a Base NCA (Program Type) in \"{}\"",
                base_data_dir.display()
            )
        })?;
        debug!(?patch_nca);

        _ = base_nca.unpack(
            &extractor,
            &patch_nca,
            outdir.as_ref().join("romfs"),
            outdir.as_ref().join("exefs"),
        );
    }

    if patch.is_none() {
        _ = base_nca.unpack(
            &extractor,
            &base_nca,
            outdir.as_ref().join("romfs"),
            outdir.as_ref().join("exefs"),
        );
    }

    Ok(())
}

pub fn patch_nsp<O: AsRef<Path>>(base: &mut Nsp, update: &mut Nsp, outdir: O) -> Result<Nsp> {
    //* It's a mess, ik and I'm not sry ;-;
    let started = time::Instant::now();

    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let extractor = Backend::new(BackendKind::Hactoolnet)?;
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let fallback_extractor = Backend::new(BackendKind::Hac2l)?;
    #[cfg(feature = "android-proot")]
    let extractor = Backend::new(BackendKind::Hactool)?;
    let packer = Backend::new(BackendKind::Hacpack)?;

    fs::create_dir_all(DEFAULT_TITLEKEYS_PATH.parent().unwrap())?;
    match fs::remove_file(DEFAULT_TITLEKEYS_PATH.as_path()) {
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => {
            bail!("{}", err);
        }
        _ => {}
    }

    let base_data_dir = tempdir_in(TEMP_DIR_IN.as_path())?;
    let update_data_dir = tempdir_in(TEMP_DIR_IN.as_path())?;
    fs::create_dir_all(base_data_dir.path())?;
    fs::create_dir_all(update_data_dir.path())?;

    println!("{}", style("Extracting NSP data...").yellow().bold());

    base.unpack(&extractor, base_data_dir.path())?;
    update.unpack(&extractor, update_data_dir.path())?;

    // switching to hac2l for NCAs
    #[cfg(feature = "android-proot")]
    let extractor = Backend::new(BackendKind::Hac2l)?;

    if let Err(err) = base.derive_title_key(base_data_dir.path()) {
        warn!(?err);
    }
    if let Err(err) = update.derive_title_key(update_data_dir.path()) {
        warn!(?err);
    }

    info!(keyfile = ?DEFAULT_TITLEKEYS_PATH.as_path(), "Storing TitleKeys");
    let mut contents = String::new();
    if let Some(key) = &base.title_key {
        contents.push_str(&key.to_string());
        contents.push('\n');
    }
    if let Some(key) = &update.title_key {
        contents.push_str(&key.to_string());
        contents.push('\n');
    }
    fs::write(DEFAULT_TITLEKEYS_PATH.as_path(), contents)?;

    let mut base_nca: Option<Nca> = None;
    'walk: for (path, mut nca) in fetch_ncas(&extractor, base_data_dir.path()) {
        let mut fallback: bool = false;
        loop {
            match nca {
                Ok(nca) => {
                    if nca.content_type == NcaType::Program {
                        base_nca = Some(nca);
                        break 'walk;
                    }
                }
                Err(err) => {
                    warn!(?err);
                    #[cfg(all(
                        target_arch = "x86_64",
                        any(target_os = "windows", target_os = "linux")
                    ))]
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
    let base_nca = base_nca.ok_or_else(|| {
        eyre!(
            "Couldn't find a Base NCA (Program Type) in \"{}\"",
            base.path.display()
        )
    })?;
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
                    warn!(?err);
                    #[cfg(all(
                        target_arch = "x86_64",
                        any(target_os = "windows", target_os = "linux")
                    ))]
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
            "Couldn't find a Update NCA (Program Type) in \"{}\"",
            update.path.display()
        )
    })?;
    debug!(?update_nca);
    let mut control_nca = control_nca.ok_or_else(|| {
        eyre!(
            "Couldn't find a Control NCA (Control Type) in \"{}\"",
            update.path.display()
        )
    })?;
    debug!(?control_nca);

    println!("{}", style("Unpacking NCAs...").yellow().bold());

    let patch_dir = tempdir_in(TEMP_DIR_IN.as_path())?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    _ = base_nca.unpack(&extractor, &update_nca, &romfs_dir, &exefs_dir);

    let nca_dir = patch_dir.path().join("nca");
    fs::create_dir_all(&nca_dir)?;
    let control_nca_filename = control_nca
        .path
        .file_name()
        .expect("File should've a filename");
    fs::rename(&control_nca.path, nca_dir.join(control_nca_filename))?;
    control_nca.path = nca_dir.join(control_nca_filename);

    // println!(
    //     "{} {}",
    //     style("Unpacked NCAs").green().bold(),
    //     style(format!("({})", HumanDuration(started.elapsed())))
    //         .bold()
    //         .dim()
    // );
    // let sp = default_spinner().with_message(format!(
    //     "{}",
    //     style("Cleaning up extracted NSPs data...").yellow().bold()
    // ));

    println!(
        "{}",
        style("Cleaning up extracted NSPs data...").yellow().bold()
    );
    // Early cleanup
    info!(dir = ?base_data_dir.path(), "Cleaning up");
    if let Err(err) = base_data_dir.close() {
        warn!(?err);
    }
    info!(dir = ?update_data_dir.path(), "Cleaning up");
    if let Err(err) = update_data_dir.close() {
        warn!(?err);
    }

    // sp.println(format!(
    //     "{} {}",
    //     style("Cleaned up extracted NSPs data").green().bold(),
    //     style(format!("({})", HumanDuration(started.elapsed())))
    //         .bold()
    //         .dim(),
    // ));
    // sp.set_message(format!(
    //     "{}",
    //     style("Packing romfs/exefs to NCA...").yellow().bold()
    // ));

    let mut title_id = base_nca
        .title_id
        .ok_or_else(|| {
            eyre!(
                "Base NCA (\"{}\") should've a TitleID",
                base_nca.path.display()
            )
        })?
        .to_lowercase(); //* Important
    title_id.truncate(TITLEID_SZ as _);

    println!("{}", style("Packing romfs/exefs to NCA...").yellow().bold());
    let patched_nca = Nca::pack(
        &extractor,
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &romfs_dir,
        &exefs_dir,
        &nca_dir,
    )?;

    // sp.println(format!(
    //     "{} {}",
    //     style("Packed romfs/exefs to NCA").green().bold(),
    //     style(format!("({})", HumanDuration(started.elapsed())))
    //         .bold()
    //         .dim(),
    // ));
    // sp.set_message(format!(
    //     "{}",
    //     style("Generating Meta NCA...").yellow().bold()
    // ));

    println!("{}", style("Generating Meta NCA...").yellow().bold());
    Nca::create_meta(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &patched_nca,
        &control_nca,
        &nca_dir,
    )?;

    // sp.println(format!(
    //     "{} {}",
    //     style("Generated Meta NCA").green().bold(),
    //     style(format!("({})", HumanDuration(started.elapsed())))
    //         .bold()
    //         .dim(),
    // ));
    // sp.set_message(format!(
    //     "{}",
    //     style("Packing NCAs to NSP...").yellow().bold()
    // ));

    println!("{}", style("Packing NCAs to NSP...").yellow().bold());
    let mut patched_nsp = Nsp::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &nca_dir,
        outdir.as_ref(),
    )?;

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-patched].nsp", title_id));
    info!(from = ?patched_nsp.path,to = ?dest,"Moving");
    move_file(&patched_nsp.path, &dest)?;

    // sp.finish_and_clear();
    // println!(
    //     "{} {}",
    //     style("Packed NCAs to NSP").green().bold(),
    //     style(format!("({})", HumanDuration(started.elapsed())))
    //         .bold()
    //         .dim(),
    // );
    println!(
        "{} \"{}\" {}",
        style("Patched NSP created at").green().bold(),
        dest.display(),
        style(format!("({})", HumanDuration(started.elapsed())))
            .bold()
            .dim(),
    );

    patched_nsp.path = dest;

    println!("{}", style("Cleaning up...").yellow().bold());
    if let Err(err) = patch_dir.close() {
        warn!(?err);
    }

    Ok(patched_nsp)
}
