mod webserver;
mod webrtcsignalling;
mod webrtcpeer;
mod chatroom;
mod packets;
mod usersession;
mod util;

use log::{info, LevelFilter};
use tokio::join;
use webrtcpeer::ClientConnection;
use webserver::webserver_run;

pub const WEBSERVER_PORT: u16 = 3000;

#[tokio::main]
async fn main(){
    colog::basic_builder()
        .filter_level(LevelFilter::Info)
        // .default_format()
        // .format_timestamp(None)
        // .format_module_path(true)
        .filter_module("webrtc_ice", LevelFilter::Error)
        .init();

    info!("Initialising");
    let _ = join!(
        webserver_run(WEBSERVER_PORT),
        cli(),
    );
}

pub async fn cli(){

}