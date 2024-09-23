mod webserver;
mod webrtcsignalling;
mod webrtcpeer;
mod chatsession;
mod util;

use log::{info, LevelFilter};
use tokio::join;
use webrtcpeer::ClientConnection;
use webserver::webserver_run;

pub const WEBSERVER_PORT: u16 = 3000;

#[tokio::main]
async fn main(){
    colog::basic_builder().filter_level(LevelFilter::Info).init();

    info!("Initialising");
    let _ = join!(
        webserver_run(WEBSERVER_PORT),
        manage_remotes(),
    );
}

pub async fn manage_remotes(){

}