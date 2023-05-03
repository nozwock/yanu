use common::{defines::DEFAULT_PRODKEYS_PATH, utils::get_fmt_size};
use egui_modal::Modal;
use eyre::{bail, Result};
use std::path::PathBuf;
use tracing::info;

pub fn default_pack_outdir() -> Result<PathBuf> {
    let outdir: PathBuf = {
        if cfg!(feature = "android-proot") {
            PathBuf::from("/storage/emulated/0")
        } else {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            std::env::current_dir()?
        }
    };

    if !outdir.is_dir() {
        bail!("Failed to set '{}' as outdir", outdir.display());
    }

    Ok(outdir)
}

pub fn check_keyfile_exists() -> Result<()> {
    if DEFAULT_PRODKEYS_PATH.is_file() {
        Ok(())
    } else {
        bail!("'prod.keys' Keyfile not found, it's required")
    }
}

/// Consumes the `Err` and shows an Error dialog.
pub fn consume_err<T>(dialog_modal: &Modal, inner: Result<T>, on_ok: impl FnOnce(T)) {
    match inner {
        Ok(t) => on_ok(t),
        Err(err) => {
            dialog_modal.open_dialog(None::<&str>, Some(err), Some(egui_modal::Icon::Error))
        }
    };
}

pub fn consume_err_or<T>(
    body: &str,
    dialog_modal: &Modal,
    inner: Option<T>,
    on_some: impl FnOnce(T),
) {
    match inner {
        Some(t) => on_some(t),
        None => dialog_modal.open_dialog(None::<&str>, Some(body), Some(egui_modal::Icon::Error)),
    };
}

pub fn pick_nsp_file(dialog_modal: &Modal, title: Option<&str>, on_success: impl FnOnce(PathBuf)) {
    let mut dialog = rfd::FileDialog::new().add_filter("NSP", &["nsp"]);
    if let Some(title) = title {
        dialog = dialog.set_title(title);
    }
    consume_err_or(
        "No file was picked",
        dialog_modal,
        dialog.pick_file(),
        |path| {
            info!(?path, size = %get_fmt_size(&path).unwrap_or_default(), "Picked file");
            on_success(path)
        },
    );
}
