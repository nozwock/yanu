use anyhow::{bail, Context, Result};
use clap::Parser;
#[cfg(any(target_os = "linux", target_os = "windows"))]
use native_dialog::{MessageDialog, MessageType};
use std::{env, ffi::OsStr, fs, path::PathBuf};
use tracing::{error, info};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use yanu::utils::{bail_with_error_dialog, browse_nsp_file};
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    config::Config,
    defines::{app_config_path, get_default_keyfile_path},
    hac::{patch::patch_nsp_with_update, rom::Nsp},
    utils::keyfile_exists,
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
    let mut config: Config = confy::load_path(app_config_path())?;
    let cli = YanuCli::parse();

    match cli.command {
        Some(CliArgs::Commands::Cli(cli)) => {
            // Cli mode
            match cli.keyfile {
                Some(keyfile) => {
                    let keyfile_path = PathBuf::from(keyfile);
                    if keyfile_path
                        .extension()
                        .and_then(OsStr::to_str)
                        .context("File should've an extension")?
                        != "keys"
                    {
                        bail!("Invalid keyfile");
                    }

                    info!("Selected keyfile {:?}", keyfile_path.display());
                    let default_path = get_default_keyfile_path()?;
                    fs::create_dir_all(default_path.parent().context("Failed to find parent")?)?;
                    fs::copy(keyfile_path, default_path)?;
                    info!("Copied keys successfully to the C2 ^-^");
                }
                None => {
                    if keyfile_exists().is_none() {
                        bail!("Failed to find keyfile");
                    }
                }
            }

            info!("Started patching!");
            println!(
                "\nPatched file saved as:\n{:?}",
                patch_nsp_with_update(
                    &mut Nsp::from(cli.base)?,
                    &mut Nsp::from(cli.update)?,
                    get_default_outdir()?
                )?
                .path
                .display()
            );
        }
        Some(CliArgs::Commands::Config(new_config)) => {
            if let Some(roms_dir) = new_config.roms_dir {
                if !roms_dir.is_dir() {
                    bail!("{:?} is not a valid directory", roms_dir);
                }
                config.roms_dir = Some(roms_dir);
            }

            info!("Updating config at {:?}", app_config_path());
            confy::store_path(app_config_path(), config)?;
        }
        None => {
            // Interactive mode
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            {
                if keyfile_exists().is_none() {
                    MessageDialog::new()
                        .set_type(MessageType::Warning)
                        .set_title("Failed to find keyfile!")
                        .set_text("Please select `prod.keys` keyfile to continue")
                        .show_alert()?;
                    let keyfile_path = native_dialog::FileDialog::new()
                        .add_filter("Keys", &["keys"])
                        .show_open_single_file()?
                        .context("No keyfile was selected")?;
                    info!("Selected keyfile {:?}", keyfile_path.display());

                    // native dialog allows for dir to be picked (prob a bug)
                    if !keyfile_path.is_file() {
                        bail_with_error_dialog(
                            &format!("{:?} is not a file", keyfile_path.display()),
                            None,
                        )?;
                    }

                    //? maybe validate if it's indeed prod.keys
                    let default_path = get_default_keyfile_path()?;
                    fs::create_dir_all(default_path.parent().context("Failed to find parent")?)?;
                    fs::copy(keyfile_path, default_path)?;
                    info!("Copied keys successfully to the C2 ^-^");
                }

                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("yanu • BASE")
                    .set_text("Please select the BASE package file to update!")
                    .show_alert()?;
                let base_path = browse_nsp_file().context("No file was selected")?;
                if !base_path.is_file() {
                    bail_with_error_dialog(
                        &format!("{:?} is not a file", base_path.display()),
                        None,
                    )?;
                }

                MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("yanu • UPDATE")
                    .set_text("Please select the UPDATE package file to apply!")
                    .show_alert()?;
                let update_path = browse_nsp_file().context("No file was selected")?;
                if !update_path.is_file() {
                    bail_with_error_dialog(
                        &format!("{:?} is not a file", update_path.display()),
                        None,
                    )?;
                }

                let base_name = base_path
                    .file_name()
                    .expect("Path should've a filename")
                    .to_string_lossy();
                let update_name = update_path
                    .file_name()
                    .expect("Path should've a filename")
                    .to_string_lossy();

                match MessageDialog::new()
                    .set_type(MessageType::Info)
                    .set_title("Is this correct?")
                    .set_text(&format!(
                        "Selected BASE package: \n\"{}\"\n\
                        Selected UPDATE package: \n\"{}\"",
                        base_name, update_name
                    ))
                    .show_confirm()?
                {
                    true => {
                        info!("Started patching!");
                        match patch_nsp_with_update(
                            &mut Nsp::from(&base_path)?,
                            &mut Nsp::from(&update_path)?,
                            get_default_outdir()?,
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
                use walkdir::WalkDir;

                if config.roms_dir.is_none() {
                    let roms_dir = PathBuf::from(
                        inquire::Text::new("Enter the path to a directory:")
                            .with_placeholder("for eg- /storage/emulated/0/SwitchRoms")
                            .with_help_message("This directory will be used to look for roms! They will be showed in the Menu GUI.\n\
                            Path to an item can be copied through some file managers such as MiXplorer, etc.")
                            .prompt()?,
                    );
                    info!("Set {:?} as roms_dir", roms_dir);

                    if !roms_dir.is_dir() {
                        bail!("{:?} is not a valid directory", roms_dir);
                    }
                    config.roms_dir = Some(roms_dir);
                    info!("Updating config at {:?}", app_config_path());
                    confy::store_path(app_config_path(), config.clone())?;
                }

                let roms_dir = config.roms_dir.expect("roms_dir should've been set");

                if keyfile_exists().is_none() {
                    // Looking for `prod.keys` in roms_dir
                    let mut keyfile_path: Option<PathBuf> = None;
                    for entry in WalkDir::new(&roms_dir)
                        .min_depth(1)
                        .into_iter()
                        .filter_map(|e| e.ok())
                    {
                        if entry.file_name() == "prod.keys" {
                            keyfile_path = Some(entry.path().into());
                            break;
                        }
                    }

                    if keyfile_path.is_none() {
                        keyfile_path = Some(PathBuf::from(inquire::Text::new(
                            "Failed to find keyfile!\nPlease enter the path to `prod.keys` keyfile:",
                        )
                        .with_help_message("This only needs to be done once!\nPath to an item can be copied through some file managers such as MiXplorer, etc.")
                        .prompt()?));
                    }

                    let keyfile_path = keyfile_path.expect("Keyfile path should've been set");
                    info!("Selected keys {:?}", keyfile_path.display());

                    let default_path = get_default_keyfile_path()?;
                    fs::create_dir_all(default_path.parent().context("Failed to find parent")?)?;
                    match keyfile_path.extension().and_then(OsStr::to_str) {
                        Some("keys") => {}
                        _ => bail!("No keyfile was selected"),
                    }
                    fs::copy(keyfile_path, default_path)?;
                    info!("Copied keys successfully to the C2 ^-^");
                }

                let roms_path = WalkDir::new(&roms_dir)
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|entry| {
                        if entry.file_type().is_file()
                            && entry
                                .path()
                                .extension()
                                .and_then(|s| Some(s.to_ascii_lowercase()))
                                == Some("nsp".into())
                        {
                            return true;
                        }
                        false
                    })
                    .collect::<Vec<_>>();

                // ! ummm...I don't feel so gud here
                let mut base: Option<Nsp> = None;
                let mut options = roms_path
                    .iter()
                    .map(|entry| entry.file_name().to_string_lossy())
                    .collect::<Vec<_>>();
                let choice =
                    inquire::Select::new("Select BASE package:", options.clone()).prompt()?;
                for entry in &roms_path {
                    if entry.file_name().to_string_lossy() == choice {
                        base = Some(Nsp::from(entry.path())?);
                    }
                }
                let mut base = base.expect("Selected option should be in roms_path");

                let mut update: Option<Nsp> = None;
                options = options
                    .into_iter()
                    .filter(|s| {
                        if s == &choice {
                            return false;
                        }
                        true
                    })
                    .collect();
                let choice = inquire::Select::new("Select UPDATE package:", options).prompt()?;
                for entry in &roms_path {
                    if entry.file_name().to_string_lossy() == choice {
                        update = Some(Nsp::from(entry.path())?);
                    }
                }
                let mut update = update.expect("Selected option should be in roms_path");

                match inquire::Confirm::new("Are you sure?")
                    .with_default(true)
                    .prompt()?
                {
                    true => {
                        info!("Started patching!");
                        match patch_nsp_with_update(&mut base, &mut update, get_default_outdir()?) {
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

fn get_default_outdir() -> Result<PathBuf> {
    let outdir: PathBuf;
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        outdir = env::current_exe()?
            .parent()
            .expect("Failed to find parent")
            .to_owned();
    }
    #[cfg(target_os = "android")]
    {
        outdir = dirs::home_dir()
            .context("Failed to find home dir")?
            .join("storage")
            .join("shared");
    }

    if !outdir.is_dir() {
        bail!("Failed to set {:?} as outdir", outdir.display());
    }

    Ok(outdir)
}
