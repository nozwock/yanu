use std::path::PathBuf;

use anyhow::{Context, Result};

pub const APP_NAME: &str = "yanu";
pub const APP_DIR: &str = "com.github.nozwock.yanu";

#[cfg(target_os = "windows")]
pub const HACPACK: &[u8] = include_bytes!("../resources/x86_64-windows/hacpack.exe");
#[cfg(target_os = "windows")]
pub const HACTOOL: &[u8] = include_bytes!("../resources/x86_64-windows/hactool.exe");

pub fn app_cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_default().join(APP_DIR)
}

pub fn app_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_default().join(APP_DIR)
}

pub fn app_config_path() -> PathBuf {
    app_config_dir().join("yanu.ron")
}

pub fn get_keyset_path() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("Failed to find home dir")?
        .join(".switch")
        .join("prod.keys"))
}
