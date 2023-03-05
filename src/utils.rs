use eyre::Result;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::FileDialog;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::defines::get_default_keyfile_path;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn browse_nsp_file() -> Option<PathBuf> {
    use tracing::info;

    let path = FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .show_open_single_file()
        .ok()?;
    if let Some(ref path) = path {
        info!(?path, "Selected file");
    }
    path
}

pub fn str_truncate(s: &str, new_len: usize) -> &str {
    match s.char_indices().nth(new_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn keyfile_exists() -> Option<()> {
    if !get_default_keyfile_path().ok()?.is_file() {
        return None;
    }
    Some(())
}

pub fn move_file<P, Q>(from: P, to: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    match fs::rename(&from, &to) {
        Ok(_) => Ok(()),
        Err(_) => {
            fs::copy(&from, &to)?;
            fs::remove_file(&from)?;
            Ok(())
        }
    }
}
