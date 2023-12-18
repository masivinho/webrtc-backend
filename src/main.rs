use anyhow::Context;
use tracing::info;
use warp::Filter;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::RwLock;
use serde::Serialize;

mod websocket;
mod webrtc;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Pings = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Debug, Clone, Serialize)]
pub struct Client {
    pub id: usize,
    pub pings: Vec<i64>
}

#[derive(Debug, Clone, Serialize)]
pub struct Ping {
    pub index: i64,
    pub time: i64,
}

fn get_env_var(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("env variable '{}' not set", name))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    info!(concat!("Repo git hash: ", env!("GIT_HASH")));

    let websocket_port: u16 = get_env_var("WEBSOCKET_PORT")?
        .parse()
        .context("failed to parse WEBSOCKET_PORT")?;

    let webrtc_addr: std::net::SocketAddr = get_env_var("WEBRTC_ADDR")?
        .parse()
        .context("failed to parse WEBRTC_ADDR")?;

    let public_addr: std::net::SocketAddr = get_env_var("PUBLIC_WEBRTC_ADDR")?
        .parse()
        .context("failed to parse PUBLIC_WEBRTC_ADDR")?;

    info!("Env: WEBRTC_ADDR => {:?}", webrtc_addr);
    info!("Env: PUBLIC_WEBRTC_ADDR => {:?}", public_addr);

    let pings: Pings = Arc::new(RwLock::new(HashMap::new()));

    let webrtc_session_endpoint =
        webrtc::handle_webrtc_server(webrtc_addr, public_addr, pings.clone())
            .await
            .expect("could not start webrtc server");

    let session_endpoint = warp::any().map(move || webrtc_session_endpoint.clone());

    let health_route = warp::path("health").map(|| format!("Server OK"));

    let w_pings = with_pings(pings.clone());

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(session_endpoint)
        .and(w_pings)
        .map(|ws: warp::ws::Ws, session_endpoint, w_pings| {
            ws.on_upgrade(move |socket| websocket::handle_ws_client(socket, session_endpoint, w_pings))
        });

    let routes = health_route
        .or(ws)
        .with(warp::cors().allow_any_origin())
        .recover(websocket::handle_rejection);

    warp::serve(routes).run(([0, 0, 0, 0], websocket_port)).await;

    Ok(())
}

fn with_pings(pings: Pings) -> impl Filter<Extract = (Pings,), Error = Infallible> + Clone {
    warp::any().map(move || pings.clone())
}