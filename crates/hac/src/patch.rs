use crate::{
    backend::{Backend, BackendKind},
    ticket::SHORT_TITLEID_LEN,
    vfs::{
        nca::{nca_with_filters, nca_with_kind, ContentType, Nca},
        nsp::Nsp,
    },
};
use common::{
    defines::{DEFAULT_PRODKEYS_PATH, DEFAULT_TITLEKEYS_PATH},
    utils::move_file,
};
use config::Config;

use super::ticket::TitleKey;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use std::{collections::HashSet, io::ErrorKind, path::Path};
use tempfile::tempdir_in;
use tracing::{debug, info, warn};

fn clear_titlekeys() -> Result<()> {
    match fs::remove_file(DEFAULT_TITLEKEYS_PATH.as_path()) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => {
            bail!(err)
        }
    }
}

/// Store TitleKeys to `DEFAULT_TITLEKEYS_PATH`.
fn store_titlekeys<'a, I>(keys: I) -> Result<()>
where
    I: Iterator<Item = &'a TitleKey>,
{
    info!(keyfile = ?DEFAULT_TITLEKEYS_PATH.as_path(), "Storing TitleKeys");
    fs::create_dir_all(DEFAULT_TITLEKEYS_PATH.parent().unwrap())?;
    fs::write(
        DEFAULT_TITLEKEYS_PATH.as_path(),
        keys.map(|key| key.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .map_err(|err| eyre!(err))
}

/// Repack romfs/exefs back to NSP.
pub fn repack_fs_data<N, E, R, O>(
    control_path: N,
    mut title_id: String,
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
    let readers = vec![
        Backend::new(BackendKind::Hactoolnet)?,
        Backend::new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::new(BackendKind::Hac2l)?];
    let packer = Backend::new(BackendKind::Hacpack)?;

    // Validating NCA as Control Type
    let control_nca = readers
        .iter()
        .map(|reader| Nca::new(reader, control_path.as_ref()).ok())
        .find(|nca| matches!(nca, Some(nca) if nca.content_type == ContentType::Control))
        .flatten()
        .ok_or_else(|| {
            eyre!(
                "'{}' is not a Control Type NCA",
                control_path.as_ref().display()
            )
        })?;

    // let mut title_id = control_nca
    //     .title_id
    //     .as_ref()
    //     .ok_or_else(|| eyre!("Failed to find TitleID in '{}'", control_nca.path.display()))?
    //     .to_lowercase();
    title_id.truncate(SHORT_TITLEID_LEN as _);
    debug!(?title_id, "Selected TitleID for packing");

    let temp_dir = tempdir_in(Config::load()?.temp_dir.as_path())?;

    // !Packing fs files to NCA
    let patched_nca_path = Nca::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        romfs_dir.as_ref(),
        exefs_dir.as_ref(),
        temp_dir.path(),
    )?;
    let patched_nca = readers
        .iter()
        // Could inspect and log the error if need be
        .map(|reader| Nca::new(reader, &patched_nca_path).ok())
        .find(|nca| nca.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Patched NCA"))?;

    // !Generating Meta NCA
    Nca::create_meta(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &patched_nca,
        &control_nca,
        temp_dir.path(),
    )?;

    // !Copying Control NCA
    let control_filename = control_nca
        .path
        .file_name()
        .expect("File should've a filename");
    fs::copy(&control_nca.path, temp_dir.path().join(control_filename))?;

    // !Packing NCAs to NSP
    let mut packed_nsp = Nsp::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        temp_dir.path(),
        outdir.as_ref(),
    )?;

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-repacked].nsp", title_id));
    info!(from = ?packed_nsp.path,to = ?dest,"Moving");
    move_file(&packed_nsp.path, &dest)?;
    packed_nsp.path = dest;

    _ = fs::remove_dir_all("hacpack_backup");

    Ok(packed_nsp)
}

