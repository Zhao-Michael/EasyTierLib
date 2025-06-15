use crate::easytier_core;
use crate::instance::instance::Instance;
use crate::peers::rpc_service::PeerManagerRpcService;
use crate::proto::cli::PeerInfo;
use lazy_static::lazy_static;
use std::option::Option;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

lazy_static! {
    pub static ref g_instance: RwLock<Option<Instance>> = RwLock::new(None);
}

lazy_static! {
    static ref g_token: std::sync::RwLock<CancellationToken> =
        std::sync::RwLock::new(CancellationToken::new());
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

pub(crate) fn run(path: &str) {
    reset_token();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(easytier_core::run(path));
}

pub async fn get_stats() -> String {
    let guard = g_instance.read().await;
    let pm = guard.as_ref().unwrap();
    let pmrs = PeerManagerRpcService::new(pm.get_peer_manager());
    let peers: Vec<PeerInfo> = pmrs.list_peers().await;
    serde_json::to_string_pretty(&peers).unwrap()
}
