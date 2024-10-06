use std::{future::Future, sync::Mutex};

use bytes::Bytes;
use derive_more::derive::{Deref, Display};
use just_webrtc::types::PeerConnectionState;
use lazy_static::lazy_static;
use log::{info, warn};
use tokio::sync::broadcast;

use crate::{chatroom::CHAT, packets::{self, Encode}, util::UUIDGen, webrtcpeer::{ClientConnection, RecvError}};

// struct SessionTable{
// }

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
        let (mut broadcast, send2everyone) = CHAT.lock().unwrap().add_participant(&self.user.username);

        loop{tokio::select! {
            c2s = self.conn.recv() => match c2s{
                // Receive data
                Ok(msg)=>{self.handle_incoming(msg, &send2everyone).await;},
                // Connection shutdown
                Err(RecvError::Abort)=>{ break; }
                Err(_)=>{ warn!("Unexpected error. Closing connection."); break; }
            },
            s2c = broadcast.recv() => match s2c{
                Ok(msg)=>{
                    match self.handle_outgoing(msg).await {
                        Ok(_) => {},
                        Err(x) => { warn!("Error: Could not send to client {}", x); break; }
                    }
                },
                Err(_)=>{ info!("Internal server error?"); break; }
            },
            state = self.conn.state_change() => match self.handle_connection_state_change(state){
                Ok(_) => {},
                Err(_) => { info!("Connection finished"); break; }
            },
        }}
        info!("Connection with {} has finished.", self.user.username);
        let _ = send2everyone.send(format!{">>> {} has left.", self.user.username});
    }
    fn handle_connection_state_change(&self, state: PeerConnectionState) -> Result<(),()>{
        use PeerConnectionState::*;
        match state{
            Failed => return Err(()),
            Closed => return Err(()),
            Connecting => info!("{} connecting...", self.user.id),
            Disconnected => info!("Connection interrupted with {}", self.user.id),
            _ => {}
        }
        return Ok(())
    }
    async fn handle_incoming(&mut self, msg: Bytes, send2everyone: &broadcast::Sender<String>)->Result<(),()>{
        use packets::PktC2S::*;
        info!("{} >> {:?}", self.user.username, msg);
        let Ok(pkt) = packets::decode(msg.to_vec()) else {
            warn!("{} connection error", self.user.username);
            return Err(())
        };
        info!("\t{:?}", pkt);
        match pkt{
            SendMsg(p)=>{
                let _ = send2everyone.send(p.msg);
            }
            SetName(p)=>{
                // Send approval
                self.send(packets::PktS2C_SetNameReply::new(p.name.clone()).encode().into()).await;
                if self.user.username != p.name {
                    let _ = send2everyone.send(format!(">>> {} is now {}", self.user.username, p.name));
                    self.user.username = p.name;
                }
            }
            _ => {},
        }
        return Ok(())
    }
    async fn handle_outgoing(&self, msg: String)->Result<usize, just_webrtc::platform::Error>{
        let msg = packets::PktS2C_ReceiveMsg::new(msg).encode();
        let bytes = msg.into();
        // info!("{} << {:?}", self.user.username, bytes);
        self.send(bytes).await
    }
}

#[derive(Copy, Clone, Debug, Display)]
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