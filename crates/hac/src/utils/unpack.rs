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
use std::path::Path;
use tracing::{debug, info, warn};

/// Unpack NSP(s) to romfs/exefs.
pub fn unpack_nsp<O>(mut base: Nsp, mut patch: Option<Nsp>, outdir: O) -> Result<()>
where
    O: AsRef<Path>,
{
    #[cfg(not(feature = "android-proot"))]
    let config = Config::load()?;

    #[cfg(not(feature = "android-proot"))]
    let readers = vec![
        Backend::try_new(BackendKind::Hactoolnet)?,
        Backend::try_new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::new(BackendKind::Hac2l)?];
    #[cfg(not(feature = "android-proot"))]
    let nsp_extractor = Backend::try_new(BackendKind::from(config.nsp_extractor))?;
    #[cfg(feature = "android-proot")]
    let nsp_extractor = Backend::new(BackendKind::Hactool)?;
    #[cfg(not(feature = "android-proot"))]
    let nca_extractor = Backend::try_new(BackendKind::from(config.nca_extractor))?;
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
        .map(|reader| nca_with_kind(reader, &base_data_dir, nca::ContentType::Program))
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
            .map(|reader| nca_with_kind(reader, &patch_data_dir, nca::ContentType::Program))
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
