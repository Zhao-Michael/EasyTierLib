use crate::common::config::TomlConfigLoader;
use crate::instance::instance::Instance;
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use std::option::Option;
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;

lazy_static! {
    pub static ref g_instance: RwLock<Option<Instance>> = RwLock::new(None);
}

pub fn init_instance(cfg: TomlConfigLoader) {
    let mut guard = g_instance.write().unwrap();
    *guard = Some(Instance::new(cfg));
}

lazy_static! {
      static ref g_token: RwLock<CancellationToken> = RwLock::new(CancellationToken::new());
}

pub fn reset_token() {
    let new_token = CancellationToken::new();
    {
        let old_token = g_token.read().unwrap();
        if !old_token.is_cancelled() {
            old_token.cancel();
        }
    }
    let mut guard = g_token.write().unwrap();
    *guard = new_token;
}

pub fn get_token() -> CancellationToken {
    g_token.read().unwrap().clone()
}
