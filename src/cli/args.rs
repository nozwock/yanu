use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    // /// Change the directory in which to look for roms
    // #[cfg(target_os = "android")]
    // #[arg(long, value_name = "PATH")]
    // pub roms_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(short_flag = 'c')]
    Cli(Cli),
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
pub struct Cli {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select update package
    #[arg(short, long, value_name = "FILE")]
    pub update: PathBuf,
}
