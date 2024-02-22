use anyhow::Context;
use tracing::{info, warn};
use webrtc_unreliable::{Server as RtcServer, SessionEndpoint};
use serde_json::Value;

use crate::{Pings, Ping};

pub async fn handle_webrtc_server(webrtc_addr: std::net::SocketAddr, public_addr: std::net::SocketAddr, pings: Pings) -> anyhow::Result<SessionEndpoint> {

    let mut rtc_server = RtcServer::new(webrtc_addr, public_addr)
        .await
        .context("could not start RTC server")?;

    let session_endpoint = rtc_server.session_endpoint();

    tokio::spawn(async move {
        let mut message_buf = Vec::new();
        loop {
            let received = match rtc_server.recv().await {
                Ok(received) => {
                    message_buf.clear();
                    message_buf.extend(received.message.as_ref());
                    Some((received.message_type, received.remote_addr))
                }
                Err(err) => {
                    warn!("could not receive RTC message: {:?}", err);
                    None
                }
            };

            let data: &str = match std::str::from_utf8(&message_buf) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let ping_data: Value = serde_json::from_str(data).unwrap();

            if let Some((message_type, remote_addr)) = received {

                let ping = Ping {
                    index: ping_data["i"].as_number().unwrap().as_i64().unwrap(),
                    time: ping_data["time"].as_u64().unwrap(),
                };

                handle_ping(
                    remote_addr.to_string(),
                    ping_data,
                    pings.clone()
                ).await;

                if let Err(err) = rtc_server
                    .send(serde_json::to_string(&ping).unwrap().as_bytes(), message_type, &remote_addr)
                    .await
                {
                    warn!("could not send message to {}: {:?}", remote_addr, err);
                }
            }
        }
    });

    Ok(session_endpoint)
}

pub async fn handle_rtc_session(rtc_session_endpoint: SessionEndpoint, data: &str) -> Result<String, String> {

    info!(data);

    Ok(rtc_session_endpoint
        .clone()
        .session_request(futures::stream::once(futures::future::ok::<
            Vec<u8>,
            std::io::Error,
        >(data.as_bytes().to_vec())))
        .await
        .map_err(|e| e.to_string())?)
}

pub async fn handle_ping(address: String, data: Value, pings: Pings) {

    let port = address.split(":").nth(1).unwrap().to_string();

    let ping = pings.read().await.get(&port).cloned();
    match ping {
        Some(_) => {
            handle_ping_push(port, data, pings).await;
        },
        None => { },
    }

}

pub async fn handle_ping_push(port: String, data: Value, pings: Pings) {
    let mut pings_write = pings.write().await;
    if let Some(client) = pings_write.get_mut(&port) {
        client.pings.push(data["i"].as_number().unwrap().as_i64().unwrap());
    }
}