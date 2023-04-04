use eyre::Result;
use fs_err as fs;
use std::path::Path;

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
