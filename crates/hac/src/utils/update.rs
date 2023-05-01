use std::{collections::HashSet, path::Path};

use common::defines::DEFAULT_PRODKEYS_PATH;
use config::Config;
use eyre::{eyre, Result};
use fs_err as fs;
use tracing::{debug, info, warn};

use crate::{
    backend::{Backend, BackendKind},
    utils::{clear_titlekeys, store_titlekeys},
    vfs::{
        nacp::{get_nacp_file, NacpData},
        nca::{self, nca_with_filters, nca_with_kind, Nca},
        nsp::Nsp,
    },
};

use super::hacpack_cleanup_install;

/// Apply update NSP to the base NSP.
pub fn update_nsp<O>(
    base: &mut Nsp,
    update: &mut Nsp,
    program_id: Option<&str>,
    outdir: O,
    cfg: &Config,
) -> Result<(Nsp, NacpData, String)>
where
    O: AsRef<Path>,
{
    let curr_dir = std::env::current_dir()?;
    let _hacpack_cleanup_bind = hacpack_cleanup_install!(curr_dir);

    #[cfg(not(feature = "android-proot"))]
    let readers = vec![
        Backend::try_new(BackendKind::Hactoolnet)?,
        Backend::try_new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::try_new(BackendKind::Hac2l)?];
    #[cfg(not(feature = "android-proot"))]
    let nsp_extractor = Backend::try_new(BackendKind::from(cfg.nsp_extractor))?;
    #[cfg(feature = "android-proot")]
    let nsp_extractor = Backend::try_new(BackendKind::Hactool)?;
    #[cfg(not(feature = "android-proot"))]
    let nca_extractor = Backend::try_new(BackendKind::from(cfg.nca_extractor))?;
    #[cfg(feature = "android-proot")]
    let nca_extractor = Backend::try_new(BackendKind::Hac2l)?;
    let packer = Backend::try_new(BackendKind::Hacpack)?;

    let base_data_dir = tempfile::tempdir_in(&cfg.temp_dir)?;
    let update_data_dir = tempfile::tempdir_in(&cfg.temp_dir)?;
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

    // Getting Nacp data
    let control_romfs_dir = tempfile::tempdir_in(&cfg.temp_dir)?;
    control_nca.unpack_romfs(&nca_extractor, control_romfs_dir.path())?;
    let nacp_data =
        NacpData::try_new(get_nacp_file(control_romfs_dir.path()).ok_or_else(|| {
            eyre!("Couldn't find NACP file, should be due to improper extraction")
        })?)?;
    if let Err(err) = control_romfs_dir.close() {
        warn!(%err);
    }

    let fs_dir = tempfile::tempdir_in(&cfg.temp_dir)?;
    let romfs_dir = fs_dir.path().join("romfs");
    let exefs_dir = fs_dir.path().join("exefs");
    // !Unpacking fs files from NCAs
    _ = base_nca.unpack_all(&nca_extractor, &update_nca, &romfs_dir, &exefs_dir); // !Ignoring err

    let program_id = match program_id {
        Some(program_id) => program_id.into(),
        None => base_nca.get_program_id().to_lowercase(),
    };
    debug!(?program_id, "Selected TitleID for packing");

    // !Moving Control NCA
    let nca_dir = tempfile::tempdir_in(&cfg.temp_dir)?;
    fs::create_dir_all(nca_dir.path())?;
    let control_nca_filename = control_nca
        .path
        .file_name()
        .expect("File should've a filename");
    fs::rename(&control_nca.path, nca_dir.path().join(control_nca_filename))?;
    control_nca.path = nca_dir.path().join(control_nca_filename);

    // Early cleanup
    if let Err(err) = base_data_dir.close() {
        warn!(?err);
    }
    if let Err(err) = update_data_dir.close() {
        warn!(?err);
    }

    // !Packing fs files to NCA
    let patched_nca = Nca::pack_program(
        readers.iter(),
        &packer,
        &program_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &romfs_dir,
        &exefs_dir,
        nca_dir.path(),
    )?;

    // Cleaning up extracted FS files
    if let Err(err) = fs_dir.close() {
        warn!(?err);
    }

    // !Generating Meta NCA
    Nca::create_meta(
        &packer,
        &program_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        &patched_nca,
        &control_nca,
        nca_dir.path(),
        &cfg.temp_dir,
    )?;

    // !Packing NCAs to NSP
    let patched_nsp = Nsp::pack(
        &packer,
        &program_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        nca_dir.path(),
        outdir.as_ref(),
    )?;

    Ok((patched_nsp, nacp_data, program_id))
}
