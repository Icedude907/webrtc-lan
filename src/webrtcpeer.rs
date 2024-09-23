use bytes::Bytes;
use derive_new::new;
use just_webrtc::{platform::{Channel, PeerConnection}, DataChannelExt};
use just_webrtc::platform::Error as WebRTCError;
use log::{info, trace, warn};
use tokio::signal::ctrl_c;

use crate::chatsession::CHAT;

/// Abstracts the WebRTC peer under a pseudo-"protocol" of unordered+unreliable or ordered+reliable streams
/// The reliable streams are for status and game graphics
/// The unreliable stream is for the client's inputs with the server
#[derive(new)]
pub struct ClientConnection{
    _peer: PeerConnection,
    main_channel: Channel,
    connection_id: u64,
}

impl ClientConnection{
    pub async fn recv(&mut self)->Result<Bytes, WebRTCError>{
        self.main_channel.receive().await
    }
    // usize = bytes sent
    pub async fn send(&mut self, data: &Bytes)->Result<usize, WebRTCError>{
        self.main_channel.send(data).await
    }
    pub fn get_id(&self)->u64{ self.connection_id }
    pub fn get_connection_name(&self)->String{ format!("{:016X}", self.connection_id) }
}

pub fn manage_connection(mut conn: ClientConnection){
    tokio::spawn(async move{
        let short_name = format!("...{:04X}", conn.get_id() % 0x10000);
        let (mut broadcast, send2everyone) = CHAT.lock().unwrap().join(&short_name);
        // TODO: How to close the connection (without waiting for a failed send / manually implementing timeouts?)
        loop{tokio::select! {
            c2s = conn.recv() => match c2s{
                // Receive data
                Ok(msg)=>{
                    info!("{}> {:?}", conn.get_connection_name(), msg);
                    let _ = send2everyone.send( String::from_utf8_lossy(&msg).into_owned() );
                },
                // Connection shutdown
                Err(_)=>{ warn!("Unexpected error. Closing connection."); break; }
            },
            s2c = broadcast.recv() => match s2c{
                Ok(msg)=>{
                    trace!("Broadcaster recv");
                    match conn.send(&msg.into()).await {
                        Ok(_) => {},
                        Err(x) => { warn!("Error: Could not send to client {}", x); break; }
                    }
                },
                Err(_)=>{ info!("Internal server error?"); break; }
            },
            _exit = ctrl_c() => { break; }
        }}
        info!("Connection with {} has finished.", conn.get_connection_name());
        let _ = send2everyone.send(format!{">>> {} has left.", short_name});
    });
}
