use eyre::Result;
use std::path::PathBuf;

/// Don't make this public.\
/// **Note:** `[..]` is there to help you, don't question it.
fn pick_file<'a, T, I>(filters: I) -> Result<PathBuf>
where
    T: AsRef<[&'a str]> + 'a,
    I: IntoIterator<Item = (&'a str, T)>,
{
    use common::utils::get_size_as_string;
    use eyre::eyre;
    use tracing::info;

    let mut filedialog = rfd::FileDialog::new();
    for (name, extensions) in filters.into_iter() {
        filedialog = filedialog.add_filter(name, extensions.as_ref());
    }
    let path = filedialog.pick_file();
    if let Some(path) = &path {
        info!(?path, size = %get_size_as_string(path).unwrap_or("None".into()), "Selected file");
    }
    path.ok_or_else(|| eyre!("No file was selected"))
}

pub fn pick_nsp_file() -> Result<PathBuf> {
    pick_file([("NSP", &["nsp"])])
}
