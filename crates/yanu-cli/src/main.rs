use std::{path::PathBuf, time::Instant};

use clap::Parser;
use common::{
    defines::{APP_CONFIG_PATH, DEFAULT_PRODKEYS_PATH},
    log,
    utils::{ext_matches, get_disk_free, get_fmt_size, get_paths_size},
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
    utils::{formatted_nsp_rename, pack::pack_fs_data, unpack::unpack_nsp, update::update_nsp},
    vfs::{nsp::Nsp, validate_program_id, xci::xci_to_nsps},
};
use indicatif::HumanDuration;
use tracing::{debug, error, info, warn};
use yanu_cli::opts::{self, YanuCli};

// TODO: This but for specifics like file, and dir
macro_rules! path_exists {
    ($($a:expr),*) => {
        [$($a,)*]
        .into_iter()
        .filter_map(|path| path.and_then(|path| Some(fs::metadata(path))))
        .find(|meta| meta.is_err())
        .transpose()
    };
}

macro_rules! check_space_with_prompt {
    ($modifier:expr, $paths:expr, $disk_path:expr) => {{
        let recommended_space = bytesize::ByteSize(get_paths_size($paths)?.as_u64() * $modifier);
        let available_space = get_disk_free($disk_path)?;
        if recommended_space > available_space {
            warn!(?recommended_space, ?available_space);
            inquire::Confirm::new(&format!(
                "Insufficient Space ({} {}) Continue?",
                style(format!("Recommended: {:?}+", recommended_space)).yellow(),
                style(format!("Available: {:?}", available_space)).red()
            ))
            .with_default(false)
            .prompt()?
        } else {
            true
        }
    }};
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
        // TODO: read from `RUST_LOG`
        .with_max_level(tracing::Level::DEBUG)
        .event_format(log::CustomFmt)
        .with_writer(non_blocking)
        .init();

    // Exit signals handling
    ctrlc::set_handler(move || {
        eprintln!("\nProcess terminated by the user, cleaning up...");
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
    info!("Parsing args, exit on error");
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
            path_exists!(Some(&opts.base), Some(&opts.update))?;

            if let Some(program_id) = &opts.titleid {
                validate_program_id(program_id)?;
            }

            info!("Started patching!");
            timer = Some(Instant::now());
            let (mut patched, nacp_data, program_id) = update_nsp(
                &mut Nsp::try_new(opts.base)?,
                &mut Nsp::try_new(opts.update)?,
                opts.titleid.as_deref(),
                opts.outdir.unwrap_or(default_outdir()?),
                &config,
            )?;
            formatted_nsp_rename(
                &mut patched.path,
                &nacp_data,
                &program_id,
                concat!("[yanu-", env!("CARGO_PKG_VERSION"), "-patched]"),
            )?;
            eprintln!(
                "{} '{}'",
                style("Patched NSP created at").green().bold(),
                patched.path.display()
            );
        }
        Some(opts::Commands::Pack(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            // Path validation
            // ?let clap do this instead
            path_exists!(
                Some(&opts.controlnca),
                Some(&opts.romfsdir),
                Some(&opts.exefsdir)
            )?;

            validate_program_id(&opts.titleid)?;

            timer = Some(Instant::now());
            let (mut patched, nacp_data) = pack_fs_data(
                opts.controlnca,
                opts.titleid.clone(),
                opts.romfsdir,
                opts.exefsdir,
                opts.outdir.unwrap_or(default_outdir()?),
                &config,
            )?;
            formatted_nsp_rename(
                &mut patched.path,
                &nacp_data,
                &opts.titleid,
                concat!("[yanu-", env!("CARGO_PKG_VERSION"), "-packed]"),
            )?;
            eprintln!(
                "{} '{}'",
                style("Packed NSP created at").green().bold(),
                patched.path.display()
            );
        }
        Some(opts::Commands::Unpack(opts)) => {
            if !DEFAULT_PRODKEYS_PATH.is_file() {
                bail!("Failed to find keyfile");
            }

            // Path validation
            path_exists!(Some(&opts.base), opts.update.as_ref())?;

            let prefix = if opts.update.is_some() {
                "base+patch."
            } else {
                "base."
            };

            let outdir = opts.outdir.unwrap_or(
                tempfile::Builder::new()
                    .prefix(prefix)
                    .tempdir_in(std::env::current_dir()?)?
                    .into_path(),
            );
            timer = Some(Instant::now());
            unpack_nsp(
                &mut Nsp::try_new(opts.base)?,
                opts.update.and_then(|f| Nsp::try_new(f).ok()).as_mut(),
                &outdir,
                &config,
            )?;
            eprintln!(
                "{} '{}'",
                style("Unpacked to").green().bold(),
                outdir.display()
            );
        }
        Some(opts::Commands::Convert(opts)) => {
            path_exists!(Some(&opts.file), opts.outdir.as_ref())?;

            let outdir = opts.outdir.unwrap_or(default_outdir()?);

            match opts.kind {
                opts::ConvertKind::Nsp => {
                    match opts.file.extension().map(|ext| ext.to_ascii_lowercase()) {
                        Some(ext) if ext == "xci" => {
                            timer = Some(Instant::now());
                            let nsps = xci_to_nsps(opts.file, outdir, &config.temp_dir)?;
                            println!("{}", style("\nPath to converted NSPs:").bold().underlined());
                            for nsp in nsps {
                                println!(
                                    "'{}' {}",
                                    nsp.path.display(),
                                    style(format!(
                                        "({})",
                                        get_fmt_size(&nsp.path).unwrap_or_default()
                                    ))
                                    .bold()
                                    .dim()
                                );
                            }
                        }
                        Some(ext) => bail!(
                            "Not supported conversion '{} -> {:?}'",
                            ext.to_string_lossy(),
                            opts.kind
                        ),
                        None => bail!("Non Unicode chars"),
                    }
                }
            }
        }
        Some(opts::Commands::Config(opts)) => {
            if let Some(yanu_dir) = opts.yanu_dir {
                if yanu_dir.is_dir() {
                    config.yanu_dir = Some(dbg!(&yanu_dir).canonicalize()?);
                } else {
                    bail!("'{}' is not a valid directory", yanu_dir.display());
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
        Some(opts::Commands::Tui) => {
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
                let yanu_dir = PathBuf::from(prompt_input);
                info!(?yanu_dir);

                if !yanu_dir.is_dir() {
                    bail!("'{}' is not a valid directory", yanu_dir.display());
                }
                config.yanu_dir = Some(yanu_dir.canonicalize()?);
                info!("Updating config at '{}'", APP_CONFIG_PATH.display());
                config.clone().store()?;
            }

            let yanu_dir = config
                .yanu_dir
                .as_ref()
                .expect("Should've been Some() as it's handeled above");

            if !DEFAULT_PRODKEYS_PATH.is_file() {
                // Looking for `prod.keys` in yanu_dir
                let keyfile_path = match WalkDir::new(yanu_dir)
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

            let roms_path = WalkDir::new(yanu_dir)
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|entry| entry.file_type().is_file() && ext_matches(entry.path(), "nsp"))
                .collect::<Vec<_>>();

            let options = roms_path
                .iter()
                .map(|entry| {
                    entry.file_name().to_str().unwrap_or_else(|| panic!("'{}' should've valid Unicode",
                        entry.path().display()))
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
                .unwrap_or_else(|| panic!("Selected package '{}' should be in {:#?}",
                    choice, roms_path));

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
                .unwrap_or_else(|| panic!("Selected package '{}' should be in {:#?}",
                    choice, roms_path));

            if !check_space_with_prompt!(2, &[&base.path, &update.path], &config.temp_dir) {
                return Ok(());
            }

            if inquire::Confirm::new("Are you sure?")
                .with_default(false)
                .prompt()?
            {
                info!("Started patching!");
                timer = Some(Instant::now());
                // TODO?: Maybe ask for ProgramID here?
                let (mut patched, nacp_data, program_id) =
                    update_nsp(&mut base, &mut update, None, default_outdir()?, &config)?;
                formatted_nsp_rename(
                    &mut patched.path,
                    &nacp_data,
                    &program_id,
                    concat!("[yanu-", env!("CARGO_PKG_VERSION"), "-patched]"),
                )?;
                eprintln!(
                    "{} '{}'",
                    style("Patched NSP created at").green().bold(),
                    patched.path.display()
                );
            }
        }
        #[cfg(unix)]
        Some(opts::Commands::SetupBackend { build }) => {
            use common::{defines::APP_CACHE_DIR, error::MultiReport};

            // List must be exhuastive
            let mut res_pool = vec![];
            if build {
                res_pool.push(Backend::build(BackendKind::Hacpack));
                res_pool.push(Backend::build(BackendKind::Hactool));
                res_pool.push(Backend::build(BackendKind::Hac2l));
                res_pool.push(Backend::build(BackendKind::FourNXCI));
            } else {
                res_pool.push(Backend::try_new(BackendKind::Hacpack));
                res_pool.push(Backend::try_new(BackendKind::Hactool));
                res_pool.push(Backend::try_new(BackendKind::Hac2l));
                res_pool.push(Backend::try_new(BackendKind::FourNXCI));
            }
            #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
            res_pool.push(Backend::try_new(BackendKind::Hactoolnet));
            let res_pool: Vec<_> = res_pool.into_iter().filter_map(|res| res.err()).collect();
            if res_pool.is_empty() {
                eprintln!(
                    "{} {}",
                    style("Successfully built backend!").green().bold(),
                    style(format!("({})", APP_CACHE_DIR.display())).bold().dim()
                );
            } else {
                let err = MultiReport::new(res_pool);
                bail!(err.join("\n"));
            }
        }
        None => {}
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
