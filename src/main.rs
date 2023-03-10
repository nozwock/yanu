use clap::Parser;
use eyre::{bail, eyre, Result};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::{env, ffi::OsStr, fs, path::PathBuf};
use tracing::{debug, error, info, warn};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use yanu::utils::pick_nsp_file;
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    config::Config,
    defines::{app_config_dir, app_config_path, get_default_keyfile_path},
    hac::{patch::patch_nsp_with_update, rom::Nsp},
    utils::keyfile_exists,
};

fn process_init() {
    use std::sync::Once;

    static INIT: Once = Once::new();

    #[allow(unused_unsafe)]
    INIT.call_once(|| unsafe {
        #[cfg(target_os = "windows")]
        winapi::um::winuser::SetProcessDPIAware();
    });
}

fn main() -> Result<()> {
    ctrlc::set_handler(move || {
        error!("Process terminated by the user!");
    })?;

    let file_appender = tracing_appender::rolling::hourly("", "yanu.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(non_blocking)
        .init();

    process_init();

    info!(
        "Launching {} on {}!",
        env!("CARGO_PKG_NAME"),
        env::consts::OS
    );
    info!(version = env!("CARGO_PKG_VERSION"));

    let cli = YanuCli::parse();
    let mut cli_mode = false;
    if let Some(CliArgs::Commands::Cli(_)) = cli.command {
        cli_mode = true;
    }

    match run(cli) {
        Ok(_) => {
            info!("Done");
            Ok(())
        }
        Err(err) => {
            error!(?err);
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            if !cli_mode {
                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Error occurred")
                    .set_description(&err.to_string())
                    .show();
            }
            bail!(err);
        }
    }
}

fn run(cli: YanuCli) -> Result<()> {
    let mut config: Config = confy::load_path(app_config_path())?;

    match cli.command {
        Some(CliArgs::Commands::Cli(cli)) => {
            // Cli mode
            match cli.keyfile {
                Some(keyfile) => {
                    let keyfile_path = PathBuf::from(keyfile);
                    if keyfile_path
                        .extension()
                        .and_then(OsStr::to_str)
                        .ok_or_else(|| eyre!("File should've an extension"))?
                        != "keys"
                    {
                        bail!("Invalid keyfile");
                    }

                    info!(?keyfile_path, "Selected keyfile");
                    let default_path = get_default_keyfile_path()?;
                    fs::create_dir_all(
                        default_path
                            .parent()
                            .ok_or_else(|| eyre!("Failed to find parent"))?,
                    )?;
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
            patch_nsp_with_update(
                &mut Nsp::from(cli.base)?,
                &mut Nsp::from(cli.update)?,
                get_default_outdir()?,
            )?;
        }
        #[cfg(target_os = "android")]
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
                        bail!("{:?} is not a file", keyfile_path);
                    }

                    //? maybe validate if it's indeed prod.keys
                    let default_path = get_default_keyfile_path()?;
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
                    bail!("{:?} is not a file", base_path);
                }

                rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_title("Update package required")
                    .set_description("Select the UPDATE package file to apply")
                    .show();
                let update_path = pick_nsp_file().ok_or_else(|| eyre!("No file was selected"))?;
                if !update_path.is_file() {
                    bail!("{:?} is not a file", update_path);
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
                    == true
                {
                    info!("Started patching!");
                    match patch_nsp_with_update(
                        &mut Nsp::from(&base_path)?,
                        &mut Nsp::from(&update_path)?,
                        get_default_outdir()?,
                    ) {
                        Ok(patched) => {
                            rfd::MessageDialog::new()
                                .set_level(rfd::MessageLevel::Info)
                                .set_title("Patching successful")
                                .set_description(&format!(
                                    "Patched file created at:\n{:?}",
                                    patched.path
                                ))
                                .show();
                        }
                        Err(err) => {
                            bail!(err);
                        }
                    }
                }
            }

            #[cfg(target_os = "android")]
            {
                use std::{ffi::OsStr, path::PathBuf};
                use walkdir::WalkDir;

                if config.roms_dir.is_none() {
                    let roms_dir = PathBuf::from(
                        inquire::Text::new("Enter the path to a directory:")
                            .with_default("/storage/emulated/0")
                            .with_placeholder("for eg- /storage/emulated/0/SwitchRoms")
                            .with_help_message(
                                "Help:\n1. This directory will be used to look for ROMs (base/update)\n\
                                2. `prod.keys` from the given directory will be used, if any",
                            )
                            .prompt()?,
                    );
                    info!(?roms_dir);

                    if !roms_dir.is_dir() {
                        bail!("{:?} is not a valid directory", roms_dir);
                    }
                    config.roms_dir = Some(roms_dir);
                    info!("Updating config at {:?}", app_config_path());
                    confy::store_path(app_config_path(), config.clone())?;
                }

                let roms_dir = config.roms_dir.expect("roms_dir should've been Some()");

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
                        eprintln!("Failed to find keyfile!");
                        keyfile_path = Some(PathBuf::from(
                            inquire::Text::new("Enter the path to `prod.keys` keyfile:")
                                .prompt()?,
                        ));
                    }

                    let keyfile_path = keyfile_path.expect("Keyfile path should've been Some()");
                    info!(?keyfile_path, "Selected keyfile");

                    let default_path = get_default_keyfile_path()?;
                    fs::create_dir_all(
                        default_path
                            .parent()
                            .ok_or_else(|| eyre!("Failed to find parent"))?,
                    )?;
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
                let mut base = base.expect(&format!(
                    "Selected package {:?} should be in {:?}",
                    choice, roms_path
                ));

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
                let mut update = update.expect(&format!(
                    "Selected package {:?} should be in {:?}",
                    choice, roms_path
                ));

                if inquire::Confirm::new("Are you sure?")
                    .with_default(false)
                    .prompt()?
                    == true
                {
                    info!("Started patching!");
                    if let Err(err) =
                        patch_nsp_with_update(&mut base, &mut update, get_default_outdir()?)
                    {
                        bail!(err);
                    }
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
        let exe_path = env::current_exe()?;
        outdir = exe_path
            .parent()
            .ok_or_else(|| eyre!("Failed to get parent of {:?}", exe_path))?
            .to_owned();
    }
    #[cfg(target_os = "android")]
    {
        outdir = dirs::home_dir()
            .ok_or_else(|| eyre!("Failed to find home dir"))?
            .join("storage")
            .join("shared");
    }

    if !outdir.is_dir() {
        bail!("Failed to set {:?} as outdir", outdir);
    }

    Ok(outdir)
}
