use super::nsp::Nsp;
use crate::backend::{Backend, BackendKind};
use common::{
    defines::DEFAULT_PRODKEYS_PATH,
    utils::{ext_matches, get_fmt_size, move_file},
};
use eyre::{bail, Result};
use fs_err as fs;
use std::{path::Path, process::Command};
use tracing::{info, warn};
use walkdir::WalkDir;

pub fn xci_to_nsps<P, Q, R>(xci: P, outdir: Q, tempdir_in: R) -> Result<Vec<Nsp>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    is_xci(xci.as_ref())?;

    info!(
        xci = %xci.as_ref().display(),
        size = %get_fmt_size(xci.as_ref()).unwrap_or_default(),
        "Converting to NSP"
    );

    let backend = Backend::try_new(BackendKind::FourNXCI)?;
    let temp_dir = tempfile::tempdir_in(tempdir_in.as_ref())?;
    let temp_outdir = tempfile::tempdir_in(tempdir_in.as_ref())?;
    fs::create_dir_all(&temp_outdir)?;
    if !Command::new(backend.path())
        .args([
            "--keyset".as_ref(),
            DEFAULT_PRODKEYS_PATH.as_path(),
            "--tempdir".as_ref(),
            temp_dir.path(),
            "--outdir".as_ref(),
            temp_outdir.path(),
            "--rename".as_ref(),
            xci.as_ref(),
        ])
        .status()?
        .success()
    {
        warn!("Encountered an error while trying to convert XCI to NSP");
    }

    let mut nsps = vec![];
    for entry in WalkDir::new(temp_outdir.path())
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(mut nsp) = Nsp::try_new(entry.path()) {
            // Moving NSP file from temp outdir to outdir
            let dest = outdir.as_ref().join(entry.file_name());
            move_file(&nsp.path, &dest)?;
            nsp.path = dest;
            nsps.push(nsp);
        }
    }

    if nsps.is_empty() {
        bail!("Failed to convert XCI to NSP");
    }

    info!(?nsps, "Converted to NSPs");

    Ok(nsps)
}

fn is_xci<P: AsRef<Path>>(path: P) -> Result<()> {
    if !path.as_ref().is_file() || !ext_matches(path.as_ref(), "xci") {
        bail!("'{}' is not a XCI file", path.as_ref().display());
    }
    Ok(())
}
