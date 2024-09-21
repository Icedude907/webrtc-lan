mod webserver;
mod webrtcsignalling;
mod webrtcpeer;

use just_webrtc::platform::{Channel, PeerConnection};
use tokio::join;
use webserver::webserver_run;

pub const WEBSERVER_PORT: u16 = 3000;

#[tokio::main]
async fn main(){
    println!("Running");
    let _ = join!(
        webserver_run(WEBSERVER_PORT),
        manage_remotes(),
    );
}

struct RelayServer{
    clients: Vec<ClientConnection>
}

/// Abstracts the WebRTC peer under a pseudo-"protocol" of unordered+unreliable or ordered+reliable streams
/// The reliable streams are for status and game graphics
/// The unreliable stream is for the client's inputs with the server
struct ClientConnection{
    as_peer: PeerConnection,
    as_channel: Channel,
}

pub async fn manage_remotes(){
    // if let Some(peer_connection) = wait_for_connection().await {
    //     // Move the peer_connection out of the global state into the local state
    //     // Implement your logic here
    // }
    let server = RelayServer{ clients: vec![] };
    for c in server.clients{
        // c.as_channel.
    }
}