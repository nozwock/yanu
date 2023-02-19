use anyhow::{Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::{FileDialog, MessageDialog};
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    utils::browse_nsp_file,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = YanuCli::parse();

    match cli.command {
        Some(CliArgs::Commands::Cli(cli)) => {
            // Cli mode
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

                MessageDialog::new()
                    .set_type(native_dialog::MessageType::Info)
                    .set_title("yanu")
                    .set_text("Please select the UPDATE package file to apply!")
                    .show_alert()?;
                let update = browse_nsp_file().context("no file was selected")?;

                dbg!(&base);
                dbg!(&update);
            }
        }
    }

    Ok(())
}
