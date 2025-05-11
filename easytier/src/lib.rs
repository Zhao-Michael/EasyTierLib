#![allow(dead_code)]

#[macro_use]
extern crate rust_i18n;

mod arch;
mod easytier_core;
mod gateway;
mod instance;
mod peer_center;
mod vpn_portal;

pub mod common;
pub mod connector;
pub mod launcher;
pub mod peers;
pub mod proto;
pub mod tunnel;
pub mod utils;
pub mod web_client;

#[cfg(test)]
mod tests;

use std::ffi::CStr;
use std::os::raw::c_char;

pub const VERSION: &str = common::constants::EASYTIER_VERSION;
rust_i18n::i18n!("locales", fallback = "en");

#[unsafe(no_mangle)]
pub extern "C" fn run(config_path: *const c_char) {
    let c_str = unsafe {
        CStr::from_ptr(config_path)
            .to_str()
            .unwrap_or("Error decoding config_path")
    };
    // Use a runtime to drive the future if main returns one
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(easytier_core::main(c_str));
}
