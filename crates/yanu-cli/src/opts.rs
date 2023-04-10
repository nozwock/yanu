use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Import `prod.keys` keyfile
    #[arg(short = 'k', long, value_name = "FILE")]
    pub keyfile: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Update NSP
    #[command()]
    Update(Update),
    /// Repack to NSP
    #[command()]
    Repack(Repack),
    /// Unpack NSP
    #[command()]
    Unpack(Unpack),
    /// Manage yanu's config
    #[command(visible_alias = "cfg")]
    Config(Config),
    /// Update NSP using prompt
    #[command()]
    UpdatePrompt,
    #[cfg(unix)]
    /// Build backend utilities
    #[command()]
    BuildBackend,
}

// TODO: add value parsers
// value_parser=clap::value_parser!(PathBuf)

#[derive(Debug, Args, Default, PartialEq, Eq)]
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
pub struct Repack {
    #[arg(long, value_name = "FILE")]
    pub controlnca: PathBuf,
    /// Set TitleID
    #[arg(short, long)]
    pub titleid: String,
    #[arg(long, value_name = "DIR")]
    pub romfsdir: PathBuf,
    #[arg(long, value_name = "DIR")]
    pub exefsdir: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
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

#[cfg(not(feature = "android-proot"))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum NspExtractor {
    #[default]
    Hactoolnet,
    Hactool,
}

#[cfg(not(feature = "android-proot"))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum NcaExtractor {
    #[default]
    Hactoolnet,
    Hac2l,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Config {
    /// Set roms directory path, used in prompt to look for ROMS
    #[arg(long, value_name = "DIR")]
    pub roms_dir: Option<PathBuf>,
    /// Temp files will be stored here while patching
    #[arg(
        long,
        value_name = "DIR",
        long_help = "Temp files will be stored here while patching\n\
        PATH must not contain Unicode characters due to the limitations of backend tools"
    )]
    pub temp_dir: Option<PathBuf>,
    #[cfg(not(feature = "android-proot"))]
    #[arg(long, value_enum)]
    pub nsp_extractor: Option<NspExtractor>,
    #[cfg(not(feature = "android-proot"))]
    #[arg(long, value_enum)]
    pub nca_extractor: Option<NcaExtractor>,
}
