use std::path::PathBuf;

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn pick_nsp_file() -> eyre::Result<PathBuf> {
    use common::utils::get_size;
    use tracing::info;

    let path = rfd::FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .pick_file();
    if let Some(path) = &path {
        info!(?path, size = %get_size(path).unwrap_or("None".into()), "Selected file");
    }
    path.ok_or_else(|| eyre::eyre!("No file was selected"))
}
