#![allow(dead_code)]

use std::io;

use clap::Command;
use clap_complete::Generator;

mod arch;
mod easytier_core;
mod gateway;
mod instance;
mod peer_center;
mod vpn_portal;

pub mod common;
pub mod connector;
pub mod instance_manager;
pub mod launcher;
pub mod instance_manager;
pub mod peers;
pub mod proto;
pub mod tunnel;
pub mod utils;
pub mod web_client;

mod helper;
#[cfg(test)]
mod tests;

use crate::helper::{get_stats, get_token, run};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::thread::sleep;
use std::time::Duration;

pub const VERSION: &str = common::constants::EASYTIER_VERSION;
rust_i18n::i18n!("locales", fallback = "en");

pub fn print_completions<G: Generator>(generator: G, cmd: &mut Command, bin_name: &str) {
    clap_complete::generate(generator, cmd, bin_name, &mut io::stdout());
}

#[unsafe(no_mangle)]
pub extern "C" fn start(config_path: *const c_char) {
    let c_str = unsafe {
        CStr::from_ptr(config_path)
            .to_str()
            .unwrap_or("Error decoding config_path")
    };
    run(c_str);
}

#[unsafe(no_mangle)]
pub extern "C" fn stop() {
    get_token().cancel();
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

fn free_string(p: *mut c_char) {
    unsafe {
        if p.is_null() {
            return;
        }
        let _ = CString::from_raw(p);
    }
}

pub(crate) fn main() {
    loop {
        {
            let path = r"config.toml";
            let rt = tokio::runtime::Runtime::new().unwrap();

            rt.spawn(async {
                tokio::time::sleep(Duration::from_secs(15)).await;
                stop();
                let ptr = get_stats().await as usize;
                println!("stats ptr: {:?}", ptr);
            });

            start(CString::new(path).unwrap().as_ptr());
        }

        sleep(Duration::from_secs(5));
    }
}
