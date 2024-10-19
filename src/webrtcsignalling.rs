use std::time::Duration;

use just_webrtc::{platform::{Channel, PeerConnection}, types::{ICECandidate, PeerConfiguration, PeerConnectionState, SessionDescription}, DataChannelExt, PeerConnectionBuilder, PeerConnectionExt};
use log::info;
use serde::Serialize;

use crate::{webrtcpeer, ClientConnection};

// The lifetime of a connection accept response.
// The amount of time for web client to establish a webrtc connection with us, after using `/connect`
const REMOTE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize)]
pub struct SessionTuple{
    description: SessionDescription,
    candidates: Vec<ICECandidate>,
}

/// Attempts to create a WebRTC answer for the given inputs. If the inputs are malformed, you'll get an error back.
pub async fn create_answer(offer: SessionDescription, connectionsource: String) -> Result<SessionTuple, ()>{
    let Ok(remote_peer_connection) = PeerConnectionBuilder::new()
        .set_config(PeerConfiguration{..Default::default()})
        .with_remote_offer(Some(offer)).map_err(|_|())?
        .build().await else { return Err(()) };
    // remote_peer_connection.add_ice_candidates(offer.sdp_type).await?;
    let Some(answer) = remote_peer_connection.get_local_description().await else{ return Err(()) };
    let candidates = remote_peer_connection.collect_ice_candidates().await.unwrap_or_default();
    // info!("Incoming: {:?}\n\tMy Response: {:?}\n\tCandidates: {:?}", remote_peer_connection, answer.sdp, candidates);

    info!("Hosting offer for {:?}", connectionsource);
    tokio::spawn(async move{
        if let Ok(conn) = await_connection(remote_peer_connection).await {
            info!("WebRTC established with {:?}", connectionsource);
            webrtcpeer::manage_connection(conn).await;
        }else{
            info!("Gave up on offer for {:?}", connectionsource);
        }
    });
    return Ok(SessionTuple{description: answer, candidates});
}

async fn await_connection(peer: PeerConnection)->Result<ClientConnection, ()>{
    let Ok(_) = tokio::time::timeout(REMOTE_CONNECTION_TIMEOUT, wait_is_connected(&peer)).await else {
        return Err(());
    };

    // Receive ro and uu data channels.
    let mut ro: Option<Channel> = None;
    let mut uu: Option<Channel> = None;
    loop{
        let Ok(remote_channel) = peer.receive_channel().await else {return Err(())};
        remote_channel.wait_ready().await;
        match remote_channel.label().as_str(){
            "ro" => ro = Some(remote_channel),
            "uu" => uu = Some(remote_channel),
            _ => return Err(()) // Unexpected channel
        }
        if ro.is_some() && uu.is_some() { break; }
    }

    let conn = ClientConnection::new(peer, ro.unwrap(), uu.unwrap());
    return Ok(conn);
}

/// Must be used in conjunction with a timeout
async fn wait_is_connected(peer: &PeerConnection) -> Result<(),()>{
    use PeerConnectionState::*;
    loop{ match peer.state_change().await {
        Failed | Closed => return Err(()),
        Connected => return Ok(()),
        _ => {}
    }}
}