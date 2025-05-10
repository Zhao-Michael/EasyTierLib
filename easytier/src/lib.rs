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

use std::env;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const VERSION: &str = common::constants::EASYTIER_VERSION;
rust_i18n::i18n!("locales", fallback = "en");

#[unsafe(no_mangle)]
pub extern "C" fn run(config_path: *const c_char) {
    unsafe {
        let c_str = CStr::from_ptr(config_path)
            .to_str()
            .unwrap_or("Error decoding config_path");
        println!("EasyTier run config_path string : {}", c_str);
        env::set_var("ET_CONFIG_FILE", c_str);
    }
    match env::var("ET_CONFIG_FILE") {
        Ok(val) => println!("ET_CONFIG_FILE  from ENV : {}", val),
        Err(e) => println!("Couldn't read ET_CONFIG_FILE from ENV : {}", e),
    }
    easytier_core::main();
}
