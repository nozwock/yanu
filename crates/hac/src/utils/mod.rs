pub mod repack;
pub mod unpack;
pub mod update;

use crate::ticket::TitleKey;
use common::defines::DEFAULT_TITLEKEYS_PATH;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use std::io::ErrorKind;
use tracing::info;

pub fn clear_titlekeys() -> Result<()> {
    match fs::remove_file(DEFAULT_TITLEKEYS_PATH.as_path()) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => {
            bail!(err)
        }
    }
}

/// Store TitleKeys to `DEFAULT_TITLEKEYS_PATH`.
pub fn store_titlekeys<'a, I>(keys: I) -> Result<()>
where
    I: Iterator<Item = &'a TitleKey>,
{
    info!(keyfile = ?DEFAULT_TITLEKEYS_PATH.as_path(), "Storing TitleKeys");
    fs::create_dir_all(DEFAULT_TITLEKEYS_PATH.parent().unwrap())?;
    fs::write(
        DEFAULT_TITLEKEYS_PATH.as_path(),
        keys.map(|key| key.to_string())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .map_err(|err| eyre!(err))
}
