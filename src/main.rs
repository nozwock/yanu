use anyhow::{bail, Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::{MessageDialog, MessageType};
use std::fs;
use tracing::{error, info};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use yanu::utils::{bail_with_error_dialog, browse_nsp_file};
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    defines::{app_config_dir, get_keyset_path},
    hac::{patch::patch_nsp_with_update, rom::Nsp},
    utils::keys_exists,
};

fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(non_blocking)
        .init();

    match app() {
        Ok(_) => {
            info!("Done");
            Ok(())
        }
        Err(err) => {
            error!("{}", err.to_string());
            bail!(err.to_string());
        }
    }
}

fn app() -> Result<()> {
    let cli = YanuCli::parse();

    match cli.command {
        Some(CliArgs::Commands::Cli(cli)) => {
            // Cli mode
            // ! Yet to handle keys
            info!("Started patching!");
            println!(
                "Patched file saved as:\n{:?}",
                patch_nsp_with_update(&mut Nsp::from(cli.base)?, &mut Nsp::from(cli.update)?)?
                    .path
                    .display()
            );
        }
        None => {
            // Interactive mode
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                if keys_exists().is_none() {
                    MessageDialog::new()
                        .set_type(MessageType::Warning)
                        .set_title("Failed to find keys!")
                        .set_text("Please select your `prod.keys` to continue further")
                        .show_alert()?;
                    let path = native_dialog::FileDialog::new()
                        .add_filter("Keys", &["keys"])
                        .show_open_single_file()?
                        .context("no key was selected")?;
                    info!("Selected keys {:?}", path.display());

                    // ! native dialog allows for dir to be picked (prob a bug)
                    // ! handle this soon

                    //? maybe validate if it's indeed prod.keys
                    let keyset_path = get_keyset_path()?;
                    fs::create_dir_all(keyset_path.parent().context("where ma parents?")?)?;
                    fs::copy(path, keyset_path)?;
                    info!("Copied keys successfully to the C2 ^-^");
                }

                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the BASE package file to update!")
                    .show_alert()?;
                let base_path = browse_nsp_file().context("no file was selected")?;

                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the UPDATE package file to apply!")
                    .show_alert()?;
                let update_path = browse_nsp_file().context("no file was selected")?;

                let base_name = base_path
                    .file_name()
                    .expect("A nsp file must've been selected by the file picker")
                    .to_string_lossy();
                let update_name = update_path
                    .file_name()
                    .expect("A nsp file must've been selected by the file picker")
                    .to_string_lossy();

                match MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("Is this correct?")
                    .set_text(&format!(
                        "Selected base pkg: \n\"{}\"\n\
                        Selected update pkg: \n\"{}\"",
                        base_name, update_name
                    ))
                    .show_confirm()?
                {
                    true => {
                        info!("Started patching!");
                        match patch_nsp_with_update(
                            &mut Nsp::from(&base_path)?,
                            &mut Nsp::from(&update_path)?,
                        ) {
                            Ok(patched) => {
                                MessageDialog::new()
                                    .set_type(MessageType::Info)
                                    .set_title("Done patching!")
                                    .set_text(&format!(
                                        "Patched file saved as:\n{:?}",
                                        patched.path.display()
                                    ))
                                    .show_alert()?;
                            }
                            Err(err) => {
                                bail_with_error_dialog(&err.to_string(), None)?;
                            }
                        }
                    }
                    false => println!("yanu exited"),
                }
            }

            #[cfg(target_os = "android")]
            {
                use std::{ffi::OsStr, path::PathBuf};

                if keys_exists().is_none() {
                    let path = PathBuf::from(inquire::Text::new(
                        "Failed to find keys!\nPlease enter the path to your `prod.keys`:",
                    )
                    .with_help_message("This only needs to be done once!\nPath to a file can be copied through some file managers such as MiXplorer, etc.")
                    .prompt()?);
                    info!("Selected keys {:?}", path.display());

                    let keyset_path = get_keyset_path()?;
                    fs::create_dir_all(keyset_path.parent().context("where ma parents?")?)?;
                    match path.extension().and_then(OsStr::to_str) {
                        Some("keys") => {}
                        _ => bail!("no keys were selected"),
                    }
                    fs::copy(path, keyset_path)?;
                    info!("Copied keys successfully to the C2 ^-^");
                }

                let mut base = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter Base package path:").prompt()?,
                ))?;
                let mut update = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter Update package path:").prompt()?,
                ))?;

                match inquire::Confirm::new("Are you sure?")
                    .with_default(true)
                    .prompt()?
                {
                    true => {
                        info!("Started patching!");
                        match patch_nsp_with_update(&mut base, &mut update) {
                            Ok(patched) => {
                                println!("Patched file saved as:\n{:?}", patched.path.display());
                            }
                            Err(err) => {
                                bail!("{}", err.to_string());
                            }
                        }
                    }
                    false => println!("yanu exited"),
                }
            }
        }
    }

    Ok(())
}
