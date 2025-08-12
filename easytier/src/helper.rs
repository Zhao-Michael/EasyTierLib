use crate::easytier_core::{run_main, Cli};
use crate::instance_manager::NetworkInstanceManager;
use crate::peers::peer_manager::PeerManager;
use crate::peers::rpc_service::PeerManagerRpcService;
use crate::proto::cli::{list_peer_route_pair, NodeInfo, PeerManageRpc, ShowNodeInfoRequest};
use crate::proto::rpc_types::controller::BaseController;
use crate::utils::{cost_to_str, float_to_str, PeerRoutePair};
use cidr::Ipv4Inet;
use clap::Parser;
use humansize::format_size;
use lazy_static::lazy_static;
use std::alloc::{alloc_zeroed, Layout};
use std::ptr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

lazy_static! {
    pub static ref g_peermanager: RwLock<Option<Arc<PeerManager>>> = RwLock::new(None);
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
    rt.block_on(start_run(path));
}

pub(crate) async fn is_running() -> bool {
    let guard = g_peermanager.read().await;
    guard.is_some()
}

pub async fn get_stats() -> *mut u8 {
    #[derive(tabled::Tabled, serde::Serialize)]
    struct PeerTableItem {
        #[tabled(rename = "ipv4")]
        cidr: String,
        #[tabled(skip)]
        ipv4: String,
        hostname: String,
        cost: String,
        lat_ms: String,
        loss_rate: String,
        rx_bytes: String,
        tx_bytes: String,
        tunnel_proto: String,
        nat_type: String,
        id: String,
        version: String,
    }

    impl From<PeerRoutePair> for PeerTableItem {
        fn from(p: PeerRoutePair) -> Self {
            let route = p.route.clone().unwrap_or_default();
            PeerTableItem {
                cidr: route.ipv4_addr.map(|ip| ip.to_string()).unwrap_or_default(),
                ipv4: route
                    .ipv4_addr
                    .map(|ip: crate::proto::common::Ipv4Inet| ip.address.unwrap_or_default())
                    .map(|ip| ip.to_string())
                    .unwrap_or_default(),
                hostname: route.hostname.clone(),
                cost: cost_to_str(route.cost),
                lat_ms: if route.cost == 1 {
                    float_to_str(p.get_latency_ms().unwrap_or(0.0), 3)
                } else {
                    route.path_latency_latency_first().to_string()
                },
                loss_rate: float_to_str(p.get_loss_rate().unwrap_or(0.0), 3),
                rx_bytes: format_size(p.get_rx_bytes().unwrap_or(0), humansize::DECIMAL),
                tx_bytes: format_size(p.get_tx_bytes().unwrap_or(0), humansize::DECIMAL),
                tunnel_proto: p
                    .get_conn_protos()
                    .unwrap_or_default()
                    .join(",")
                    .to_string(),
                nat_type: p.get_udp_nat_type(),
                id: route.peer_id.to_string(),
                version: if route.version.is_empty() {
                    "unknown".to_string()
                } else {
                    route.version.to_string()
                },
            }
        }
    }

    impl From<NodeInfo> for PeerTableItem {
        fn from(p: NodeInfo) -> Self {
            PeerTableItem {
                cidr: p.ipv4_addr.clone(),
                ipv4: Ipv4Inet::from_str(&p.ipv4_addr)
                    .map(|ip| ip.address().to_string())
                    .unwrap_or_default(),
                hostname: p.hostname.clone(),
                cost: "Local".to_string(),
                lat_ms: "-".to_string(),
                loss_rate: "-".to_string(),
                rx_bytes: "-".to_string(),
                tx_bytes: "-".to_string(),
                tunnel_proto: "-".to_string(),
                nat_type: if let Some(info) = p.stun_info {
                    info.udp_nat_type().as_str_name().to_string()
                } else {
                    "Unknown".to_string()
                },
                id: p.peer_id.to_string(),
                version: p.version,
            }
        }
    }

    let guard = g_peermanager.read().await;
    if guard.is_none() {
        return get_buffer();
    }
    let peer_mgr_c = guard.as_ref().unwrap().clone();
    let routes = peer_mgr_c.list_routes().await;
    let pmrs = PeerManagerRpcService::new(peer_mgr_c.clone());
    let peers = PeerManagerRpcService::list_peers(&peer_mgr_c).await;
    let peer_routes = list_peer_route_pair(peers, routes);
    let mut items: Vec<PeerTableItem> = vec![];
    let res = pmrs
        .show_node_info(BaseController::default(), ShowNodeInfoRequest::default())
        .await
        .expect("[]");
    items.push(res.node_info.unwrap().into());
    if !get_token().is_cancelled() {
        for p in peer_routes {
            items.push(p.into());
        }
    }
    let json = print_output(&*items);
    write_json(&*json);
    unsafe { BUF_PTR }
}

fn print_output<T>(items: &[T]) -> String
where
    T: tabled::Tabled + serde::Serialize,
{
    serde_json::to_string(items).unwrap()
}

// 1. rename easytier-core.rs to easytier_core.rs
// 2. add codes: let token = &crate::helper::get_token();  _ = token.cancelled() => { println!("任务被取消"); }
// 3. add codes: return Ok(None);   in  init_logger
// 4. fix the import/package issue
async fn start_run(path: &str) -> u8 {
    let cli = Cli::parse_from(["app", &format!("-c{}", path)]);
    let mut ret_code = 0;
    if let Err(e) = run_main(cli).await {
        eprintln!("error: {:?}", e);
        ret_code = 1;
    }

    ret_code
}
