use eyre::Result;
use fs_err as fs;
use once_cell::sync::Lazy;
use std::{
    env,
    path::{Path, PathBuf},
};

pub static EXE_DIR: Lazy<PathBuf> =
    Lazy::new(|| env::current_exe().unwrap().parent().unwrap().into());

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

#[cfg(target_family = "unix")]
pub fn set_executable_bit<P: AsRef<Path>>(path: P, on: bool) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::File::open(path.as_ref())?
        .metadata()?
        .permissions()
        .mode();
    fs::set_permissions(
        path.as_ref(),
        std::fs::Permissions::from_mode(if on { mode | 0o111 } else { mode & 0o666 }),
    )?;
    tracing::info!(path = ?path.as_ref(), "Given executable permission");
    Ok(())
}
