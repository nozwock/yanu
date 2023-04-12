use std::{collections::HashSet, path::Path};

use common::{defines::DEFAULT_PRODKEYS_PATH, utils::move_file};
use config::Config;
use eyre::{eyre, Result};
use fs_err as fs;
use tempfile::tempdir_in;
use tracing::{debug, info, warn};

use crate::{
    backend::{Backend, BackendKind},
    utils::{clear_titlekeys, store_titlekeys},
    vfs::{
        nca::{self, nca_with_filters, nca_with_kind, Nca},
        nsp::Nsp,
        ticket::SHORT_TITLEID_LEN,
    },
};

use super::hacpack_cleanup_install;

// TODO: update can be reduced to a combination of unpack and repack

/// Apply update NSP to the base NSP.
pub fn update_nsp<O: AsRef<Path>>(base: &mut Nsp, update: &mut Nsp, outdir: O) -> Result<Nsp> {
    let config = Config::load()?;
    let curr_dir = std::env::current_dir()?;
    let _hacpack_cleanup_bind = hacpack_cleanup_install!(curr_dir);

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
        .map(|reader| nca_with_kind(reader, base_data_dir.path(), nca::ContentType::Program))
        .find(|filtered| filtered.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Base NCA in '{}'", base.path.display()))?
        .remove(0);
    debug!(?base_nca);

    // !Getting Update and Control NCA
    let filters = HashSet::from([nca::ContentType::Program, nca::ContentType::Control]);
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
        .remove(&nca::ContentType::Program)
        .expect("Should be Some due the all() check")
        .remove(0);
    let mut control_nca = filtered_ncas
        .remove(&nca::ContentType::Control)
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
    // maybe handle this by having a override flag for TitleID
    // once unpack/repack combo is being used for updating
    let mut title_id = base_nca
        .title_id
        .as_ref()
        .ok_or_else(|| eyre!("Failed to find TitleID in '{}'", base_nca.path.display()))?
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

    Ok(patched_nsp)
}
