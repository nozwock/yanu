use eyre::Result;
use std::path::PathBuf;

/// Don't make this public.\
/// **Note:** `[..]` is there to help you, don't question it.
fn pick_file<'a, T, I>(title: Option<&str>, filters: I) -> Result<PathBuf>
where
    T: AsRef<[&'a str]> + 'a,
    I: IntoIterator<Item = (&'a str, T)>,
{
    use common::utils::get_size_as_string;
    use eyre::eyre;
    use tracing::info;

    let mut filedialog = rfd::FileDialog::new();
    if let Some(title) = title {
        filedialog = filedialog.set_title(title);
    }
    for (name, extensions) in filters.into_iter() {
        filedialog = filedialog.add_filter(name, extensions.as_ref());
    }
    let path = filedialog.pick_file();
    if let Some(path) = &path {
        info!(?path, size = %get_size_as_string(path).unwrap_or_default(), "Selected file");
    }
    path.ok_or_else(|| eyre!("No file was selected"))
}

pub fn pick_nsp_file(title: Option<&str>) -> Result<PathBuf> {
    pick_file(title, [("NSP", &["nsp"])])
}

pub fn pick_nca_file(title: Option<&str>) -> Result<PathBuf> {
    pick_file(title, [("NCA", &["nca"])])
}
