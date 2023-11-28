use anyhow::Context;
use tracing::info;
use warp::Filter;
use structopt::StructOpt;

mod websocket;

#[derive(StructOpt, Debug)]
#[structopt(name = "webrtc-backend")]
struct Opts {
    #[structopt(short, long, default_value = "8000")]
    port: u16,
}

fn get_env_var(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("env variable '{}' not set", name))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let opt = Opts::from_args();

    info!(concat!("Repo git hash: ", env!("GIT_HASH")));

    let webrtc_addr: std::net::SocketAddr = get_env_var("WEBRTC_ADDR")?
        .parse()
        .context("failed to parse WEBRTC_ADDR")?;

    info!("Env: WEBRTC_ADDR => {:?}", webrtc_addr);

    let health_route = warp::path("health").map(|| format!("Server OK"));

    let ws = warp::path("ws").and(warp::ws()).map(|ws: warp::ws::Ws| {
        ws.on_upgrade(websocket::handle_ws_client)
    });

    let routes = health_route
        .or(ws)
        .with(warp::cors().allow_any_origin())
        .recover(websocket::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], opt.port)).await;

    Ok(())
}