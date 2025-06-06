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

mod helper;
#[cfg(test)]
mod tests;

use crate::easytier_core::init_instance;
use crate::helper::{g_instance, get_stats, get_token, run};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::thread::sleep;
use std::time::Duration;

pub const VERSION: &str = common::constants::EASYTIER_VERSION;
rust_i18n::i18n!("locales", fallback = "en");

#[unsafe(no_mangle)]
pub extern "C" fn start(config_path: *const c_char) {
    let c_str = unsafe {
        CStr::from_ptr(config_path)
            .to_str()
            .unwrap_or("Error decoding config_path")
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        init_instance(c_str).await;
    });
    run(c_str)
}

#[unsafe(no_mangle)]
pub extern "C" fn stop() {
    get_token().cancel()
}

#[unsafe(no_mangle)]
pub extern "C" fn status() -> usize {
    let mut result: usize = 0;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        result = get_stats().await as usize;
    });
    result
}

#[no_mangle]
pub extern "C" fn free_string(p: *mut c_char) {
    unsafe {
        if p.is_null() {
            return;
        }
        let _ = CString::from_raw(p);
    }
}

pub(crate) fn main() {
    let path = r"%USERPROFILE%\source\repos\ConsoleApp1\ConsoleApp1\bin\Debug\net8.0\config.toml";
    let rt = tokio::runtime::Runtime::new().unwrap();

    rt.block_on(async {
        init_instance(path).await;
    });

    rt.spawn(async {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let ptr = get_stats().await as usize;
        println!("stats ptr: {:?}", ptr);
        let ptr = get_stats().await as usize;
        println!("stats ptr: {:?}", ptr);
        let ptr = get_stats().await as usize;
        println!("stats ptr: {:?}", ptr);
    });

    rt.block_on(easytier_core::run(path));
}
