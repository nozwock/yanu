use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
#[command(arg_required_else_help = true)]
pub enum Commands {
    #[command(short_flag = 'c')]
    Cli(Cli),
    /// Manage yanu's config
    #[command()]
    Config(Config),
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
pub struct Cli {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select update package
    #[arg(short, long, value_name = "FILE")]
    pub update: PathBuf,
    /// Select `prod.keys` keyfile
    #[arg(short, long, value_name = "FILE")]
    pub keyfile: Option<String>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(arg_required_else_help = true)]
pub struct Config {
    /// Set roms directory path
    #[arg(long, value_name = "PATH")]
    pub roms_dir: Option<PathBuf>,
}
