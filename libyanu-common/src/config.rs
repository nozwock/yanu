use confy::ConfyError;
use eyre::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::warn;

use crate::defines::{APP_CONFIG_PATH, TEMP_DIR_IN};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub roms_dir: Option<PathBuf>,
    pub temp_dir: PathBuf,
    #[cfg(unix)]
    pub hacpack_rev: String,
    #[cfg(unix)]
    pub hactool_rev: String,
    #[cfg(unix)]
    pub hac2l_rev: String,
    #[cfg(unix)]
    pub atmosphere_rev: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            #[cfg(unix)]
            hacpack_rev: "7845e7be8d03a263c33430f9e8c2512f7c280c88".into(),
            #[cfg(unix)]
            hactool_rev: "c2c907430e674614223959f0377f5e71f9e44a4a".into(),
            #[cfg(unix)]
            hac2l_rev: "7fc1b3a32c6a870c47d7459b23fd7c7b63014186".into(),
            #[cfg(unix)]
            atmosphere_rev: "1afb184c143f4319e5d6d4ea27260e61830c42a0".into(),
            roms_dir: Default::default(),
            temp_dir: TEMP_DIR_IN.to_owned(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let cfg: Self = match confy::load_path(APP_CONFIG_PATH.as_path()) {
            Ok(cfg) => cfg,
            Err(ConfyError::BadRonData(err)) => {
                warn!(?err, "BadConfig! Rewriting config...");
                fs::remove_file(APP_CONFIG_PATH.as_path())?;
                confy::load_path(APP_CONFIG_PATH.as_path())?
            }
            Err(err) => bail!(err),
        };
        Ok(cfg)
    }
    pub fn store(self) -> Result<()> {
        confy::store_path(APP_CONFIG_PATH.as_path(), self)?;
        Ok(())
    }
}
