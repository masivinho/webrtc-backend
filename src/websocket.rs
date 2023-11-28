use std::convert::Infallible;
use tracing::{info, error};
use serde::{Deserialize, Serialize};
use warp::http::StatusCode;

#[derive(Serialize, Debug)]
struct WsResult {
    status: String,
    response: String,
}

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

pub async fn handle_ws_client(websocket: warp::ws::WebSocket) {
    info!("client disconnected");
}