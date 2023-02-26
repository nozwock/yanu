mod cache;
pub mod cli;
// pub mod config;
pub mod defines;
pub mod hac;
pub mod utils;

#[cfg(target_os = "android")]
pub mod termux;
