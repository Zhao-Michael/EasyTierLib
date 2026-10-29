#![allow(dead_code)]

#[cfg(feature = "management")]
use std::ffi::{CStr, CString, c_char};
use std::io;

use clap::Command;
use clap_complete::{Generator, Shell};

mod arch;
mod gateway;
mod host_runtime;
pub mod instance;
mod vpn_portal;

pub mod common;
#[cfg(feature = "management")]
pub mod core;
#[cfg(all(feature = "management", feature = "cli"))]
pub mod helper;
#[cfg(feature = "upnp")]
pub(crate) mod igd_next;
pub mod proto;
#[cfg(feature = "management-rpc")]
pub mod rpc_service;
#[cfg(feature = "management")]
pub mod service_manager;
pub(crate) mod socket;
pub mod socket_protector;
pub mod tunnel;
pub mod utils;
#[cfg(feature = "web-client")]
pub mod web_client;

#[cfg(all(feature = "management", feature = "cli"))]
use crate::helper::{get_stats, get_token, is_running, remote_status, run};

#[cfg(test)]
mod tests;

pub const VERSION: &str = common::constants::EASYTIER_VERSION;
rust_i18n::i18n!("locales", fallback = "en");

#[derive(clap::ValueEnum, Debug, Clone, PartialEq)]
pub enum ShellType {
    Bash,
    Elvish,
    Fish,
    Powershell,
    Zsh,
    Nu,
}

impl ShellType {
    pub fn to_shell(&self) -> Option<Shell> {
        match self {
            ShellType::Bash => Some(Shell::Bash),
            ShellType::Elvish => Some(Shell::Elvish),
            ShellType::Fish => Some(Shell::Fish),
            ShellType::Powershell => Some(Shell::PowerShell),
            ShellType::Zsh => Some(Shell::Zsh),
            ShellType::Nu => None,
        }
    }
}

pub fn print_completions<G: Generator>(generator: G, cmd: &mut Command, bin_name: &str) {
    clap_complete::generate(generator, cmd, bin_name, &mut io::stdout());
}

pub fn print_nushell_completions(cmd: &mut Command, bin_name: &str) {
    clap_complete::generate(
        clap_complete_nushell::Nushell,
        cmd, bin_name,
        &mut io::stdout(),
    );
}

// ===== DLL (cdylib) C ABI, kept for compatibility with EasyTier.dll consumers =====

#[cfg(all(feature = "management", feature = "cli"))]
#[unsafe(no_mangle)]
pub extern "C" fn start(config_path: *const c_char) {
    let c_str = unsafe {
        CStr::from_ptr(config_path)
            .to_str()
            .unwrap_or("Error decoding config_path")
    };
    run(c_str);
}

#[cfg(all(feature = "management", feature = "cli"))]
#[unsafe(no_mangle)]
pub extern "C" fn stop() {
    get_token().cancel();
}

#[cfg(all(feature = "management", feature = "cli"))]
#[unsafe(no_mangle)]
pub extern "C" fn status() -> usize {
    let mut result: usize = 0;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        result = get_stats().await as usize;
    });
    result
}

#[cfg(all(feature = "management", feature = "cli"))]
#[unsafe(no_mangle)]
pub extern "C" fn rstatus() -> usize {
    remote_status() as usize
}

#[cfg(all(feature = "management", feature = "cli"))]
#[unsafe(no_mangle)]
pub extern "C" fn isrunning() -> bool {
    is_running()
}

#[cfg(all(feature = "management", feature = "cli"))]
fn free_string(p: *mut c_char) {
    unsafe {
        if p.is_null() {
            return;
        }
        let _ = CString::from_raw(p);
    }
}
