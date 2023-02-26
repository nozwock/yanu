use anyhow::{bail, Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::MessageDialog;
use std::path::PathBuf;
use tracing::debug;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use yanu::utils::browse_nsp_file;
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    hac::{patch::patch_nsp_with_update, rom::Nsp},
};

fn main() -> Result<()> {
    // let current_exe_path = env::current_exe().expect("should be able to get current exe path");

    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(non_blocking)
        .init();

    let cli = YanuCli::parse();

    match cli.command {
        Some(CliArgs::Commands::Cli(_cli)) => {
            // Cli mode
            todo!();
        }
        None => {
            // Interactive mode
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the BASE package file to update!")
                    .show_alert()?;
                let base_path = browse_nsp_file().context("no file was selected")?;
                if !base_path.is_file() {
                    bail!("no file was selected");
                }
                debug!("Selected base package: \"{}\"", base_path.to_string_lossy());

                MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the UPDATE package file to apply!")
                    .show_alert()?;
                let update_path = browse_nsp_file().context("no file was selected")?;
                if !update_path.is_file() {
                    bail!("no file was selected");
                }
                debug!(
                    "Selected update package: \"{}\"",
                    base_path.to_string_lossy()
                );

                let base_name = base_path
                    .file_name()
                    .expect("A nsp file must've been selected by the file picker")
                    .to_string_lossy();
                let update_name = update_path
                    .file_name()
                    .expect("A nsp file must've been selected by the file picker")
                    .to_string_lossy();

                match MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("Is this correct?")
                    .set_text(&format!(
                        "Selected base pkg: \n\"{}\"\n\n\
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
                                MessageDialog::new()
                                    .set_type(native_dialog::MessageType::Info)
                                    .set_title("Done patching!")
                                    .set_text(&format!(
                                        "Patched file saved as:\n{:?}",
                                        patched.path.display()
                                    ))
                                    .show_alert()?;
                            }
                            Err(err) => {
                                MessageDialog::new()
                                    .set_type(native_dialog::MessageType::Error)
                                    .set_title("Error occured!")
                                    .set_text(&err.to_string())
                                    .show_alert()?;
                            }
                        }
                    }
                    false => println!("Program exited."),
                }
            }

            #[cfg(target_os = "android")]
            {
                let mut base = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter base nsp path:").prompt()?,
                ))?;
                let mut update = Nsp::from(PathBuf::from(
                    inquire::Text::new("Enter update nsp path:").prompt()?,
                ))?;

                match inquire::Confirm::new("Are you sure?")
                    .with_default(true)
                    .prompt()?
                {
                    true => match patch_nsp_with_update(&mut base, &mut update) {
                        Ok(_) => {
                            println!("Done patching");
                        }
                        Err(_) => println!("fk"),
                    },
                    false => todo!(),
                }
            }
        }
    }

    Ok(())
}
