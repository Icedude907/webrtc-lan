use std::net::{Ipv4Addr, SocketAddr};

use axum::{extract::ConnectInfo, http::{header, Uri}, response::{IntoResponse, Redirect, Response}, routing::{get, post}, Json, Router};
use just_webrtc::types::SessionDescription;
use log::info;
use serde_json::{json, Value};

use crate::webrtcsignalling;

const WEBSERVER_HOST: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[derive(rust_embed::Embed)]
#[folder = "webclient/dist/"]
struct Assets;

pub async fn webserver_run(port: u16) {
    // Serve the web folder with the client in it
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/connect", post(respond_to_webrtc_offer)) // Also the signalling subsystem
        .fallback_service(get(serve_static))
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

async fn serve_index() -> impl IntoResponse{
    StaticFile("index.html")
}

async fn serve_static(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    StaticFile(path)
}

pub struct StaticFile<T>(pub T);
impl<T> IntoResponse for StaticFile<T> where T: Into<String>{
  fn into_response(self) -> Response {
    let path = self.0.into();

    match Assets::get(path.as_str()) {
      Some(content) => {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
      }
      None => {
        // Not found -> redirect
        Redirect::to("/").into_response()
      }
    }
  }
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