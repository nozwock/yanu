use anyhow::{bail, Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::MessageDialog;
use tracing::debug;
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    utils::browse_nsp_file,
};

fn main() -> Result<()> {
    // tracing_subscriber::fmt::init();
    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    let cli = YanuCli::parse();

    match cli.command {
        Some(CliArgs::Commands::Cli(_cli)) => {
            // Cli mode
            unimplemented!();
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
                let base = browse_nsp_file().context("no file was selected")?;
                if !base.is_file() {
                    bail!("no file was selected");
                }
                debug!("Selected base package: \"{}\"", base.to_string_lossy());

                MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the UPDATE package file to apply!")
                    .show_alert()?;
                let update = browse_nsp_file().context("no file was selected")?;
                if !update.is_file() {
                    bail!("no file was selected");
                }
                debug!("Selected update package: \"{}\"", base.to_string_lossy());

                let base_name = base
                    .file_name()
                    .expect("A nsp file must've been selected by the file picker")
                    .to_string_lossy();
                let update_name = update
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
                    true => unimplemented!(),
                    false => println!("Program exited."),
                }
            }

            #[cfg(target_os = "android")]
            {}
        }
    }

    Ok(())
}
