use std::{collections::HashMap, future::Future, sync::Arc};
use lazy_static::lazy_static;
use log::{info, warn};
use tokio::sync::{broadcast, mpsc, RwLock, RwLockWriteGuard};

use crate::{packets::{Encode, PktS2C_LobbyInfo}, usersession::{ActiveSession, SessionId, UserSession}};

#[derive(Clone)]
pub enum ChatMsg{
    User(String),
    Server(String),
}
#[derive(Clone)]
pub enum ParticipantMsg{
    Message(ChatMsg),
    RawPacket(Vec<u8>)
}

// Global lobby
lazy_static!{
    pub static ref LOBBY: Lobby = Lobby::new();
}

/// Represents the chat lobby
pub struct Lobby{
    sync: RwLock<LobbySync>,
    pub broadcast_tx: broadcast::Sender<ParticipantMsg>,
}
unsafe impl Sync for Lobby{}
/// Contains the parts of the chat that must be synchronised in their modification
pub struct LobbySync{
    log: Vec<ChatMsg>,
    // Maps client session ids with their individual send channel - used so the lobby can send directly to a single person (e.g.: Name changes)
    members: HashMap<SessionId, LobbyMember>,
}

pub struct LobbyMember{
    send: mpsc::Sender<ParticipantMsg>,
    view: Arc<RwLock<UserSession>>, // NOTE: We never have dead members because LobbyHandle removes from the view
}

/// A client's handle to the lobby.
/// Destroying this object exits the session from the lobby.
pub struct LobbyHandle{
    pub broadcast_rx: broadcast::Receiver<ParticipantMsg>,
    pub individual_rx: mpsc::Receiver<ParticipantMsg>,
    // The session associated with this handle.
    sessionid: SessionId
}

//
impl Lobby{
    pub fn new()->Self{
        let (broadcast_tx, _) = broadcast::channel(64);
        Self {
            sync: RwLock::new(LobbySync {
                log: vec![],
                members: HashMap::new()
            }),
            broadcast_tx
        }
    }
    // Joins the lobby.
    // Registers the sessionid in the lobby struct, and returns a handle that receives both broadcast and individual messages.
    // For a lobby participant to send to the lobby, access through the global LOBBY
    pub async fn join(&self, session: &ActiveSession)->LobbyHandle{
        // Send a join message to all other participants
        let announcement = format!(">>> {} has joined", session.user().await.username);
        let _ = self.broadcast_tx.send(ParticipantMsg::Message(ChatMsg::Server(announcement)));

        // Create the lobby handle
        let broadcast_rx = self.broadcast_tx.subscribe();
        let (individual_tx, individual_rx) = mpsc::channel(64);
                // Send welcome
                let welcome = format!(">>> Welcome, {}.", session.user().await.username);
                let _ = individual_tx.send(ParticipantMsg::Message(ChatMsg::Server(welcome))).await;
        let member = LobbyMember{
            send: individual_tx,
            view: session.user.clone(),
        };
        self.write_sync().await.members.insert(session.user().await.id, member);
        let handle = LobbyHandle{ broadcast_rx, individual_rx, sessionid: session.user().await.id };

        // update_participants
        self.update_lobby_participants().await;
        return handle;
    }
    // Removes a member & broadcasts the new lobby participant table
    pub async fn remove(&self, sessionid: SessionId){
        let Some(session) = self.write_sync().await.members.remove(&sessionid) else {
            warn!("Attempt to remove non-existent session {} from the lobby", sessionid);
            return;
        };

        let username = session.view.read().await.username.clone();
        let announcement = format!(">>> {} has left.", username);
        let _ = self.broadcast_tx.send(ParticipantMsg::Message(ChatMsg::Server(announcement)));

        info!("Removed session {}", sessionid);
        self.update_lobby_participants().await;
    }
    // Broadcasts a list of lobby participants to all clients
    async fn update_lobby_participants(&self){
        // Network programming is so different to your run-of-the-mill sequence of operations.
        // This is not technically optimal because I should run futures for each of these reads.
        // ARRGH.
        let mut list = vec![];
        for x in self.sync.read().await.members.iter() {
            // Soundness: always sound since destruction of LobbyHandle removes Weak in map.
            let name = x.1.view.read().await.username.clone();
            list.push(name);
        }
        let packet = PktS2C_LobbyInfo::new(list).encode();
        let _ = self.broadcast_tx.send(ParticipantMsg::RawPacket(packet));
    }

    pub async fn send_message(&self, msg: ChatMsg){
        let _ = self.broadcast_tx.send(ParticipantMsg::Message(msg.clone()));
        self.write_sync().await.log.push(msg);
    }

    fn write_sync(&self)->impl Future<Output = RwLockWriteGuard<'_, LobbySync>>{
        self.sync.write()
    }
}
impl std::ops::Drop for LobbyHandle{
    // Leaves the lobby
    fn drop(&mut self) {
        let sid = self.sessionid;
        tokio::spawn(async move{
            LOBBY.remove(sid).await;
        });
    }
}