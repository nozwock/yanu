use crate::{
    backend::{Backend, BackendKind},
    utils::{clear_titlekeys, store_titlekeys},
    vfs::{
        nca::{self, nca_with_kind},
        nsp::Nsp,
    },
};
use config::Config;
use eyre::{eyre, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// It corresponds to **(ProgramID, BaseUnpacked, UpdateUnpacked, MainRomFS, MainExeFS)**.
type UnpackedNSPData = (String, PathBuf, PathBuf, PathBuf, PathBuf);

/// Unpack NSPs to RomFS/ExeFS.\
/// **Note:** Whether `BaseUnpacked` path is valid depends on the `update` value.
pub fn unpack_nsp<O>(
    base: &mut Nsp,
    mut update: Option<&mut Nsp>,
    outdir: O,
    cfg: &Config,
) -> Result<UnpackedNSPData>
where
    O: AsRef<Path>,
{
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

    let base_data_dir = outdir.as_ref().join("basedata");
    let update_data_dir = outdir.as_ref().join("updatedata");
    let romfs_dir = outdir.as_ref().join("romfs");
    let exefs_dir = outdir.as_ref().join("exefs");

    // Important to do before any sort of unpacking
    // to avoid them being used when they were not supposed to
    clear_titlekeys()?;

    // !Extracting pfs0
    base.unpack(&nsp_extractor, &base_data_dir)?;
    // Setting TitleKeys
    if let Err(err) = base.derive_title_key(&base_data_dir) {
        warn!(?err);
    }

    // If update is also to be extracted
    if let Some(update) = update.as_deref_mut() {
        // !Extracting pfs0
        update.unpack(&nsp_extractor, &update_data_dir)?;
        // Setting TitleKeys
        if let Err(err) = update.derive_title_key(&update_data_dir) {
            warn!(?err);
        }
    }

    // !Storing TitleKeys file
    if let Some(update) = update.as_deref_mut() {
        store_titlekeys(
            [&base.title_key, &update.title_key]
                .iter()
                .filter_map(|key| key.as_ref()),
        )?;
    } else {
        store_titlekeys([&base.title_key].iter().filter_map(|key| key.as_ref()))?;
    }

    // !Getting Base NCA
    let base_nca = readers
        .iter()
        .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
        .map(|reader| nca_with_kind(reader, &base_data_dir, nca::ContentType::Program))
        .find(|filtered| filtered.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Base NCA in '{}'", base.path.display()))?
        .remove(0);
    debug!(?base_nca);

    if let Some(patch) = update.as_deref() {
        // !Getting Patch NCA
        let patch_nca = readers
            .iter()
            .inspect(|reader| info!("Using {:?} as reader", reader.kind()))
            .map(|reader| nca_with_kind(reader, &update_data_dir, nca::ContentType::Program))
            .find(|filtered| filtered.is_some())
            .flatten()
            .ok_or_else(|| eyre!("Failed to find Patch NCA in '{}'", patch.path.display()))?
            .remove(0);
        debug!(?patch_nca);

        // !Unpacking FS files from NCAs
        _ = base_nca.unpack_all(&nca_extractor, &patch_nca, &romfs_dir, &exefs_dir);
    } else {
        // !Unpacking FS files from NCAs
        _ = base_nca.unpack_all(&nca_extractor, &base_nca, &romfs_dir, &exefs_dir);
    }

    Ok((
        base_nca.get_program_id().to_lowercase(),
        base_data_dir,
        update_data_dir,
        romfs_dir,
        exefs_dir,
    ))
}
