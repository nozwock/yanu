mod utils;

#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
use crate::utils::pick_nsp_file;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use libyanu_common::defines::{DEFAULT_PRODKEYS_PATH, EXE_DIR};
use libyanu_common::hac::{patch::patch_nsp, rom::Nsp};
use std::time::Instant;
use std::{env, path::PathBuf};
use tracing::{error, info};

fn main() -> Result<()> {
    // Colorful errors
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;

    // Tracing
    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(non_blocking)
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        arch = std::env::consts::ARCH,
        os = std::env::consts::OS,
        "Launching {}",
        env!("CARGO_PKG_NAME"),
    );

    match run() {
        Ok(_) => {
            info!("Done");
            Ok(())
        }
        Err(err) => {
            error!(?err);
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Error occurred")
                .set_description(&err.to_string())
                .show();
            bail!(err);
        }
    }
}

fn run() -> Result<()> {
    if !DEFAULT_PRODKEYS_PATH.is_file() {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("Keyfile required")
            .set_description("Select `prod.keys` keyfile to continue")
            .show();
        let keyfile_path = rfd::FileDialog::new()
            .add_filter("Keys", &["keys"])
            .pick_file()
            .ok_or_else(|| eyre!("No keyfile was selected"))?;
        info!(?keyfile_path, "Selected keyfile");

        // Dialog allows picking dir, atleast on GTK (prob a bug)
        //* ^^^^ doesn't seems to have this issue with the xdg portal backend
        if !keyfile_path.is_file() {
            bail!("\"{}\" is not a file", keyfile_path.display());
        }

        //? maybe validate if it's indeed prod.keys
        let default_path = DEFAULT_PRODKEYS_PATH.as_path();
        fs::create_dir_all(
            default_path
                .parent()
                .ok_or_else(|| eyre!("Failed to find parent"))?,
        )?;
        fs::copy(keyfile_path, default_path)?;
        info!("Copied keys successfully to the C2 ^-^");
    }

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("Base package required")
        .set_description("Select the BASE package file to update")
        .show();
    let base_path = pick_nsp_file().ok_or_else(|| eyre!("No file was selected"))?;
    if !base_path.is_file() {
        bail!("\"{}\" is not a file", base_path.display());
    }

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("Update package required")
        .set_description("Select the UPDATE package file to apply")
        .show();
    let update_path = pick_nsp_file().ok_or_else(|| eyre!("No file was selected"))?;
    if !update_path.is_file() {
        bail!("\"{}\" is not a file", update_path.display());
    }

    let base_name = base_path
        .file_name()
        .expect("File should've a filename")
        .to_string_lossy();
    let update_name = update_path
        .file_name()
        .expect("File should've a filename")
        .to_string_lossy();

    if rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("Is this correct?")
        .set_description(&format!(
            "Selected BASE package: \n\"{}\"\n\
                        Selected UPDATE package: \n\"{}\"",
            base_name, update_name
        ))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
    {
        info!("Started patching!");
        let started = Instant::now();
        let patched = patch_nsp(
            &mut Nsp::new(&base_path)?,
            &mut Nsp::new(&update_path)?,
            default_outdir()?,
        )?;
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("Patching successful")
            .set_description(&format!(
                "Patched file created at:\n\"{}\"\nTook {:?}",
                patched.path.display(),
                started.elapsed()
            ))
            .show();
    }

    Ok(())
}

fn default_outdir() -> Result<PathBuf> {
    let outdir: PathBuf = {
        if cfg!(feature = "android-proot") {
            PathBuf::from("/storage/emulated/0")
        } else {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            EXE_DIR.to_owned()
        }
    };

    if !outdir.is_dir() {
        bail!("Failed to set \"{}\" as outdir", outdir.display());
    }

    Ok(outdir)
}
