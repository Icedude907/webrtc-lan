use std::net::{Ipv4Addr, SocketAddr};

use axum::{extract::ConnectInfo, http::{header, HeaderMap, Uri}, response::{IntoResponse, Redirect, Response}, routing::{get, post}, Json, Router};
use just_webrtc::types::SessionDescription;
use log::info;
use rust_embed_for_web::EmbedableFile;
use serde_json::{json, Value};
use tokio::net::TcpListener;

use crate::webrtcsignalling;

const WEBSERVER_HOST: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

pub async fn webserver_run(port: u16) {
    // Serve the web folder with the client in it
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/connect", post(respond_to_webrtc_offer)) // Also the signalling subsystem
        .fallback_service(get(serve_static))
        .into_make_service_with_connect_info::<SocketAddr>();

    let socket = SocketAddr::from((WEBSERVER_HOST, port));
    let listener = TcpListener::bind(socket).await.unwrap(); // Failed to bind is a fatal error
    let server = axum::serve(listener, app);
    let localip = local_ip_address::local_ip().map(|x| x.to_string()).unwrap_or("?".into());
    info!("Webserver listening at {} (localhost: http://127.0.0.1:{port}/, LAN: http://{}:{port}/)", socket, localip);

    async fn shutdown_detector(){ tokio::signal::ctrl_c().await.unwrap() } // Infallible
    server
        .with_graceful_shutdown(shutdown_detector())
        .await.unwrap(); // Run it. Unexpected failure is fatal
}

async fn serve_index(headers: HeaderMap) -> impl IntoResponse{
    StaticFile("index.html", headers)
}

async fn serve_static(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').to_string();
    StaticFile(path, headers)
}

// TODO: Embed compressed and extract for transmission?
#[derive(rust_embed_for_web::RustEmbed)]
#[folder = "webclient/dist/"]
#[gzip = false] // gzip plebs use raw instead
struct StaticFiles;

pub struct StaticFile<T>(pub T, HeaderMap);
impl<T> IntoResponse for StaticFile<T> where T: Into<String>{
  fn into_response(self) -> Response {
    let path = self.0.into();
    let file = StaticFiles::get(path.as_str());

    let requested_br = self.1.get(header::ACCEPT_ENCODING).map(|x| x.to_str().unwrap_or("").contains("br")).unwrap_or(false);

    match file {
      Some(content) => {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        let mut using_br = false;
        let data = if requested_br {
          if let Some(x) = content.data_br() { using_br = true; x} else { content.data() }
        } else { content.data() };

        let mut headers = HeaderMap::new();
        if let Ok(x) = mime.to_string().parse() {
          headers.insert(header::CONTENT_TYPE, x);
        }
        if using_br {
          headers.insert(header::CONTENT_ENCODING, "br".parse().unwrap());
        }

        (headers, data).into_response()
      }
      None => {
        // Not found -> redirect
        StaticFile("404.html", self.1).into_response()
      }
    }
  }
}

async fn respond_to_webrtc_offer(ConnectInfo(addr): ConnectInfo<SocketAddr>, payload: Option<Json<SessionDescription>>)->Json<Value>{
    if let Some(params) = payload {
        let id = format!("{}", addr);
        let x = webrtcsignalling::create_answer(params.0, id).await;
        return match x{
          Ok(x) => Json(json!(x)),
          Err(_) => Json(json!({"Malformed":"The provided WebRTC offer is unusable."})),
        };
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