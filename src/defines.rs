use std::path::PathBuf;

pub const APP_DIR: &str = "com.github.nozwock.yanu";

#[cfg(target_os = "linux")]
mod embed {
    pub const HACPACK: &[u8] = include_bytes!("../resources/x86_64-linux/hacpack");
    pub const HACTOOL: &[u8] = include_bytes!("../resources/x86_64-linux/hactool");
}

#[cfg(target_os = "windows")]
mod embed {
    pub const HACPACK: &[u8] = include_bytes!("../resources/x86_64-windows/hacpack");
    pub const HACTOOL: &[u8] = include_bytes!("../resources/x86_64-windows/hactool");
}

#[cfg(target_os = "android")]
mod embed {
    pub const HACPACK: &[u8] = include_bytes!("../resources/aarch64-android/hacpack");
    pub const HACTOOL: &[u8] = include_bytes!("../resources/aarch64-android/hactool");
}

pub fn app_cache_dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join(APP_DIR))
}
