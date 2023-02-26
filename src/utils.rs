#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::FileDialog;
use std::path::PathBuf;

use crate::defines::keys_path;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn browse_nsp_file() -> Option<PathBuf> {
    use tracing::info;

    let path = FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .show_open_single_file()
        .ok()?;
    if let Some(ref path) = path {
        info!("Selected file {:?}", path.display());
    }
    path
}

pub fn str_truncate(s: &str, new_len: usize) -> &str {
    match s.char_indices().nth(new_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

pub fn keys_exists() -> Option<()> {
    if !keys_path().ok()?.is_file() {
        return None;
    }
    Some(())
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub fn bail_with_error_dialog(msg: &str, title: Option<&str>) -> anyhow::Result<()> {
    tracing::error!("{}", msg);
    native_dialog::MessageDialog::new()
        .set_type(native_dialog::MessageType::Error)
        .set_title(title.unwrap_or("Error occurred!"))
        .set_text(msg)
        .show_alert()?;
    anyhow::bail!("{}", msg);
}