/// Unpack NSP(s) to romfs/exefs.
pub fn unpack_nsp<O>(mut base: Nsp, mut patch: Option<Nsp>, outdir: O) -> Result<()>
where
    O: AsRef<Path>,
{
    #[cfg(not(feature = "android-proot"))]
    let config = Config::load()?;

    #[cfg(not(feature = "android-proot"))]
    let readers = vec![
        Backend::new(BackendKind::Hactoolnet)?,
        Backend::new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::new(BackendKind::Hac2l)?];
    #[cfg(not(feature = "android-proot"))]
    let nsp_extractor = Backend::new(BackendKind::from(config.nsp_extractor))?;
    #[cfg(feature = "android-proot")]
    let nsp_extractor = Backend::new(BackendKind::Hactool)?;
    #[cfg(not(feature = "android-proot"))]
    let nca_extractor = Backend::new(BackendKind::from(config.nca_extractor))?;
    #[cfg(feature = "android-proot")]
    let nca_extractor = Backend::new(BackendKind::Hac2l)?;

    let base_data_dir = outdir.as_ref().join("basedata");
    let patch_data_dir = outdir.as_ref().join("patchdata");

    // Important to do before any sort of unpacking
    // to avoid them being used when they were not supposed to
    clear_titlekeys()?;

    // !Extracting pfs0
    base.unpack(&nsp_extractor, &base_data_dir)?;
    // Setting TitleKeys
    if let Err(err) = base.derive_title_key(&base_data_dir) {
        warn!(?err);
    }

    if let Some(patch) = patch.as_mut() {
        // If patch is also to be extracted
        patch.unpack(&nsp_extractor, &patch_data_dir)?;
        // Setting TitleKeys
        if let Err(err) = patch.derive_title_key(&patch_data_dir) {
            warn!(?err);
        }
    }

    // !Storing TitleKeys file
    store_titlekeys([&base.title_key].iter().filter_map(|key| key.as_ref()))?;
    if let Some(patch) = patch.as_ref() {
        store_titlekeys(
            [&base.title_key, &patch.title_key]
                .iter()
                .filter_map(|key| key.as_ref()),
        )?;
    }

    // !Getting Base NCA
    let base_nca = readers
        .iter()
        .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
        .map(|reader| nca_with_kind(reader, &base_data_dir, ContentType::Program))
        .find(|filtered| filtered.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Base NCA in '{}'", base.path.display()))?
        .remove(0);
    debug!(?base_nca);

    if let Some(patch) = &patch {
        // !Getting Patch NCA
        let patch_nca = readers
            .iter()
            .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
            .map(|reader| nca_with_kind(reader, &patch_data_dir, ContentType::Program))
            .find(|filtered| filtered.is_some())
            .flatten()
            .ok_or_else(|| eyre!("Failed to find Patch NCA in '{}'", patch.path.display()))?
            .remove(0);
        debug!(?patch_nca);

        // !Unpacking fs files from NCAs
        _ = base_nca.unpack(
            &nca_extractor,
            &patch_nca,
            outdir.as_ref().join("romfs"),
            outdir.as_ref().join("exefs"),
        );
    }

    if patch.is_none() {
        // !Unpacking fs files from NCAs
        _ = base_nca.unpack(
            &nca_extractor,
            &base_nca,
            outdir.as_ref().join("romfs"),
            outdir.as_ref().join("exefs"),
        );
    }

    Ok(())
}

/// Apply update NSP to the base NSP.
pub fn update_nsp<O: AsRef<Path>>(base: &mut Nsp, update: &mut Nsp, outdir: O) -> Result<Nsp> {
    let config = Config::load()?;

    #[cfg(not(feature = "android-proot"))]
    let readers = vec![
        Backend::new(BackendKind::Hactoolnet)?,
        Backend::new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::new(BackendKind::Hac2l)?];
    #[cfg(not(feature = "android-proot"))]
    let nsp_extractor = Backend::new(BackendKind::from(config.nsp_extractor))?;
    #[cfg(feature = "android-proot")]
    let nsp_extractor = Backend::new(BackendKind::Hactool)?;
    #[cfg(not(feature = "android-proot"))]
    let nca_extractor = Backend::new(BackendKind::from(config.nca_extractor))?;
    #[cfg(feature = "android-proot")]
    let nca_extractor = Backend::new(BackendKind::Hac2l)?;
    let packer = Backend::new(BackendKind::Hacpack)?;

    let base_data_dir = tempdir_in(config.temp_dir.as_path())?;
    let update_data_dir = tempdir_in(config.temp_dir.as_path())?;
    fs::create_dir_all(base_data_dir.path())?;
    fs::create_dir_all(update_data_dir.path())?;

    clear_titlekeys()?;

    // !Extracting pfs0
    base.unpack(&nsp_extractor, base_data_dir.path())?;
    update.unpack(&nsp_extractor, update_data_dir.path())?;

    // Setting TitleKeys
    if let Err(err) = base.derive_title_key(base_data_dir.path()) {
        warn!(?err);
    }
    if let Err(err) = update.derive_title_key(update_data_dir.path()) {
        warn!(?err);
    }

    // !Storing TitleKeys file
    store_titlekeys(
        [&base.title_key, &update.title_key]
            .iter()
            .filter_map(|key| key.as_ref()),
    )?;

    // !Getting Base NCA
    let base_nca = readers
        .iter()
        .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
        .map(|reader| nca_with_kind(reader, base_data_dir.path(), ContentType::Program))
        .find(|filtered| filtered.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Base NCA in '{}'", base.path.display()))?
        .remove(0);
    debug!(?base_nca);

    // !Getting Update and Control NCA
    let filters = HashSet::from([ContentType::Program, ContentType::Control]);
    let mut filtered_ncas = readers
        .iter()
        .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
        .map(|reader| nca_with_filters(reader, update_data_dir.path(), &filters))
        .find(|filtered| {
            filters
                .iter()
                .map(|kind| filtered.get(kind))
                .all(|ncas| ncas.is_some())
        })
        .ok_or_else(|| {
            eyre!(
                "Failed to find Update and/or Control NCA in '{}'",
                update.path.display()
            )
        })?;
    let update_nca = filtered_ncas
        .remove(&ContentType::Program)
        .expect("Should be Some due the all() check")
        .remove(0);
    let mut control_nca = filtered_ncas
        .remove(&ContentType::Control)
        .expect("Should be Some due the all() check")
        .remove(0);
    debug!(?update_nca);
    debug!(?control_nca);

    let patch_dir = tempdir_in(config.temp_dir.as_path())?;
    let romfs_dir = patch_dir.path().join("romfs");
    let exefs_dir = patch_dir.path().join("exefs");
    // !Unpacking fs files from NCAs
    _ = base_nca.unpack(&nca_extractor, &update_nca, &romfs_dir, &exefs_dir); // !Ignoring err

    // TODO?: support for when main and update's titleid don't match
    // HacPack seems to pick update NCA's npdm TitleID in case of a id mismatch b/w base and update NCA
    let mut title_id = update_nca
        .title_id
        .as_ref()
        .ok_or_else(|| eyre!("Failed to find TitleID in '{}'", update_nca.path.display()))?
        .to_lowercase(); //* Important
    title_id.truncate(SHORT_TITLEID_LEN as _);
    debug!(?title_id, "Selected TitleID for packing");

    // let mut control_nca = if let Some(control_title_id) = control_nca.title_id.as_mut() {
    //     control_title_id.truncate(TITLEID_SIZE as _);
    //     let control_title_id = control_title_id.to_lowercase();
    //     if title_id != control_title_id {
    //         get_nca(&readers[0], base_data_dir.path(), NcaType::Control)
    //             .ok_or_else(|| eyre!("well fk"))?
    //             .remove(0)
    //     } else {
    //         control_nca
    //     }
    // } else {
    //     control_nca
    // };
    // debug!(?control_nca, "Changed Control NCA");

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

    // !Packing fs files to NCA
    let patched_nca_path = Nca::pack(
        &packer,
        &title_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &romfs_dir,
        &exefs_dir,
        &nca_dir,
    )?;
    let patched_nca = readers
        .iter()
        // Could inspect and log the error if need be
        .map(|reader| Nca::new(reader, &patched_nca_path).ok())
        .find(|nca| nca.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Patched NCA"))?;

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

    _ = fs::remove_dir_all("hacpack_backup");

    Ok(patched_nsp)
}
