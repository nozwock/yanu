use common::filename::{self, UNICODE_REPLACEMENT_CHAR};
use eyre::{bail, Result};
use fs_err as fs;
use std::{
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};
use tracing::info;
use walkdir::WalkDir;

const NACP_FILENAME: &'static str = "control.nacp";

/// https://switchbrew.org/wiki/NACP_Format
///
/// Provides access to some of the data contained within a NACP file.
#[derive(Debug, Default, Clone)]
pub struct NacpData {
    pub title_entry: ApplicationTitle, // only first title entry instead of [Title; 0x10]
    pub application_version: [u8; 0x10],
}

impl NacpData {
    const TITLE_ENTRY_OFFSET: usize = 0x0;
    const APPLICATION_VERSION_OFFSET: usize = 0x3060;
}

#[derive(Debug, Clone)]
pub struct ApplicationTitle {
    pub application_name: [u8; 0x200],
    pub application_publisher: [u8; 0x100],
}

impl Default for ApplicationTitle {
    fn default() -> Self {
        Self {
            application_name: std::array::from_fn(|_| Default::default()),
            application_publisher: std::array::from_fn(|_| Default::default()),
        }
    }
}

impl NacpData {
    pub fn try_new<P: AsRef<Path>>(nacp_path: P) -> Result<Self> {
        if !nacp_path.as_ref().is_file() || !is_nacp(nacp_path.as_ref()) {
            bail!("'{}' is not a NACP file", nacp_path.as_ref().display());
        }

        info!("Reading NACP data");

        let mut nacp_data = NacpData::default();
        let mut nacp = fs::File::open(nacp_path.as_ref())?;

        nacp.seek(io::SeekFrom::Start(NacpData::TITLE_ENTRY_OFFSET as _))?;
        nacp.read_exact(nacp_data.title_entry.application_name.as_mut())?;
        nacp.read_exact(nacp_data.title_entry.application_publisher.as_mut())?;

        nacp.seek(io::SeekFrom::Start(
            NacpData::APPLICATION_VERSION_OFFSET as _,
        ))?;
        nacp.read_exact(nacp_data.application_version.as_mut())?;

        info!("Successfully read NACP data");

        Ok(nacp_data)
    }
    pub fn get_application_name(&self) -> String {
        NacpData::sanitize(&String::from_utf8_lossy(&self.title_entry.application_name))
    }
    pub fn get_application_publisher(&self) -> String {
        NacpData::sanitize(&String::from_utf8_lossy(
            &self.title_entry.application_publisher,
        ))
    }
    pub fn get_application_version(&self) -> String {
        NacpData::sanitize(&String::from_utf8_lossy(&self.application_version))
    }
    fn sanitize(s: &str) -> String {
        s.chars()
            .filter(|ch| ch != &UNICODE_REPLACEMENT_CHAR && !filename::is_forbidden(*ch))
            .collect()
    }
}

fn is_nacp<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().file_name() == Some(NACP_FILENAME.as_ref())
}

/// Returns the first Nacp file in a dir.
pub fn get_nacp_file<P: AsRef<Path>>(from: P) -> Option<PathBuf> {
    for entry in WalkDir::new(from.as_ref())
        .min_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_file() && is_nacp(entry.path()) {
            return Some(entry.into_path());
        }
    }
    None
}
