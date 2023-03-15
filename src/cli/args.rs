use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Import `prod.keys` keyfile
    #[arg(long, value_name = "FILE")]
    pub import_keyfile: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Update NSPs in CLI mode
    #[command(short_flag = 'U')]
    Update(Update),
    /// Repack to NSP
    #[command()]
    Repack(Repack),
    /// Unpack NSPs
    #[command()]
    Unpack(Unpack),
    /// Manage yanu's config
    #[command()]
    Config(Config),
    /// Update NSPs in TUI mode
    #[command()]
    Tui,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Update {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select patch package
    #[arg(short, long, value_name = "FILE")]
    pub patch: PathBuf,
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
    /// Select patch package
    #[arg(short, long, value_name = "FILE")]
    pub patch: Option<PathBuf>,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Config {
    /// Set roms directory path
    #[arg(long, value_name = "PATH")]
    pub roms_dir: Option<PathBuf>,
}
