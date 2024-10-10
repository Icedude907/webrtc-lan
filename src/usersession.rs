use std::sync::Mutex;

use bytes::Bytes;
use derive_more::derive::{Deref, Display};
use just_webrtc::types::PeerConnectionState;
use lazy_static::lazy_static;
use log::{info, warn};

use crate::{chatroom::{ChatMsg, LobbyHandle, ParticipantMsg, LOBBY}, packets::{self, Encode, PktS2C_ReceiveMsg, PktS2C_SetNameReply}, util::UUIDGen, webrtcpeer::{ClientConnection, RecvError}};

#[derive(Deref)]
pub struct ActiveSession{
    #[deref]
    conn: ClientConnection,
    pub user: UserSession
}
impl ActiveSession{
    pub fn new(conn: ClientConnection)->Self{
        Self{conn, user: UserSession::new()}
    }
    pub async fn handle_active_session(mut self){
        let mut handle = LOBBY.join(&self).await;

        loop{tokio::select! {
            // Receive data. If error, drop the session.
            c2s = self.conn.recv() => match c2s{
                Ok(msg)=>{
                    match self.handle_incoming(msg, &handle).await{
                        Ok(_) => {},
                        Err(_) => break,
                    };
                },
                // Connection shutdown
                Err(RecvError::Abort)=>{ break; }
                Err(_)=>{ warn!("Unexpected error. Closing the connection."); break; }
            },
            // Send data. If error, drop the session.
            s2c = handle.broadcast_rx.recv() => match s2c{
                Ok(msg)=>{
                    match self.handle_outgoing(msg).await {
                        Ok(_) => {},
                        Err(x) => { warn!("Error: Could not send to client {}", x); break; }
                    }
                },
                Err(_)=>{ info!("Internal server error."); break; }
            },
            // If the WebRTC state is failed, close the session.
            state = self.conn.state_change() => match self.handle_connection_state_change(state){
                Ok(_) => {},
                Err(_) => { break; }
            },
        }}
        info!("Connection with {} has finished.", self.user.username);
    }
    fn handle_connection_state_change(&self, state: PeerConnectionState) -> Result<(),()>{
        use PeerConnectionState::*;
        match state{
            Failed | Closed => return Err(()),
            Connecting => info!("{} connecting...", self.user.id),
            Disconnected => info!("Connection interrupted with {}", self.user.id),
            _ => {}
        }
        return Ok(())
    }

    // Handles incoming raw client messages and dispatches them to the appropriate location.
    // If Err(), the caller should drop the connection.
    async fn handle_incoming(&mut self, msg: Bytes, handle: &LobbyHandle)->Result<(),()>{
        use packets::PktC2S::*;
        use crate::chatroom::ChatMsg::*;

        let Ok(pkt) = packets::decode(msg.to_vec()) else {
            warn!("(DROPPING) {} >> {:?}", self.user.username, msg);
            return Err(());
        };

        info!("{} >> {:?}", self.user.username, pkt);
        match pkt{
            SendMsg(p)=>{
                let msg = format!("{}) {}", self.user.username, p.msg);
                LOBBY.send_message(ChatMsg::User(msg)).await;
            }
            SetName(p)=>{
                // Send approval
                let _ = self.send(PktS2C_SetNameReply::new(p.name.clone()).encode()).await;
                // Propagate server message
                let announcement = format!(">>> {} is now {}", self.user.username, p.name);
                if self.user.username != p.name {
                    let _ = LOBBY.broadcast_tx.send(ParticipantMsg::Message(Server(announcement)));
                    self.user.username = p.name;
                }
            }
            Goodbye(_)=>{
                return Err(());
            }
            _ => {
                warn!("(UNEXPECTED. DROPPING) {}", self.user.username);
                return Err(());
            },
        }
        return Ok(());
    }

    // Propagates outgoing messages onto the wire.
    // TODO: Should this be serialising messages or not?
    async fn handle_outgoing(&self, msg: ParticipantMsg)->Result<usize, just_webrtc::platform::Error>{
        use crate::chatroom::ChatMsg::*;
        let msg = match msg{
            ParticipantMsg::Message(msg) => PktS2C_ReceiveMsg::new(match msg{User(x)=>x, Server(x)=>x}).encode(),
            ParticipantMsg::RawPacket(x) => x,
        };
        // info!("{} << {:?}", self.user.username, bytes);
        self.send(msg).await
    }
}

#[derive(Copy, Clone, Debug, Display, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[display("{_0:x}")]
pub struct SessionId(pub u64);

pub struct UserSession{
    pub id: SessionId,
    pub username: String,
}
impl UserSession{
    pub fn new()->Self{
        lazy_static!{
            static ref ID_GEN: Mutex<UUIDGen> = UUIDGen::new_now().into();
        }
        let id = ID_GEN.lock().unwrap().next();
        Self { id: SessionId(id), username: Self::get_username_for_id(id) }
    }
    pub fn get_username_for_id(id: u64)->String{
        let usernames = ["Abiu","Akebi","Ackee","African","American","Apple","Apricot","Aratiles","Araza","Avocado","Banana","Bilberry","Blackberry","Blackcurrant","Blueberry","Boysenberry","Breadfruit","Cactus","Canistel","Catmon","Cempedak","Cherimoya","Cherry","Chico","Citron","Cloudberry","Coco","Coconut","Crab","Cranberry","Currant","Damson","Date","Dragonfruit","Durian","Elderberry","Feijoa","Fig","Finger","Gac","Goji","Gooseberry","Grape","Raisin","Grapefruit","Grewia","Guava","Hala","Haws,","Honeyberry","Huckleberry","Jabuticaba","Jackfruit","Jambul","Japanese","Jostaberry","Jujube","Juniper","Kaffir","Kiwano","Kiwifruit","Kumquat","Lanzones","Lemon","Lime","Loganberry","Longan","Loquat","Lulo","Lychee","Magellan","Macopa","Mamey","Mamey","Mango","Mangosteen","Marionberry","Medlar","Melon","Cantaloupe","Galia","Honeydew","Mouse","Muskmelon","Watermelon","Miracle","Momordica","Monstera","Mulberry","Nance","Nectarine","Orange","Blood","Clementine","Mandarine","Tangerine","Papaya","Passionfruit","Pawpaw","Peach","Pear","Persimmon","Plantain","Plum","Prune","Pineapple","Pineberry","Plumcot","Pomegranate","Pomelo","Quince","Raspberry","Salmonberry","Rambutan","Redcurrant","Rose","Salal","Salak","Santol","Sapodilla","Sapote","Sarguelas","Satsuma","Sloe","Soursop","Star","Strawberry","Sugar","Suriname","Tamarillo","Tamarind","Tangelo","Tayberry","Thimbleberry","Ugli","White","Ximenia","Yuzu"];
        let idx = id / 10_u64.pow(3);
        let digits = id % 10_u64.pow(3);
        let idx = idx % usernames.len() as u64;

        let name = usernames[idx as usize];
        let name = format!("{}{:03}", name, digits);
        return name;
    }
}