use crate::utils::get_section;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

const SECTION_PADDING: &str = "  ";

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
#[command(after_help = get_section("Tip",r#"Remember that you can get help for subcommands too:
$ yanu-cli pack --help
Examples may or may not be included.
"#, SECTION_PADDING))]
pub struct YanuCli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Import `prod.keys` keyfile
    #[arg(short = 'k', long, value_name = "FILE")]
    pub keyfile: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Apply an update to a base NSP
    #[command()]
    Update(Update),
    /// Pack FS files to NSP
    #[command()]
    Pack(Pack),
    /// Unpack a NSP
    #[command()]
    Unpack(Unpack),
    /// Convert Switch file formats
    #[command()]
    Convert(Convert),
    /// Manage yanu's config
    #[command(visible_alias = "cfg")]
    Config(Config),
    #[command()]
    Tui,
    #[cfg(unix)]
    /// Builds or extracts embedded backend components;
    /// Useful when creating read-only containers
    #[command()]
    SetupBackend {
        /// Build backends that can be built
        #[arg(short, long, action)]
        build: bool,
    },
}

// TODO: Figure out value parsing
// value_parser=clap::value_parser!(PathBuf)

#[derive(Debug, Args, Default, PartialEq, Eq)]
pub struct Update {
    /// Select base package
    #[arg(short, long, value_name = "FILE")]
    pub base: PathBuf,
    /// Select update package
    #[arg(short, long, value_name = "FILE")]
    pub update: PathBuf,
    /// Overwrite TitleID
    #[arg(
        short,
        long,
        long_help = "Overwrite TitleID\n\
        Check the logs for guidance on which TitleID to use if using the wrong one."
    )]
    pub titleid: Option<String>,
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
"#, SECTION_PADDING))]
pub struct Pack {
    /// Set Control NCA, it's typically around 1MB in size.
    #[arg(long, value_name = "FILE")]
    pub controlnca: PathBuf,
    /// Set TitleID
    #[arg(
        short,
        long,
        long_help = "Set TitleID\n\
        Check the logs for guidance on which TitleID to use if using the wrong one."
    )]
    pub titleid: String,
    /// Set path to extracted main NCA's RomFS
    #[arg(long, value_name = "DIR")]
    pub romfsdir: PathBuf,
    /// Set path to extracted main NCA's ExeFS
    #[arg(long, value_name = "DIR")]
    pub exefsdir: PathBuf,
    #[arg(short, long, value_name = "DIR")]
    pub outdir: Option<PathBuf>,
}

#[derive(Debug, Args, Default, PartialEq, Eq)]
#[command(after_long_help = get_section("Examples", r#"For unpacking only single NSP:
$ yanu-cli unpack --base './path/to/base
For unpacking both base and update NSPs together (i.e. updating):
$ yanu-cli unpack --base '/path/to/base' --update '/path/to/update'
"#, SECTION_PADDING))]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum ConvertKind {
    Nsp,
}

#[derive(Debug, Args, PartialEq, Eq)]
#[command(after_help = get_section("Examples", r#"For converting XCI to NSP:
$ yanu-cli convert --kind nsp gta6.xci
"#, SECTION_PADDING))]
pub struct Convert {
    /// File format to convert to
    #[arg(
        short,
        long,
        value_enum,
        long_help = r#"File format to convert to
Possible coversions:
    To nsp: xci"#
    )]
    pub kind: ConvertKind,
    /// Input file
    #[arg()]
    pub file: PathBuf,
    /// By default it'll be 'pwd'
    #[arg(short, long)]
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
    /// Set Yanu directory path, used in tui to look for Game Packages and keys
    #[arg(long, value_name = "DIR")]
    pub yanu_dir: Option<PathBuf>,
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
