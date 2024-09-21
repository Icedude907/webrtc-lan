use bytes::Bytes;
use derive_new::new;
use just_webrtc::{platform::{Channel, PeerConnection}, DataChannelExt};
use just_webrtc::platform::Error as WebRTCError;

use crate::chatsession::CHAT;

/// Abstracts the WebRTC peer under a pseudo-"protocol" of unordered+unreliable or ordered+reliable streams
/// The reliable streams are for status and game graphics
/// The unreliable stream is for the client's inputs with the server
#[derive(new)]
pub struct ClientConnection{
    as_peer: PeerConnection,
    as_channel: Channel,
}

impl ClientConnection{
    pub async fn recv(&mut self)->Result<Bytes, WebRTCError>{
        self.as_channel.receive().await
    }
    // usize = bytes sent
    pub async fn send(&mut self, data: &Bytes)->Result<usize, WebRTCError>{
        self.as_channel.send(data).await
    }
}

pub fn manage_connection(mut conn: ClientConnection){
    tokio::spawn(async move{
        let (mut broadcast, send2everyone) = CHAT.lock().unwrap().join();

        loop{tokio::select! {
            c2s = conn.recv() => match c2s{
                // Receive data
                Ok(msg)=>{
                    println!("Got message {:?}", msg);
                    let _ = send2everyone.send( String::from_utf8_lossy(&msg).into_owned() );
                },
                // Connection shutdown
                Err(_)=>{ println!("Error: Connection lost.") }
            },
            s2c = broadcast.recv() => match s2c{
                Ok(msg)=>{
                    match conn.send(&msg.into()).await {
                        Ok(_) => {},
                        Err(_) => { println!("Error: Could not send to client"); break; }
                    }
                },
                Err(_)=>{ println!("Internal server error?"); break; }
            }
        }}
    });
}
