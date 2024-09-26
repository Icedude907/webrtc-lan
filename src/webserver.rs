use std::net::{Ipv4Addr, SocketAddr};

use axum::{extract::ConnectInfo, middleware::map_response, response::Response, routing::{self, post}, Json, Router};
use just_webrtc::types::SessionDescription;
use log::info;
use serde_json::{json, Value};
use tower_http::services::{ServeDir, ServeFile};

use crate::webrtcsignalling;

const WEBSERVER_HOST: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

pub async fn webserver_run(port: u16) {
    // Serve the web folder with the game wasm in it

    let serve_dir = ServeDir::new("web").not_found_service(ServeFile::new("web/index.html"));
    let serve_nocache = routing::get_service(serve_dir).layer(map_response(disable_browser_cache));

    let app = Router::new()
        .route("/connect", post(respond_to_webrtc_offer)) // Also the signalling subsystem
        .fallback_service(serve_nocache)
        .into_make_service_with_connect_info::<SocketAddr>();

    let socket = SocketAddr::from((WEBSERVER_HOST, port));
    let listener = tokio::net::TcpListener::bind(socket).await.unwrap();
    let server = axum::serve(listener, app);
    info!("Webserver listening at {} (access: http://127.0.0.1:{}/)", socket, socket.port());

    async fn shutdown_detector(){ tokio::signal::ctrl_c().await.unwrap() }
    server
        .with_graceful_shutdown(shutdown_detector())
        .await.unwrap(); // Run it
}

async fn respond_to_webrtc_offer(ConnectInfo(addr): ConnectInfo<SocketAddr>, payload: Option<Json<SessionDescription>>)->Json<Value>{
    if let Some(params) = payload {
        let id = format!("{}", addr);
        let x = webrtcsignalling::create_offer(params.0, id).await.unwrap();
        return Json(json!(x));
    }else{
        return Json(json!({"Malformed":"LOL"}));
    }
}

async fn disable_browser_cache<R>(mut r: Response<R>) -> Response<R>{
    let headers = r.headers_mut();
    headers.insert(
        "Cache-Control",
        "no-cache, no-store, must-revalidate".parse().unwrap(),
    );
    headers.insert("Pragma", "no-cache".parse().unwrap());
    headers.insert("Expires", "0".parse().unwrap());
    r
}