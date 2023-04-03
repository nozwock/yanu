use std::path::PathBuf;

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
