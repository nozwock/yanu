use common::{defines::DEFAULT_PRODKEYS_PATH, utils::move_file};
use config::Config;
use eyre::{eyre, Result};
use fs_err as fs;
use std::path::Path;
use tempfile::tempdir_in;
use tracing::{debug, info};

use crate::{
    backend::{Backend, BackendKind},
    utils::hacpack_cleanup_install,
    vfs::{
        nca::{self, Nca},
        nsp::Nsp,
        PROGRAMID_LEN,
    },
};

/// Repack romfs/exefs back to NSP.
pub fn repack_fs_data<N, E, R, O>(
    control_path: N,
    mut program_id: String,
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
    let curr_dir = std::env::current_dir()?;
    let _hacpack_cleanup_bind = hacpack_cleanup_install!(curr_dir);

    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "windows", target_os = "linux")
    ))]
    let readers = vec![
        Backend::try_new(BackendKind::Hactoolnet)?,
        Backend::try_new(BackendKind::Hac2l)?,
    ];
    #[cfg(feature = "android-proot")]
    let readers = vec![Backend::new(BackendKind::Hac2l)?];
    let packer = Backend::try_new(BackendKind::Hacpack)?;

    // Validating NCA as Control Type
    let control_nca = readers
        .iter()
        .map(|reader| Nca::try_new(reader, control_path.as_ref()).ok())
        .find(|nca| matches!(nca, Some(nca) if nca.content_type == nca::ContentType::Control))
        .flatten()
        .ok_or_else(|| {
            eyre!(
                "'{}' is not a Control Type NCA",
                control_path.as_ref().display()
            )
        })?;

    program_id.truncate(PROGRAMID_LEN as _);
    debug!(?program_id, "Selected ProgramID for packing");

    let temp_dir = tempdir_in(Config::load()?.temp_dir.as_path())?;

    // !Packing fs files to NCA
    let patched_nca_path = Nca::pack(
        &packer,
        &program_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        romfs_dir.as_ref(),
        exefs_dir.as_ref(),
        temp_dir.path(),
    )?;
    let patched_nca = readers
        .iter()
        // Could inspect and log the error if need be
        .map(|reader| Nca::try_new(reader, &patched_nca_path).ok())
        .find(|nca| nca.is_some())
        .flatten()
        .ok_or_else(|| eyre!("Failed to find Patched NCA"))?;

    // !Generating Meta NCA
    Nca::create_meta(
        &packer,
        &program_id,
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
        &program_id,
        DEFAULT_PRODKEYS_PATH.as_path(),
        temp_dir.path(),
        outdir.as_ref(),
    )?;

    let dest = outdir
        .as_ref()
        .join(format!("{}[yanu-repacked].nsp", program_id));
    info!(from = ?packed_nsp.path,to = ?dest,"Moving");
    move_file(&packed_nsp.path, &dest)?;
    packed_nsp.path = dest;

    Ok(packed_nsp)
}
