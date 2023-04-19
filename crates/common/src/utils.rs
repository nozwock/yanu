use bytesize::ByteSize;
use eyre::Result;
use fs_err as fs;
use std::path::Path;
use tracing::{debug, warn};

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

pub fn get_size_as_string<P: AsRef<Path>>(path: P) -> Option<String> {
    Some(ByteSize::b(path.as_ref().metadata().ok()?.len()).to_string_as(false))
}

pub fn get_size<P: AsRef<Path>>(path: P) -> Result<ByteSize> {
    Ok(ByteSize::b(path.as_ref().metadata()?.len()))
}

/// Returns free disk space.\
/// `Disk` is retrieved from a given `path`.
/// For example, The `Disk` mounted on `/` will be used for a given path `/home`.
pub fn get_disk_free<P: AsRef<Path>>(path: P) -> Result<ByteSize> {
    use sysinfo::{DiskExt, RefreshKind, System, SystemExt};

    let system = System::new_with_specifics(RefreshKind::new().with_disks().with_disks_list());

    // canonicalizing both paths is important, since we want both paths to be of same type
    // and absent of any intermediate components.
    let abs_path = path.as_ref().canonicalize()?;
    let mut parent = Some(abs_path.as_path());
    loop {
        if let Some(inner_parent) = parent {
            for disk in system.disks() {
                if inner_parent == disk.mount_point().canonicalize()? {
                    debug!(?disk);
                    return Ok(ByteSize(disk.available_space()));
                }
            }
            parent = parent.and_then(|path| path.parent());
        } else {
            break;
        }
    }

    unreachable!()
}

pub fn get_paths_size<P>(paths: &[P]) -> Result<ByteSize>
where
    P: AsRef<Path>,
{
    let mut size = 0;
    for path in paths {
        size += path.as_ref().metadata()?.len();
    }
    Ok(ByteSize(size))
}
