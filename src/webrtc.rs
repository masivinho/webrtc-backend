use anyhow::Context;
use tracing::{info, warn};
use webrtc_unreliable::{Server as RtcServer, SessionEndpoint};
use datachannel::{DataChannelHandler, DataChannelInfo, PeerConnectionHandler, RtcConfig, RtcPeerConnection};

pub async fn handle_webrtc_server(
    webrtc_addr: std::net::SocketAddr,
    location_description: String) -> anyhow::Result<SessionEndpoint> {

    let webrtc_listen_addr = std::net::SocketAddr::new(
        std::net::Ipv4Addr::UNSPECIFIED.into(),
        webrtc_addr.port(),
    );

    let mut rtc_server = RtcServer::new(webrtc_listen_addr, webrtc_addr)
        .await
        .context("could not start RTC server")?;

    let session_endpoint = rtc_server.session_endpoint();

    tokio::spawn(async move {
        let mut buf = Vec::new();
        loop {
            let (message_type, remote_addr) = match rtc_server.recv().await {
                Ok(received) => {
                    buf.clear();
                    buf.extend(received.message.as_slice());
                    (received.message_type, received.remote_addr)
                }
                Err(err) => {
                    warn!("could not receive RTC message: {}", err);
                    continue;
                }
            };
            let data: &str = match std::str::from_utf8(&buf) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let mut last_line = "";
            let mut send_location = false;
            for line in data.split('\n') {
                info!("line {}", line);
                if line.starts_with("LOC?") {
                    send_location = true;
                } else if let Some(ping_num_str) = line.strip_prefix("NUM:\t") {
                    let ping_num: u64 = ping_num_str.parse().unwrap_or(0);
                    info!("ping_num {}", ping_num);
                }
                last_line = line;
            }
            let mut to_send = last_line;
            let storage_when_sending_location: String;
            if send_location {
                storage_when_sending_location =
                    format!("LOC:\t{}\n{}", &location_description, last_line);
                to_send = &storage_when_sending_location;
            }
            let send_result = rtc_server
                .send(to_send.as_bytes(), message_type, &remote_addr)
                .await;
            info!("result {}, remote {}, line {}", to_send, remote_addr, last_line);
            if let Err(err) = send_result {
                warn!("could not send RTC message: {}", err)
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