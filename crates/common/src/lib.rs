#[cfg(not(any(
    all(
        target_arch = "x86_64",
        any(windows, unix),
        not(feature = "android-proot")
    ),
    all(target_arch = "aarch64", feature = "android-proot")
)))]
compile_error!("This target configuration is not supported");

pub mod defines;
pub mod error;
pub mod filename;
pub mod format;
pub mod log;
pub mod utils;

#[cfg(target_os = "android")]
pub mod termux;
