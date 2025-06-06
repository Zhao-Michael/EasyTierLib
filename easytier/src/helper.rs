use crate::easytier_core;
use crate::instance::instance::Instance;
use crate::peers::rpc_service::PeerManagerRpcService;
use crate::proto::cli::PeerInfo;
use lazy_static::lazy_static;
use std::alloc::{alloc_zeroed, Layout};
use std::option::Option;
use std::ptr;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

lazy_static! {
    pub static ref g_instance: RwLock<Option<Instance>> = RwLock::new(None);
}

lazy_static! {
    static ref g_token: std::sync::RwLock<CancellationToken> =
        std::sync::RwLock::new(CancellationToken::new());
}

const SIZE: usize = 100 * 1024;

static mut BUF_PTR: *mut u8 = ptr::null_mut();

fn get_buffer() -> *mut u8 {
    unsafe {
        if BUF_PTR.is_null() {
            let layout = Layout::array::<u8>(SIZE).unwrap();
            BUF_PTR = alloc_zeroed(layout) as *mut u8;
        }
        BUF_PTR
    }
}

fn write_json(s: &str) {
    let ptr = get_buffer();
    unsafe {
        ptr::write_bytes(ptr, 0, SIZE);
    }
    let bytes = s.as_bytes();
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
    }
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

pub async fn get_stats() -> *mut u8 {
    let guard = g_instance.read().await;
    let pm = guard.as_ref().unwrap();
    let pmrs = PeerManagerRpcService::new(pm.get_peer_manager());
    let peers: Vec<PeerInfo> = pmrs.list_peers().await;
    let json = serde_json::to_string_pretty(&peers).unwrap();
    write_json(&*json);
    unsafe { BUF_PTR }
}
