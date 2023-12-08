use anyhow::Context;
use tracing::info;
use warp::Filter;
use structopt::StructOpt;

mod websocket;
mod webrtc;

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

    let public_webrtc_addr = get_env_var("PUBLIC_WEBRTC_ADDR")?;

    let webrtc_session_endpoint =
        webrtc::handle_webrtc_server(webrtc_addr, public_webrtc_addr)
            .await
            .expect("could not start webrtc server");

    //SessionManager::instance(webrtc_session_endpoint).get_session_endpoint();

    let session_endpoint = warp::any().map(move || webrtc_session_endpoint.clone());

    let health_route = warp::path("health").map(|| format!("Server OK"));

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(session_endpoint)
        .map(|ws: warp::ws::Ws, session_endpoint| {
            ws.on_upgrade(move |socket| websocket::handle_ws_client(socket, session_endpoint))
            //ws.on_upgrade(websocket::handle_ws_client)
        });

    let routes = health_route
        .or(ws)
        .with(warp::cors().allow_any_origin())
        .recover(websocket::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], opt.port)).await;

    Ok(())
}