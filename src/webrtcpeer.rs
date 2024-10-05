use bytes::Bytes;
use derive_new::new;
use just_webrtc::{platform::{Channel, PeerConnection}, types::PeerConnectionState, DataChannelExt, PeerConnectionExt};
use just_webrtc::platform::Error as WebRTCError;
use log::{info, warn};
use tokio::signal::ctrl_c;

use crate::{chatroom::CHAT, packets::{self, Encode}};

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
    pub async fn recv(&self)->Result<Bytes, WebRTCError>{
        self.main_channel.receive().await
    }
    // usize = bytes sent
    pub async fn send(&self, data: Bytes)->Result<usize, WebRTCError>{
        self.main_channel.send(&data).await
    }
    pub fn get_id(&self)->u64{ self.connection_id }
    pub fn get_connection_name(&self)->String{ format!("{:016X}", self.connection_id) }
}

pub fn manage_connection(mut conn: ClientConnection){
    tokio::spawn(async move{
        let short_name = format!("*{:04X}", conn.get_id() % 0x10000);
        let (mut broadcast, send2everyone) = CHAT.lock().unwrap().join(&short_name);
        // TODO: How to close the connection (without waiting for a failed send / manually implementing timeouts?)

        let handle_incoming = |msg: Bytes|->_{
            use packets::PktC2S::*;
            info!("{} >> {:?}", conn.get_connection_name(), msg);
            let Ok(pkt) = packets::decode(msg.to_vec()) else { warn!("{} connection error", conn.get_connection_name()); return Err(())};
            info!("\t{:?}", pkt);
            match pkt{
                SendMsg(p)=>{
                    // let msg = format!("{}: {}", short_name, String::from_utf8_lossy(&msg));
                    let Ok(_) = send2everyone.send(p.msg) else {return Err(())};
                }
                _ => {},
            }
            return Ok(())
        };
        let handle_outgoing = |msg: String|->_{
            let msg = packets::PktS2C_ReceiveMsg::new(msg).encode();
            let bytes = msg.into();
            info!("{} << {:?}", conn.get_connection_name(), bytes);
            conn.send(bytes)
        };

        loop{tokio::select! {
            c2s = conn.recv() => match c2s{
                // Receive data
                Ok(msg)=>{handle_incoming(msg);},
                // Connection shutdown
                Err(_)=>{ warn!("Unexpected error. Closing connection."); break; }
            },
            s2c = broadcast.recv() => match s2c{
                Ok(msg)=>{
                    match handle_outgoing(msg).await {
                        Ok(_) => {},
                        Err(x) => { warn!("Error: Could not send to client {}", x); break; }
                    }
                },
                Err(_)=>{ info!("Internal server error?"); break; }
            },
            state = conn._peer.state_change() => match handle_connection_state_change(state, &conn){
                Ok(_) => {},
                Err(_) => { info!("Connection finished"); break }
            },
            _exit = ctrl_c() => { break; }
        }}
        info!("Connection with {} has finished.", conn.get_connection_name());
        let _ = send2everyone.send(format!{">>> {} has left.", short_name});
    });
}

fn handle_connection_state_change(state: PeerConnectionState, conn: &ClientConnection) -> Result<(),()>{
    use PeerConnectionState::*;
    match state{
        Failed => return Err(()),
        Closed => return Err(()),
        Connecting => info!("{} connecting...", conn.get_connection_name()),
        Disconnected => info!("Connection interrupted with {}", conn.get_connection_name()),
        _ => {}
    }
    return Ok(())
}