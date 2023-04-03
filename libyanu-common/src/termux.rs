use eyre::{bail, Result};
use std::{path::Path, process::Command};
use tracing::{info, warn};

pub fn clipboard_get() -> Result<String> {
    let output = String::from_utf8(Command::new("termux-clipboard-get").output()?.stdout)?;
    info!("Copied from clipboard \"{}\"", output);
    Ok(output)
}

pub fn storage_get<P: AsRef<Path>>(output: P) -> Result<()> {
    warn!(
        "Using unstable API \"termux-storage-get \"{}\"\"",
        output.as_ref().display()
    );
    if !Command::new("termux-storage-get")
        .arg(output.as_ref())
        .status()?
        .success()
    {
        bail!("Failed to get file from storage");
    }

    // making sure the file is copied ;-;
    if !output.as_ref().is_file() {
        bail!("Failed to get file from storage");
    } else {
        info!("Copied file to \"{}\"", output.as_ref().display());
    }
    Ok(())
}
