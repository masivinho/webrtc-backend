use std::convert::Infallible;
use tracing::{info, error, warn};
use serde::Serialize;
use warp::http::StatusCode;
use futures::{SinkExt, StreamExt};
use warp::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use serde_json::Value;
use crate::{webrtc::handle_rtc_session, Pings, Client, NEXT_USER_ID};
use std::sync::atomic::Ordering;

#[derive(Serialize, Debug)]
struct ErrorResult {
    detail: String,
}

#[derive(Serialize)]
struct PingResult {
    #[serde(rename = "type")]
    result_type: String,
    list: Vec<i64>,
    #[serde(rename = "receivedCount")]
    received_count: usize
}

pub async fn handle_rejection(err: warp::reject::Rejection) -> std::result::Result<impl warp::reply::Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found";
    } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        message = "Invalid Body";
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&ErrorResult { detail: message.into() });
    Ok(warp::reply::with_status(json, code))
}

pub async fn handle_ws_client(
    websocket: warp::ws::WebSocket,
    session_endpoint: webrtc_unreliable::SessionEndpoint,
    pings: Pings
) {

    info!("client connected");

    let client_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    
    let (mut sender, mut receiver) = websocket.split();

    while let Some(body) = receiver.next().await {
        let message = match body {
            Ok(msg) => msg,
            Err(e) => {
                error!("error reading message on websocket: {}", e);
                break;
            }
        };

        handle_ws_message(message, &mut sender, client_id, session_endpoint.clone(), pings.clone()).await;
    }

    let port = get_client_port(client_id, pings.clone()).await;
    hadle_client_remove(port, pings.clone()).await;

    info!("client disconnected");
}

async fn handle_ws_message(
    msg: Message,
    sender: &mut SplitSink<WebSocket, Message>,
    client_id: usize,
    session_endpoint: webrtc_unreliable::SessionEndpoint,
    pings: Pings
) {
        
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let signal: Value = match serde_json::from_str(msg){
        Result::Ok(val) => {val},
        Result::Err(_) => {
            serde_json::Value::Null
        }
    };

    if signal["type"] == "offer" {
        let session = handle_rtc_session(session_endpoint, signal["sdp"].as_str().unwrap()).await.unwrap();
        info!(session);
        sender.send(Message::text(String::from(session).as_str())).await.unwrap();

    } else if signal["type"] == "ice" {
        info!("{}", signal);
        
        let candidate: String = signal["ice"]["candidate"].to_string();
        let port = candidate.split(" ").nth(5).unwrap();

        handle_client_register(client_id, port.to_string(), pings).await;

    } else if signal["type"] == "done" {

        let pings_read = pings.read().await;
        let selected_ports: Vec<_> = pings_read.values().filter(|client| client.id == client_id).collect();

        if selected_ports.len() < 1 {
            return;
        }

        let client: Client = selected_ports[0].clone();

        let ping_result = PingResult {
            result_type: "results".to_string(),
            list: client.pings.clone(),
            received_count: client.pings.len()
        };

        sender.send(Message::text(serde_json::to_string(&ping_result).unwrap())).await.unwrap();

        if let Err(err) = sender.close().await {
            warn!("could not disconnect user: {:?}", err);
        }
    }
}

pub async fn handle_client_register(client_id: usize, port: String, pings: Pings) {
    pings.write().await.insert(
        port,
        Client {
            id: client_id,
            pings: vec![]
        }
    );
}

pub async fn get_client_port(client_id: usize, pings: Pings) -> String {
    let pings_read = pings.read().await;
    let selected_ports: Vec<_> = pings_read.iter().filter(|(_key, client)| client.id == client_id).map(|(key, value)| {
        return (key.clone(), value.clone());
    }).collect();

    if selected_ports.len() < 1 {
        return "not found".to_string()
    }

    let (key, _value) = selected_ports[0].clone(); {
        return key
    };
}

pub async fn hadle_client_remove(port: String, pings: Pings) {
    pings.write().await.remove(&port);
}