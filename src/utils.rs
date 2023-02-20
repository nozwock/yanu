use native_dialog::FileDialog;
use std::path::PathBuf;

pub fn browse_nsp_file() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("NSP Files", &["nsp"])
        .show_open_single_file()
        .ok()?
}

pub fn str_truncate(s: &str, new_len: usize) -> &str {
    match s.char_indices().nth(new_len) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}
