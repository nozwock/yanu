use once_cell::sync::Lazy;
use std::path::PathBuf;

pub const APP_NAME: &str = "yanu";
pub const APP_DIR: &str = "com.github.nozwock.yanu";

#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
pub const HACPACK: &[u8] = include_bytes!("../../resources/x86_64-windows/hacpack.exe");
#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
pub const HACTOOL: &[u8] = include_bytes!("../../resources/x86_64-windows/hactool.exe");
#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
pub const HACTOOLNET: &[u8] = include_bytes!("../../resources/x86_64-windows/hactoolnet.exe");
#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
pub const HAC2L: &[u8] = include_bytes!("../../resources/x86_64-windows/hac2l.exe");

// Hactoolnet v0.18
#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
pub const HACTOOLNET: &[u8] = include_bytes!("../../resources/x86_64-linux/hactoolnet");

#[cfg(feature = "android-proot")]
pub const HAC2L: &[u8] = include_bytes!("../../resources/aarch64-linux/hac2l");

pub static APP_CACHE_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::cache_dir().unwrap_or_default().join(APP_DIR));
pub static APP_CONFIG_DIR: Lazy<PathBuf> =
    Lazy::new(|| dirs::config_dir().unwrap_or_default().join(APP_DIR));
pub static APP_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| APP_CONFIG_DIR.join("yanu.ron"));
pub static DEFAULT_PRODKEYS_PATH: Lazy<PathBuf> =
    Lazy::new(|| dirs::home_dir().unwrap().join(".switch").join("prod.keys"));
pub static DEFAULT_TITLEKEYS_PATH: Lazy<PathBuf> =
    Lazy::new(|| dirs::home_dir().unwrap().join(".switch").join("title.keys"));

pub static EXE_DIR: Lazy<PathBuf> =
    Lazy::new(|| std::env::current_exe().unwrap().parent().unwrap().into());
#[cfg(not(feature = "android-proot"))]
pub static TEMP_DIR_IN: Lazy<PathBuf> = Lazy::new(|| std::env::temp_dir());
#[cfg(feature = "android-proot")]
pub static TEMP_DIR_IN: Lazy<PathBuf> = Lazy::new(|| dirs::home_dir().unwrap());
