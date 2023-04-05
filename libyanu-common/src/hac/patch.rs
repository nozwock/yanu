use crate::{
    config::Config,
    defines::{DEFAULT_PRODKEYS_PATH, DEFAULT_TITLEKEYS_PATH},
    hac::{
        backend::{Backend, BackendKind},
        rom::{get_filtered_ncas, Nca, NcaType},
    },
    utils::move_file,
};

use super::rom::Nsp;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use std::{
    cmp,
    collections::HashSet,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
};
use tempfile::tempdir_in;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

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

    let temp_dir = tempdir_in(Config::load()?.temp_dir.as_path())?;

    let patched_path = Nca::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        romfs_dir.as_ref(),
        exefs_dir.as_ref(),
        temp_dir.path(),
    )?;
    // TODO: Fix this later
    let patched = Nca::new(&extractor, &patched_path)?;

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
    let config = Config::load()?;

    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let extractor = vec![
        Backend::new(BackendKind::Hactoolnet)?,
        Backend::new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let mut extractor = vec![
        Backend::new(BackendKind::Hactool)?,
        Backend::new(BackendKind::Hac2l)?,
    ];
    let packer = Backend::new(BackendKind::Hacpack)?;

    // Clearing TitleKeys
    fs::create_dir_all(DEFAULT_TITLEKEYS_PATH.parent().unwrap())?;
    match fs::remove_file(DEFAULT_TITLEKEYS_PATH.as_path()) {
        Err(ref err) if err.kind() == io::ErrorKind::PermissionDenied => {
            bail!("{}", err);
        }
        _ => {}
    }

    let base_data_dir = tempdir_in(config.temp_dir.as_path())?;
    let update_data_dir = tempdir_in(config.temp_dir.as_path())?;
    fs::create_dir_all(base_data_dir.path())?;
    fs::create_dir_all(update_data_dir.path())?;

    // !Extracting pfs0
    base.unpack(
        extractor.first().expect("should've atleast 1 backend"),
        base_data_dir.path(),
    )?;
    update.unpack(
        extractor.first().expect("should've atleast 1 backend"),
        update_data_dir.path(),
    )?;

    // !Writing TitleKeys file
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

    // Removing hactool on Android
    // Since it's not useful after extracting NSPs
    #[cfg(feature = "android-proot")]
    extractor.remove(0);

    // !Getting Base NCA
    let mut readers = extractor.iter();
    let filters = HashSet::from([NcaType::Program]);
    let base_nca = loop {
        match readers.next() {
            Some(reader) => {
                info!("Using {:?} as reader", reader.kind());
                let filtered_ncas = get_filtered_ncas(reader, base_data_dir.path(), &filters);
                if filters
                    .iter()
                    .map(|kind| filtered_ncas.get(kind))
                    .all(|ncas| ncas.is_some())
                {
                    break Some(filtered_ncas);
                }
            }
            None => break None,
        }
    }
    .ok_or_else(|| eyre!("Failed to find Base NCA in \"{}\"", base.path.display()))?
    .remove(&NcaType::Program)
    .expect("Should be Some due the all() check")
    .remove(0);
    debug!(?base_nca);

    // !Getting Update and Control NCA
    let mut readers = extractor.iter();
    let filters = HashSet::from([NcaType::Program, NcaType::Control]);
    let mut filtered_ncas = loop {
        match readers.next() {
            Some(reader) => {
                info!("Using {:?} as reader", reader.kind());
                let filtered_ncas = get_filtered_ncas(reader, update_data_dir.path(), &filters);
                if filters
                    .iter()
                    .map(|kind| filtered_ncas.get(kind))
                    .all(|ncas| ncas.is_some())
                {
                    break Some(filtered_ncas);
                }
            }
            None => break None,
        }
    }
    .ok_or_else(|| {
        eyre!(
            "Failed to find Update and/or Control NCA in \"{}\"",
            update.path.display()
        )
    })?;
    let update_nca = filtered_ncas
        .remove(&NcaType::Program)
        .expect("Should be Some due the all() check")
        .remove(0);
    let mut control_nca = filtered_ncas
        .remove(&NcaType::Control)
        .expect("Should be Some due the all() check")
        .remove(0);
    debug!(?update_nca);
    debug!(?control_nca);

    let patch_dir = tempdir_in(config.temp_dir.as_path())?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    // !Unpacking fs files from NCAs
    _ = base_nca.unpack(
        &extractor.first().expect("should've atleast 1 backend"),
        &update_nca,
        &romfs_dir,
        &exefs_dir,
    ); // Ignoring err

    // !Moving Control NCA
    let nca_dir = patch_dir.path().join("nca");
    fs::create_dir_all(&nca_dir)?;
    let control_nca_filename = control_nca
        .path
        .file_name()
        .expect("File should've a filename");
    fs::rename(&control_nca.path, nca_dir.join(control_nca_filename))?;
    control_nca.path = nca_dir.join(control_nca_filename);

    // Early cleanup
    info!(dir = ?base_data_dir.path(), "Cleaning up");
    if let Err(err) = base_data_dir.close() {
        warn!(?err);
    }
    info!(dir = ?update_data_dir.path(), "Cleaning up");
    if let Err(err) = update_data_dir.close() {
        warn!(?err);
    }

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

    // !Packing fs files to NCA
    let patched_nca_path = Nca::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &romfs_dir,
        &exefs_dir,
        &nca_dir,
    )?;
    let mut readers = extractor.iter();
    let patched_nca = loop {
        match readers.next() {
            Some(reader) => {
                if let Ok(nca) = Nca::new(reader, &patched_nca_path) {
                    break Some(nca);
                }
            }
            None => break None,
        }
    }
    .ok_or_else(|| eyre!("Invalid Patched NCA"))?;

    // !Generating Meta NCA
    Nca::create_meta(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &patched_nca,
        &control_nca,
        &nca_dir,
    )?;

    // !Packing NCAs to NSP
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
    patched_nsp.path = dest;

    if let Err(err) = patch_dir.close() {
        warn!(?err);
    }

    Ok(patched_nsp)
}
