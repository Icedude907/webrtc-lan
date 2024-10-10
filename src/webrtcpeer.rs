use bytes::Bytes;
use derive_new::new;
use just_webrtc::{platform::{Channel, PeerConnection}, types::PeerConnectionState, DataChannelExt, PeerConnectionExt};
use just_webrtc::platform::Error as WebRTCError;
use log::info;
use tokio::signal::ctrl_c;
use packets::{PktC2S, PktS2C_HelloReply};

use crate::{packets::{self, Encode}, usersession::ActiveSession};

/// Abstracts the WebRTC peer under a pseudo-"protocol" of unordered+unreliable or ordered+reliable streams
/// The reliable streams are for status and data transfer
/// The unreliable stream is for rapidly changing info e.g.: keypresses, rng seeds
#[derive(new)]
pub struct ClientConnection{
    peer: PeerConnection,
    chan: Channel,
}

pub enum RecvError{
    Abort,
    WebRTCError(WebRTCError),
}

impl ClientConnection{
    pub async fn recv(&self)->Result<Bytes, RecvError>{
        use RecvError::*;
        tokio::select!{
            x = self.chan.receive() => { return x.map_err(|e|WebRTCError(e)); },
            _ = ctrl_c() => { return Err(Abort); }
        }
    }
    // usize = bytes sent
    pub async fn send(&self, data: impl Into<Bytes>)->Result<usize, WebRTCError>{
        let data = data.into();
        info!("{} << {:?}", "Out", data);
        self.chan.send(&data).await
    }
    pub async fn state_change(&self)->PeerConnectionState{
        self.peer.state_change().await
    }
}

/// Awaiting this function will block until the connection is closed.
pub async fn manage_connection(conn: ClientConnection){
    // Step 1: Client needs to send a Hello message to introduce itself.
    // Anything else breaks the link.
    let Some(msg) = conn.recv().await
        .ok() // Received a message (e.g.: connection didn't fail)
        .and_then(|x| packets::decode(x.to_vec()).ok()) // Was a valid packet
        .and_then(|x| match x { PktC2S::Hello(p) => Some(p), _ => None }) // Was Hello
        else { info!("Drop"); return; /* TODO: drop closes? */ };

    // Step 2: We create a session
    // TODO: SessionId session recovery
    let session = ActiveSession::new(conn);

    // Step 3: We send HelloReply
    let reply = {
        let lock = session.user.read().await;
        PktS2C_HelloReply::new(lock.id, lock.username.clone())
    };
    let Ok(_) = session.send(reply.encode()).await else { info!("Drop"); return; };

    // Step 4: We defer to the session handler
    session.handle_active_session().await;
}