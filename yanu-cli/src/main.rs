mod opts;

use std::{ffi::OsStr, path::PathBuf};

use clap::Parser;
use eyre::{bail, eyre, Result};
use fs_err as fs;
use libyanu_common::{
    config::Config,
    defines::{APP_CONFIG_PATH, DEFAULT_PRODKEYS_PATH, EXE_DIR},
    hac::{
        patch::{patch_nsp, repack_to_nsp, unpack_to_fs},
        rom::Nsp,
    },
};
use opts::YanuCli;
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

    // Exit signals handling
    ctrlc::set_handler(move || {
        eprintln!("\n{}", "Process terminated by the user, cleaning up...");
        error!("Process terminated by the user");
    })?;

    info!(
        os = std::env::consts::OS,
        version = env!("CARGO_PKG_VERSION"),
        "Launching {}",
        env!("CARGO_PKG_NAME"),
    );

    let opts = YanuCli::parse();
    let mut config = Config::load()?;

    if let Some(keyfile) = opts.import_keyfile {
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

    match opts.command {
        Some(opts::Commands::Update(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            info!("Started patching!");
            patch_nsp(
                &mut Nsp::new(opts.base)?,
                &mut Nsp::new(opts.update)?,
                default_outdir()?,
            )?;
        }
        Some(opts::Commands::Repack(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            let outdir = if let Some(outdir) = opts.outdir {
                outdir
            } else {
                default_outdir()?
            };

            repack_to_nsp(opts.controlnca, opts.romfsdir, opts.exefsdir, outdir)?;
        }
        Some(opts::Commands::Unpack(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            let prefix = if opts.update.is_some() {
                "base+patch."
            } else {
                "base."
            };

            let outdir = if let Some(outdir) = opts.outdir {
                outdir
            } else {
                tempfile::Builder::new()
                    .prefix(prefix)
                    .tempdir_in(std::env::current_dir()?)?
                    .into_path()
            };

            unpack_to_fs(
                Nsp::new(opts.base)?,
                opts.update.map(|f| Nsp::new(f).ok()).flatten(),
                outdir,
            )?;
        }
        Some(opts::Commands::Config(opts)) => {
            if let Some(roms_dir) = opts.roms_dir {
                if roms_dir.is_dir() {
                    config.roms_dir = Some(roms_dir);
                } else {
                    bail!("\"{}\" is not a valid directory", roms_dir.display());
                }
            }

            info!("Updating config at \"{}\"", APP_CONFIG_PATH.display());
            Config::store(config)?;
        }
        Some(opts::Commands::Tui) => {
            use walkdir::WalkDir;

            if config.roms_dir.is_none() {
                let prompt = inquire::Text::new("Enter the path to a directory:")
                    .with_help_message(
                        "Help:\n1. This directory will be used to look for ROMs (base/update)\n\
            2. `prod.keys` from the given directory will be used, if any",
                    );
                #[cfg(feature = "android-proot")]
                let prompt = prompt
                    .with_default("/storage/emulated/0/yanu")
                    .with_placeholder("for eg- /storage/emulated/0/SwitchcwRoms");
                let prompt_input = prompt.prompt()?;
                #[cfg(unix)]
                let prompt_input =
                    String::from_utf8(tilde_expand::tilde_expand(prompt_input.as_bytes()))?;
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
        }
        None => todo!(),
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
