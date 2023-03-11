use once_cell::sync::Lazy;
use std::path::PathBuf;

pub const APP_NAME: &str = "yanu";
pub const APP_DIR: &str = "com.github.nozwock.yanu";

#[cfg(target_os = "windows")]
pub const HACPACK: &[u8] = include_bytes!("../resources/x86_64-windows/hacpack.exe");
#[cfg(target_os = "windows")]
pub const HACTOOL: &[u8] = include_bytes!("../resources/x86_64-windows/hactool.exe");
#[cfg(target_os = "windows")]
pub const HACTOOLNET: &[u8] = include_bytes!("../resources/x86_64-windows/hactoolnet.exe");
#[cfg(target_os = "windows")]
pub const HAC2L: &[u8] = include_bytes!("../resources/x86_64-windows/hac2l.exe");

// Hactoolnet v0.18
#[cfg(target_os = "linux")]
pub const HACTOOLNET: &[u8] = include_bytes!("../resources/x86_64-linux/hactoolnet");

pub static APP_CACHE_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::cache_dir().unwrap_or_default().join(APP_DIR));
pub static APP_CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().unwrap_or_default().join(APP_DIR));
pub static APP_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| APP_CONFIG_DIR.join("yanu.ron"));
pub static DEFAULT_PRODKEYS_PATH: Lazy<PathBuf> =
    Lazy::new(|| dirs::home_dir().unwrap().join(".switch").join("prod.keys"));
pub static DEFAULT_TITLEKEYS_PATH: Lazy<PathBuf> =
    Lazy::new(|| dirs::home_dir().unwrap().join(".switch").join("title.keys"));
