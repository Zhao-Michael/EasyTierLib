use crate::core::{Cli, run_main};
use crate::proto::api::instance::{
    ListPeerRequest, ListRouteRequest, NodeInfo, PeerManageRpc, PeerManageRpcClientFactory,
    ShowNodeInfoRequest, list_peer_route_pair,
};
use crate::proto::rpc::standalone::runtime_rpc_client;
use crate::proto::rpc_types::controller::BaseController;
use crate::utils::string::cost_to_str;
use crate::utils::PeerRoutePair;
use cidr::Ipv4Inet;
use clap::Parser;
use humansize::format_size;
use lazy_static::lazy_static;
use std::alloc::{Layout, alloc_zeroed};
use std::ptr;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;

lazy_static! {
    static ref g_token: std::sync::RwLock<CancellationToken> =
        std::sync::RwLock::new(CancellationToken::new());
}

lazy_static! {
    static ref g_isrunning: std::sync::RwLock<bool> = std::sync::RwLock::new(false);
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

pub fn write_json(s: &str) {
    let ptr = get_buffer();
    unsafe {
        ptr::write_bytes(ptr, 0, SIZE);
    }
    let bytes = s.as_bytes();
    let len = bytes.len().min(SIZE);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
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
    set_running_state(true);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(start_run(path));
    set_running_state(false);
}

#[allow(static_mut_refs)]
pub(crate) fn is_running() -> bool {
    g_isrunning.read().unwrap().clone()
}

pub(crate) fn set_running_state(running: bool) {
    let mut guard = g_isrunning.write().unwrap();
    *guard = running;
}

fn float_to_str(f: f64, precision: usize) -> String {
    format!("{:.*}", precision, f)
}

#[derive(serde::Serialize)]
struct PeerTableItem {
    cidr: String,
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

/// Query the local management RPC portal for the peer list.
///
/// The RPC portal binds to a free port in 15888..15900 when started without an
/// explicit `--rpc-portal`, so probe the whole range.
async fn collect_peer_items() -> anyhow::Result<Vec<PeerTableItem>> {
    for port in 15888..15900 {
        let mut client =
            runtime_rpc_client(format!("tcp://127.0.0.1:{}", port).parse().unwrap());
        let Ok(pm) = client
            .scoped_client::<PeerManageRpcClientFactory<BaseController>>("".to_string())
            .await
        else {
            continue;
        };

        let peers = pm
            .list_peer(
                BaseController::default(),
                ListPeerRequest { instance: None },
            )
            .await?;
        let routes = pm
            .list_route(
                BaseController::default(),
                ListRouteRequest { instance: None },
            )
            .await?;
        let node = pm
            .show_node_info(
                BaseController::default(),
                ShowNodeInfoRequest { instance: None },
            )
            .await?;

        let peer_routes = list_peer_route_pair(peers.peer_infos, routes.routes);
        let mut items: Vec<PeerTableItem> = Vec::new();
        if let Some(node_info) = node.node_info {
            items.push(node_info.into());
        }
        for p in peer_routes {
            items.push(p.into());
        }
        return Ok(items);
    }
    Err(anyhow::anyhow!(
        "no management rpc portal found in 127.0.0.1:15888..15900"
    ))
}

async fn collect_peer_json() -> *mut u8 {
    let items = collect_peer_items().await.unwrap_or_default();
    let json = serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string());
    write_json(&json);
    unsafe { BUF_PTR }
}

/// Collect peer list json and store it in the shared buffer.
pub async fn get_stats() -> *mut u8 {
    collect_peer_json().await
}

/// Collect peer list json (via local rpc portal) and store it in the shared
/// buffer. Kept as a separate entry point for the legacy `rstatus` export.
pub fn remote_status() -> *mut u8 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(collect_peer_json())
}

async fn start_run(path: &str) -> u8 {
    let cli = Cli::parse_from(["app".to_string(), format!("-c{}", path)]);
    let mut ret_code = 0;
    if let Err(e) = run_main(cli).await {
        eprintln!("error: {:?}", e);
        ret_code = 1;
    }

    ret_code
}
