mod opts;

use std::{path::PathBuf, time::Instant};

use clap::Parser;
use common::{
    defines::{APP_CONFIG_PATH, DEFAULT_PRODKEYS_PATH},
    utils::ext_matches,
};
use config::Config;
#[cfg(not(feature = "android-proot"))]
use config::{NcaExtractor, NspExtractor};
use console::style;
use eyre::{bail, eyre, Result};
use fs_err as fs;
#[cfg(unix)]
use hac::backend::{Backend, BackendKind};
use hac::{
    utils::{repack::repack_fs_data, unpack::unpack_nsp, update::update_nsp},
    vfs::{nsp::Nsp, PROGRAMID_LEN},
};
use indicatif::HumanDuration;
use opts::YanuCli;
use tracing::{debug, error, info};

macro_rules! validate_paths {
    ($($a:expr),*) => {
        [$($a,)*]
        .into_iter()
        .filter_map(|path| path.and_then(|path| Some(fs::metadata(path))))
        .find(|meta| meta.is_err())
        .transpose()
    };
}

fn main() -> Result<()> {
    // Colorful errors
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;

    // Tracing
    let file_appender = tracing_appender::rolling::hourly(".", "yanu.log");
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
            bail!(err);
        }
    }
}

fn run() -> Result<()> {
    info!("Parsing args, will exit on error");
    let opts = YanuCli::parse();
    let mut config = Config::load()?;
    debug!(?config);

    if let Some(keyfile) = opts.keyfile {
        if !ext_matches(&keyfile, "keys") {
            bail!("Invalid keyfile");
        }
        info!(?keyfile, "Selected keyfile");

        let default_path = DEFAULT_PRODKEYS_PATH.as_path();
        fs::create_dir_all(
            default_path
                .parent()
                .ok_or_else(|| eyre!("Failed to find parent"))?,
        )?;
        fs::copy(&keyfile, default_path)?;
        info!("Copied keys successfully to the C2 ^-^");
        eprintln!(
            "{} '{}'",
            style("Copied keys successfully from").green().bold(),
            keyfile.display()
        )
    }

    let mut timer: Option<Instant> = None;
    match opts.command {
        Some(opts::Commands::Update(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            // Path validation
            validate_paths!(Some(&opts.base), Some(&opts.update))?;

            info!("Started patching!");
            timer = Some(Instant::now());
            eprintln!(
                "{} '{}'",
                style("Patched NSP created at").green().bold(),
                update_nsp(
                    &mut Nsp::try_new(opts.base)?,
                    &mut Nsp::try_new(opts.update)?,
                    default_outdir()?,
                )?
                .path
                .display()
            );
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

            // Path validation
            // ?let clap do this instead
            validate_paths!(
                Some(&opts.controlnca),
                Some(&opts.romfsdir),
                Some(&opts.exefsdir)
            )?;

            if opts.titleid.len() != PROGRAMID_LEN as _ {
                bail!(
                    "len: {} '{}' is invalid TitleID, TitleID should be in hexadecimal with a size of 8 bytes, i.e. 16 hexadecimal characters",
                    opts.titleid.len(),
                    opts.titleid
                )
            }

            timer = Some(Instant::now());
            eprintln!(
                "{} '{}'",
                style("Repacked NSP created at").green().bold(),
                repack_fs_data(
                    opts.controlnca,
                    opts.titleid,
                    opts.romfsdir,
                    opts.exefsdir,
                    outdir
                )?
                .path
                .display()
            );
        }
        Some(opts::Commands::Unpack(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            // Path validation
            validate_paths!(Some(&opts.base), opts.update.as_ref())?;

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

            timer = Some(Instant::now());
            unpack_nsp(
                Nsp::try_new(opts.base)?,
                opts.update.map(|f| Nsp::try_new(f).ok()).flatten(),
                &outdir,
            )?;
            eprintln!(
                "{} '{}'",
                style("Unpacked to").green().bold(),
                outdir.display()
            );
        }
        Some(opts::Commands::Config(opts)) => {
            if let Some(roms_dir) = opts.roms_dir {
                if roms_dir.is_dir() {
                    config.yanu_dir = Some(dbg!(&roms_dir).canonicalize()?);
                } else {
                    bail!("'{}' is not a valid directory", roms_dir.display());
                }
            }

            if let Some(temp_dir) = opts.temp_dir {
                if !temp_dir.as_os_str().is_ascii() {
                    bail!(
                        "Temp dir path must not contain Unicode characters due to the limitations of backend tools"
                    )
                }
                if temp_dir.is_dir() {
                    config.temp_dir = dbg!(&temp_dir).canonicalize()?;
                } else {
                    bail!("'{}' is not a valid directory", temp_dir.display());
                }
            }

            #[cfg(not(feature = "android-proot"))]
            if let Some(nsp_extractor) = opts.nsp_extractor {
                // ? How to do this better? and also not have dup enums
                config.nsp_extractor = match dbg!(nsp_extractor) {
                    opts::NspExtractor::Hactoolnet => NspExtractor::Hactoolnet,
                    opts::NspExtractor::Hactool => NspExtractor::Hactool,
                };
            }

            #[cfg(not(feature = "android-proot"))]
            if let Some(nsp_extractor) = opts.nca_extractor {
                config.nca_extractor = match dbg!(nsp_extractor) {
                    opts::NcaExtractor::Hactoolnet => NcaExtractor::Hactoolnet,
                    opts::NcaExtractor::Hac2l => NcaExtractor::Hac2l,
                };
            }

            info!("Updating config at '{}'", APP_CONFIG_PATH.display());
            Config::store(config)?;
            eprintln!("{}", style("Successfully modified config").green().bold());
        }
        Some(opts::Commands::UpdatePrompt) => {
            use walkdir::WalkDir;

            if config.yanu_dir.is_none() {
                let prompt = inquire::Text::new("Enter the path to a directory:")
                    .with_help_message(
                        "Help:\n1. This directory will be used to look for ROMs (base/update)\n\
            2. `prod.keys` from the given directory will be used, if any",
                    );
                #[cfg(feature = "android-proot")]
                let prompt = prompt
                    .with_default("/storage/emulated/0/yanu")
                    .with_placeholder("for eg- /storage/emulated/0/SwitchRoms");
                let prompt_input = prompt.prompt()?;
                #[cfg(unix)]
                let prompt_input =
                    String::from_utf8(tilde_expand::tilde_expand(prompt_input.as_bytes()))?;
                let roms_dir = PathBuf::from(prompt_input);
                info!(?roms_dir);

                if !roms_dir.is_dir() {
                    bail!("'{}' is not a valid directory", roms_dir.display());
                }
                config.yanu_dir = Some(roms_dir.canonicalize()?);
                info!("Updating config at '{}'", APP_CONFIG_PATH.display());
                config.clone().store()?;
            }

            let yanu_dir = config
                .yanu_dir
                .as_ref()
                .expect("Should've been Some() as it's handeled above");

            if !DEFAULT_PRODKEYS_PATH.is_file() {
                // Looking for `prod.keys` in yanu_dir
                let keyfile_path = match WalkDir::new(&yanu_dir)
                    .min_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .find(|entry| entry.file_name() == "prod.keys")
                    .map(|entry| entry.into_path())
                {
                    Some(path) => path,
                    None => {
                        eprintln!(
                            "{} '{}'",
                            style("Couldn't find keyfile in").red().bold(),
                            yanu_dir.display()
                        );
                        PathBuf::from(
                            inquire::Text::new("Enter the path to `prod.keys` keyfile:")
                                .prompt()?,
                        )
                    }
                };
                info!(?keyfile_path, "Selected keyfile");

                if !ext_matches(&keyfile_path, "keys") {
                    bail!("Invalid keyfile");
                }

                let default_path = DEFAULT_PRODKEYS_PATH.as_path();
                fs::create_dir_all(
                    default_path
                        .parent()
                        .ok_or_else(|| eyre!("Failed to find parent"))?,
                )?;
                fs::copy(keyfile_path, default_path)?;
                info!("Copied keys successfully to the C2 ^-^");
            }

            let roms_path = WalkDir::new(&yanu_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|entry| entry.file_type().is_file() && ext_matches(entry.path(), "nsp"))
                .collect::<Vec<_>>();

            let options = roms_path
                .iter()
                .map(|entry| {
                    entry.file_name().to_str().expect(&format!(
                        "'{}' should've valid Unicode",
                        entry.path().display()
                    ))
                })
                .collect::<Vec<_>>();
            if options.is_empty() {
                bail!("No NSPs found in '{}'", yanu_dir.display());
            }
            let choice = inquire::Select::new("Select BASE package:", options.clone()).prompt()?;
            let mut base = roms_path
                .iter()
                .find(|entry| entry.file_name() == choice)
                .map(|entry| Nsp::try_new(entry.path()))
                .transpose()?
                .expect(&format!(
                    "Selected package '{}' should be in {:#?}",
                    choice, roms_path
                ));

            let options = options
                .into_iter()
                .filter(|filename| filename != &choice)
                .collect::<Vec<_>>();
            if options.is_empty() {
                bail!("No other NSPs found in '{}'", yanu_dir.display());
            }
            let choice = inquire::Select::new("Select UPDATE package:", options).prompt()?;
            let mut update = roms_path
                .iter()
                .find(|entry| entry.file_name() == choice)
                .map(|entry| Nsp::try_new(entry.path()))
                .transpose()?
                .expect(&format!(
                    "Selected package '{}' should be in {:#?}",
                    choice, roms_path
                ));

            if inquire::Confirm::new("Are you sure?")
                .with_default(false)
                .prompt()?
            {
                info!("Started patching!");
                timer = Some(Instant::now());
                eprintln!(
                    "{} '{}'",
                    style("Patched NSP created at").green().bold(),
                    update_nsp(&mut base, &mut update, default_outdir()?)?
                        .path
                        .display()
                );
            }
        }
        #[cfg(unix)]
        Some(opts::Commands::BuildBackend) => {
            #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
            Backend::try_new(BackendKind::Hactoolnet)?;
            Backend::try_new(BackendKind::Hactool)?;
            Backend::try_new(BackendKind::Hac2l)?;
            Backend::try_new(BackendKind::Hacpack)?;
            eprintln!("{}", style("Successfully built backend").green().bold());
        }
        None => unreachable!(),
    }

    if let Some(timer) = timer {
        eprintln!(
            "{} {}",
            style("Process completed").green().bold(),
            style(format!("({})", HumanDuration(timer.elapsed())))
                .bold()
                .dim()
        );
    }

    Ok(())
}

fn default_outdir() -> Result<PathBuf> {
    let outdir: PathBuf = {
        if cfg!(feature = "android-proot") {
            PathBuf::from("/storage/emulated/0")
        } else {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            std::env::current_dir()?
        }
    };

    if !outdir.is_dir() {
        bail!("Failed to set '{}' as outdir", outdir.display());
    }

    Ok(outdir)
}
