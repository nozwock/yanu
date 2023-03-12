use eyre::Result;
use fs_err as fs;
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn pick_nsp_file() -> Option<PathBuf> {
    let path = rfd::FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .pick_file();
    if let Some(ref path) = path {
        tracing::info!(?path, "Selected file");
    }
    path
}

pub fn str_truncate(s: &str, new_len: usize) -> &str {
    match s.char_indices().nth(new_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
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
