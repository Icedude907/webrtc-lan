use std::{sync::{Mutex, OnceLock}, time::Duration};

use anyhow::Result;
use just_webrtc::{platform::PeerConnection, types::{ICECandidate, PeerConfiguration, SessionDescription}, DataChannelExt, PeerConnectionBuilder, PeerConnectionExt};
use lazy_static::lazy_static;
use serde::Serialize;

use crate::{util::UUIDGen, webrtcpeer, ClientConnection};

#[derive(Serialize)]
pub struct SessionTuple{
    description: SessionDescription,
    candidates: Vec<ICECandidate>,
}

const REMOTE_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn create_offer(offer: SessionDescription, connectionsource: String) -> Result<SessionTuple, ()>{
    let Ok(mut remote_peer_connection) = PeerConnectionBuilder::new()
        .set_config(PeerConfiguration{..Default::default()})
        .with_remote_offer(Some(offer)).map_err(|_|())?
        .build().await else{ return Err(()) };
    // remote_peer_connection.add_ice_candidates(offer.sdp_type).await?;
    let Some(answer) = remote_peer_connection.get_local_description().await else{ return Err(()) };
    let candidates = remote_peer_connection.collect_ice_candidates().await.unwrap_or_default();
    // println!("Incoming: {:?}\n\tMy Response: {:?}\n\tCandidates: {:?}", remote_peer_connection, answer.sdp, candidates);

    println!("Hosting offer for {:?}", connectionsource);
    tokio::spawn(async move{
        if let Ok(conn) = await_connection(remote_peer_connection, &connectionsource).await {
            println!("WebRTC established with {:?} as {:016x}", connectionsource, conn.get_id());
            webrtcpeer::manage_connection(conn);
        }
    });
    return Ok(SessionTuple{description: answer, candidates});
}

pub async fn await_connection(mut peer: PeerConnection, connectionsource: &str)->Result<ClientConnection, ()>{
    lazy_static!{
        static ref ID_GEN: Mutex<UUIDGen> = UUIDGen::new().into();
    }

    let Ok(_) = tokio::time::timeout(REMOTE_CONNECTION_TIMEOUT, peer.wait_peer_connected()).await else {
        println!("Gave up on offer for {:?}", connectionsource);
        return Err(());
    };

    let remote_channel = peer.receive_channel().await.unwrap();
    remote_channel.wait_ready().await;

    let connection_id = ID_GEN.lock().unwrap().next(); // This is sound. Lock is freed after expression, and async can't interrupt so async secure. Thread safety is guaranteed by mutex.

    let conn = ClientConnection::new(peer, remote_channel, connection_id);
    return Ok(conn);
}