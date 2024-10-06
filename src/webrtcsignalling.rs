use std::time::Duration;

use just_webrtc::{platform::PeerConnection, types::{ICECandidate, PeerConfiguration, PeerConnectionState, SessionDescription}, DataChannelExt, PeerConnectionBuilder, PeerConnectionExt};
use log::info;
use serde::Serialize;

use crate::{webrtcpeer, ClientConnection};

#[derive(Serialize)]
pub struct SessionTuple{
    description: SessionDescription,
    candidates: Vec<ICECandidate>,
}

const REMOTE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn create_offer(offer: SessionDescription, connectionsource: String) -> Result<SessionTuple, ()>{
    let Ok(remote_peer_connection) = PeerConnectionBuilder::new()
        .set_config(PeerConfiguration{..Default::default()})
        .with_remote_offer(Some(offer)).map_err(|_|())?
        .build().await else{ return Err(()) };
    // remote_peer_connection.add_ice_candidates(offer.sdp_type).await?;
    let Some(answer) = remote_peer_connection.get_local_description().await else{ return Err(()) };
    let candidates = remote_peer_connection.collect_ice_candidates().await.unwrap_or_default();
    // info!("Incoming: {:?}\n\tMy Response: {:?}\n\tCandidates: {:?}", remote_peer_connection, answer.sdp, candidates);

    info!("Hosting offer for {:?}", connectionsource);
    tokio::spawn(async move{
        if let Ok(conn) = await_connection(remote_peer_connection, &connectionsource).await {
            info!("WebRTC established with {:?}", connectionsource);
            webrtcpeer::manage_connection(conn).await;
        }
    });
    return Ok(SessionTuple{description: answer, candidates});
}

// Must be used with a timeout
async fn wait_is_connected(peer: &PeerConnection) -> Result<(),()>{
    use PeerConnectionState::*;
    loop{ match peer.state_change().await {
        Failed => return Err(()),
        Connected => return Ok(()),
        _ => {}
    }}
}

pub async fn await_connection(peer: PeerConnection, connectionsource: &str)->Result<ClientConnection, ()>{
    let Ok(_) = tokio::time::timeout(REMOTE_CONNECTION_TIMEOUT, wait_is_connected(&peer)).await else {
        info!("Gave up on offer for {:?}", connectionsource);
        return Err(());
    };

    let remote_channel = peer.receive_channel().await.unwrap();
    remote_channel.wait_ready().await;

    let conn = ClientConnection::new(peer, remote_channel);
    return Ok(conn);
}