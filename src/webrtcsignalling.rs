use std::time::Duration;

use anyhow::Result;
use just_webrtc::{types::{ICECandidate, PeerConfiguration, SessionDescription}, DataChannelExt, PeerConnectionBuilder, PeerConnectionExt};
use serde::Serialize;

use crate::{webrtcpeer, ClientConnection};

#[derive(Serialize)]
pub struct SessionTuple{
    description: SessionDescription,
    candidates: Vec<ICECandidate>,
}

const REMOTE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn add_remote_peer(offer: SessionDescription, peerid: String) -> Result<SessionTuple>{
    // create simple remote peer connection from received offer and candidates
    let mut remote_peer_connection = PeerConnectionBuilder::new()
        .set_config(PeerConfiguration{..Default::default()})
        .with_remote_offer(Some(offer)).unwrap()
        .build().await.unwrap();
    // remote_peer_connection.add_ice_candidates(offer.sdp_type).await?;
    let answer = remote_peer_connection.get_local_description().await.unwrap();
    let candidates = remote_peer_connection.collect_ice_candidates().await?;
    // println!("Incoming: {:?}\n\tMy Response: {:?}\n\tCandidates: {:?}", remote_peer_connection, answer.sdp, candidates);

    // For the remote to connect to.
    tokio::spawn(async move{
        println!("Signalling to {:?}", peerid);
        let is_connected = tokio::time::timeout(REMOTE_CONNECTION_TIMEOUT,
            remote_peer_connection.wait_peer_connected()).await.is_ok();

        if !is_connected {
            println!("Giving up on {:?}", peerid);
            return;
        }

        let remote_channel = remote_peer_connection.receive_channel().await.unwrap();
        remote_channel.wait_ready().await;
        println!("WebRTC established with {:?}", peerid);

        let conn = ClientConnection::new(remote_peer_connection, remote_channel);
        webrtcpeer::manage_connection(conn);
    });
    return Ok(SessionTuple{description: answer, candidates});
}