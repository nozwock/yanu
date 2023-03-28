use clap::Parser;
use console::style;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use std::{env, ffi::OsStr, path::PathBuf};
#[cfg(unix)]
use tilde_expand::tilde_expand;
use tracing::{error, info};
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
use yanu::utils::pick_nsp_file;
use yanu::{
    cli::{args as CliArgs, args::YanuCli},
    config::Config,
    defines::{APP_CONFIG_PATH, DEFAULT_PRODKEYS_PATH, EXE_DIR},
    hac::{
        patch::{patch_nsp, repack_to_nsp, unpack_to_fs},
        rom::Nsp,
    },
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
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;
    ctrlc::set_handler(move || {
        eprintln!("{}", style("Process terminated by the user!").red().bold());
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
    if cli.command.is_some() || cli.import_keyfile.is_some() {
        cli_mode = true;
    }

    match run(cli) {
        Ok(_) => {
            info!("Done");
            Ok(())
        }
        Err(err) => {
            error!(?err);
            #[cfg(all(
                target_arch = "x86_64",
                any(target_os = "windows", target_os = "linux")
            ))]
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
    let mut config = Config::load()?;

    if let Some(keyfile) = cli.import_keyfile {
        if keyfile
            .extension()
            .and_then(OsStr::to_str)
            .ok_or_else(|| eyre!("File should've an extension"))?
            != "keys"
        {
            bail!("Invalid keyfile");
        }

        info!(?keyfile, "Selected keyfile");
        let default_path = DEFAULT_PRODKEYS_PATH.as_path();
        fs::create_dir_all(
            default_path
                .parent()
                .ok_or_else(|| eyre!("Failed to find parent"))?,
        )?;
        fs::copy(keyfile, default_path)?;
        info!("Copied keys successfully to the C2 ^-^");
    }

    match cli.command {
        Some(CliArgs::Commands::Update(args)) => {
            // Cli mode
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            info!("Started patching!");
            patch_nsp(
                &mut Nsp::new(args.base)?,
                &mut Nsp::new(args.patch)?,
                default_outdir()?,
            )?;
        }
        Some(CliArgs::Commands::Repack(args)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            let outdir = if let Some(outdir) = args.outdir {
                outdir
            } else {
                default_outdir()?
            };

            repack_to_nsp(args.controlnca, args.romfsdir, args.exefsdir, outdir)?;
        }
        Some(CliArgs::Commands::Unpack(args)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            let prefix = if args.patch.is_some() {
                "base+patch."
            } else {
                "base."
            };

            let outdir = if let Some(outdir) = args.outdir {
                outdir
            } else {
                tempfile::Builder::new()
                    .prefix(prefix)
                    .tempdir_in(env::current_dir()?)?
                    .into_path()
            };

            let patch = if let Some(path) = args.patch {
                Some(Nsp::new(path)?)
            } else {
                None
            };

            unpack_to_fs(Nsp::new(args.base)?, patch, outdir)?;
        }
        Some(CliArgs::Commands::Config(new_config)) => {
            if let Some(roms_dir) = new_config.roms_dir {
                if !roms_dir.is_dir() {
                    bail!("\"{}\" is not a valid directory", roms_dir.display());
                }
                config.roms_dir = Some(roms_dir);
            }

            info!("Updating config at \"{}\"", APP_CONFIG_PATH.display());
            Config::store(config)?;
        }
        Some(CliArgs::Commands::Tui) => {
            tui(&mut config)?;
        }
        None => {
            // Interactive mode
            if cfg!(feature = "android-proot") {
                tui(&mut config)?;
            } else {
                #[cfg(all(
                    target_arch = "x86_64",
                    any(target_os = "windows", target_os = "linux")
                ))]
                {
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
                    let update_path =
                        pick_nsp_file().ok_or_else(|| eyre!("No file was selected"))?;
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
                        let patched = patch_nsp(
                            &mut Nsp::new(&base_path)?,
                            &mut Nsp::new(&update_path)?,
                            default_outdir()?,
                        )?;
                        rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Info)
                            .set_title("Patching successful")
                            .set_description(&format!(
                                "Patched file created at:\n\"{}\"",
                                patched.path.display()
                            ))
                            .show();
                    }
                }
            };
        }
    }

    Ok(())
}

fn tui(config: &mut Config) -> Result<()> {
    use walkdir::WalkDir;

    if config.roms_dir.is_none() {
        let prompt = inquire::Text::new("Enter the path to a directory:").with_help_message(
            "Help:\n1. This directory will be used to look for ROMs (base/update)\n\
            2. `prod.keys` from the given directory will be used, if any",
        );
        #[cfg(feature = "android-proot")]
        let prompt = prompt
            .with_default("/storage/emulated/0/yanu")
            .with_placeholder("for eg- /storage/emulated/0/SwitchcwRoms");
        let prompt_input = prompt.prompt()?;
        #[cfg(unix)]
        let prompt_input = String::from_utf8(tilde_expand(prompt_input.as_bytes()))?;
        let roms_dir = PathBuf::from(prompt_input);
        info!(?roms_dir);

        if !roms_dir.is_dir() {
            bail!("\"{}\" is not a valid directory", roms_dir.display());
        }
        config.roms_dir = Some(roms_dir);
        info!("Updating config at \"{}\"", APP_CONFIG_PATH.display());
        Config::store(config.clone())?;
    }

    let roms_dir = config
        .roms_dir
        .as_ref()
        .expect("roms_dir should've been Some()");

    if !DEFAULT_PRODKEYS_PATH.is_file() {
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
                inquire::Text::new("Enter the path to `prod.keys` keyfile:").prompt()?,
            ));
        }

        let keyfile_path = keyfile_path.expect("Keyfile path should've been Some()");
        info!(?keyfile_path, "Selected keyfile");

        let default_path = DEFAULT_PRODKEYS_PATH.as_path();
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
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|s| Some(s.to_ascii_lowercase()))
                    == Some("nsp".into())
        })
        .collect::<Vec<_>>();

    let mut base: Option<Nsp> = None;
    let mut options = roms_path
        .iter()
        .map(|entry| entry.file_name().to_string_lossy())
        .collect::<Vec<_>>();
    let choice = inquire::Select::new("Select BASE package:", options.clone()).prompt()?;
    for entry in &roms_path {
        if entry.file_name() == choice.as_ref() {
            base = Some(Nsp::new(entry.path())?);
        }
    }
    let mut base = base.expect(&format!(
        "Selected package \"{}\" should be in {:#?}",
        choice, roms_path
    ));

    let mut update: Option<Nsp> = None;
    options = options.into_iter().filter(|s| s != &choice).collect();
    let choice = inquire::Select::new("Select UPDATE package:", options).prompt()?;
    for entry in &roms_path {
        if entry.file_name().to_string_lossy() == choice {
            update = Some(Nsp::new(entry.path())?);
        }
    }
    let mut update = update.expect(&format!(
        "Selected package \"{}\" should be in {:#?}",
        choice, roms_path
    ));

    if inquire::Confirm::new("Are you sure?")
        .with_default(false)
        .prompt()?
    {
        info!("Started patching!");
        patch_nsp(&mut base, &mut update, default_outdir()?)?;
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
