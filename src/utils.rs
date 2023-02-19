use native_dialog::FileDialog;
use std::path::PathBuf;

pub fn browse_nsp_file() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .show_open_single_file()
        .ok()?
}
