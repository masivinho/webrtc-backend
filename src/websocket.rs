use std::convert::Infallible;
use tracing::{info, error};
use serde::Serialize;
use warp::http::StatusCode;
use futures::{SinkExt, StreamExt};
use warp::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use serde_json::{json, Value};
use crate::webrtc::handle_rtc_session;

#[derive(Serialize, Debug)]
struct ErrorResult {
    detail: String,
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

pub async fn handle_ws_client(websocket: warp::ws::WebSocket, session_endpoint: webrtc_unreliable::SessionEndpoint) {

    info!("client connected");
    
    let (mut sender, mut receiver) = websocket.split();

    while let Some(body) = receiver.next().await {
        let message = match body {
            Ok(msg) => msg,
            Err(e) => {
                error!("error reading message on websocket: {}", e);
                break;
            }
        };

        handle_ws_message(message, &mut sender, session_endpoint.clone()).await;
    }

    info!("client disconnected");
}

async fn handle_ws_message(msg: Message, sender: &mut SplitSink<WebSocket, Message>, session_endpoint: webrtc_unreliable::SessionEndpoint) {
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let signal: Value = serde_json::from_str(msg).unwrap();
    info!("type {}", signal["type"]);

    if signal["type"] == "offer" {
        let session_answer = handle_rtc_session(session_endpoint, signal["sdp"].as_str().unwrap()).await.unwrap();
        info!(session_answer);
        let answer_message: Value = serde_json::from_str(String::from(session_answer).as_str()).unwrap();
        sender.send(Message::text(answer_message["answer"].to_string())).await.unwrap();
    } else if signal["type"] == "ice" {
        handle_datachannel(signal);
    }

}

async fn handle_datachannel(signal: Value) {

}