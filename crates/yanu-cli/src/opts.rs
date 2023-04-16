use crate::utils::get_section;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
#[command(after_help = get_section("Tip",r#"Remember that you can get help for subcommands too:
$ yanu-cli pack --help
Examples may or may not be included."#, "  "))]
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
    /// Pack to NSP
    #[command()]
    Pack(Pack),
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
#[command(after_long_help = get_section("Examples", r#"For packing unpacked NSP data (both base+update were unpacked):
$ yanu-cli pack \
            --controlnca './base+update.xxxxxx/patchdata/control.nca' \
            --titleid 'xxxxxxxxxxxxxxxx' \
            --romfsdir './base+update.xxxxxx/romfs' \
            --exefsdir './base+update.xxxxxx/exefs'
If only base was unpacked, get the control NCA from basedata.
"#, "  "))]
pub struct Pack {
    /// Set Control NCA, it's usually the NCA file around ~1MB in size
    #[arg(long, value_name = "FILE")]
    pub controlnca: PathBuf,
    /// Set TitleID
    #[arg(
        short,
        long,
        long_help = "Set TitleID\n\
        Look at the logs if you're using a wrong TitleID, it'll mention which TitleID to use instead."
    )]
    pub titleid: String,
    /// Set path to extracted romfs
    #[arg(long, value_name = "DIR")]
    pub romfsdir: PathBuf,
    /// Set path to extracted exefs
    #[arg(long, value_name = "DIR")]
    pub exefsdir: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(after_long_help = get_section("Examples", r#"For unpacking only single NSP:
$ yanu-cli unpack --base './path/to/base
For unpacking both base and update NSPs together (i.e. updating):
$ yanu-cli unpack --base '/path/to/base' --update '/path/to/update'"#, "  "))]
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
    // TODO: change `roms_dir` to `yanu_dir` once mod functionality is introduced
    /// Set roms directory path, used in prompt to look for ROMS
    #[arg(long, value_name = "DIR")]
    pub roms_dir: Option<PathBuf>,
    /// Temp files generated while patching will be stored here
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
