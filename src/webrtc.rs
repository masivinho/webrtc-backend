use anyhow::Context;
use tracing::{info, warn};
use webrtc_unreliable::{Server as RtcServer, SessionEndpoint};

pub async fn handle_webrtc_server(webrtc_addr: std::net::SocketAddr) -> anyhow::Result<SessionEndpoint> {

    let webrtc_listen_addr = std::net::SocketAddr::new(
        std::net::Ipv4Addr::UNSPECIFIED.into(),
        webrtc_addr.port(),
    );

    let mut rtc_server = RtcServer::new(webrtc_listen_addr, webrtc_addr)
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

            info!("{:?} | data: {}", received.unwrap(), data);

            if let Some((message_type, remote_addr)) = received {
                if let Err(err) = rtc_server
                    .send(&message_buf, message_type, &remote_addr)
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