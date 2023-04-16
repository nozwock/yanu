#[cfg(not(any(
    all(
        target_arch = "x86_64",
        any(windows, unix),
        not(feature = "android-proot")
    ),
    all(target_arch = "aarch64", feature = "android-proot")
)))]
compile_error!("This traget configuration is not supported");

pub mod backend;
pub mod utils;
pub mod vfs;
