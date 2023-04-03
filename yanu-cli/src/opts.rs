use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Import `prod.keys` keyfile
    #[arg(short = 'k', long, value_name = "FILE")]
    pub import_keyfile: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Update NSPs
    #[command()]
    Update(Update),
    /// Repack to NSP
    #[command()]
    Repack(Repack),
    /// Unpack NSPs
    #[command()]
    Unpack(Unpack),
    /// Manage yanu's config
    #[command(visible_alias = "cfg")]
    Config(Config),
    /// Update NSPs in TUI mode
    #[command()]
    UpdateTui,
    #[cfg(unix)]
    /// Build backend utilities
    #[command()]
    BuildBackend,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Update {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select update package
    #[arg(short, long, value_name = "FILE")]
    pub update: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Repack {
    #[arg(long, value_name = "FILE")]
    pub controlnca: PathBuf,
    #[arg(long, value_name = "DIR")]
    pub romfsdir: PathBuf,
    #[arg(long, value_name = "DIR")]
    pub exefsdir: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Unpack {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select update package
    #[arg(short, long, value_name = "FILE")]
    pub update: Option<PathBuf>,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Config {
    /// Set roms directory path, used in TUI to look for ROMS
    #[arg(long, value_name = "DIR")]
    pub roms_dir: Option<PathBuf>,
    /// Temp files will be stored here while patching,
    /// PATH must not contain Unicode characters due to a limitation of Backend tools
    #[arg(long, value_name = "DIR")]
    pub temp_dir: Option<PathBuf>,
}
