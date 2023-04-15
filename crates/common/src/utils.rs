use bytesize::ByteSize;
use eyre::Result;
use fs_err as fs;
use std::path::Path;
use tracing::warn;

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
    if let Err(err) = fs::rename(from.as_ref(), to.as_ref()) {
        warn!(from = %from.as_ref().display(), to = %to.as_ref().display(), %err, "Renaming failed, falling back to copy");
        fs::copy(&from, &to)?;
        fs::remove_file(&from)?;
    };
    Ok(())
}

#[cfg(unix)]
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

pub fn ext_matches<P: AsRef<Path>>(path: P, ext: &str) -> bool {
    path.as_ref()
        .extension()
        .map(|_ext| _ext.to_ascii_lowercase() == ext)
        .unwrap_or(false)
}

pub fn get_size<P: AsRef<Path>>(path: P) -> Result<String> {
    Ok(ByteSize::b(path.as_ref().metadata()?.len()).to_string_as(false))
}
