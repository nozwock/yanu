use anyhow::{Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::{MessageDialog, MessageType};
use std::fs;
use tracing::info;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use yanu::utils::{bail_with_error_dialog, browse_nsp_file};
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    config::Config,
    defines::{app_config_dir, keys_path},
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

    let config: Config = confy::load_path(app_config_dir())?;

    let cli = YanuCli::parse();
    match cli.command {
        Some(CliArgs::Commands::Cli(cli)) => {
            // Cli mode
            println!(
                "Patched file saved as:\n{:?}",
                patch_nsp_with_update(&mut Nsp::from(cli.base)?, &mut Nsp::from(cli.update)?)?
                    .path
                    .display()
            );
            info!("Done");
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

                    //? maybe validate if it's indeed prod.keys
                    let keyset_path = keys_path()?;
                    fs::create_dir_all(keyset_path.parent().context("where ma parents?")?)?;
                    fs::copy(path, keyset_path)?;
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
                        match patch_nsp_with_update(
                            &mut Nsp::from(&base_path)?,
                            &mut Nsp::from(&update_path)?,
                        ) {
                            Ok(patched) => {
                                info!("Done");
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
                use anyhow::bail;
                use std::{ffi::OsStr, path::PathBuf};
                use tracing::error;

                if keys_exists().is_none() {
                    let path = PathBuf::from(inquire::Text::new(
                        "Failed to find keys! Please enter the path to your `prod.keys`:",
                    )
                    .with_help_message("This only needs to be done once!\nPath to a file can be copied through some file managers such as MiXplorer, etc.")
                    .prompt()?);

                    let to = keys_path()?;
                    fs::create_dir_all(to.parent().context("where ma parents?")?)?;
                    info!("Selected keys {:?}", path.display());
                    match path.extension().and_then(OsStr::to_str) {
                        Some("keys") => {}
                        _ => bail!("no keys were selected"),
                    }
                    fs::copy(path, to)?;
                }

                let mut base = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter Base pkg path:").prompt()?,
                ))?;
                let mut update = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter Update pkg path:").prompt()?,
                ))?;

                match inquire::Confirm::new("Are you sure?")
                    .with_default(true)
                    .prompt()?
                {
                    true => match patch_nsp_with_update(&mut base, &mut update) {
                        Ok(patched) => {
                            info!("Done");
                            println!("Patched file saved as:\n{:?}", patched.path.display());
                        }
                        Err(err) => {
                            error!("{}", err.to_string());
                            println!("{}", err.to_string());
                        }
                    },
                    false => println!("yanu exited"),
                }
            }
        }
    }

    Ok(())
}
