mod cache;
pub mod cli;
pub mod defines;
pub mod hac;
pub mod utils;

#[cfg(target_os = "android")]
pub mod termux;
