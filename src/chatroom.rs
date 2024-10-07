use std::collections::HashMap;
use lazy_static::lazy_static;
use log::{info, warn};
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::{packets::{self, Encode}, usersession::{ActiveSession, SessionId}};

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
#[derive(Clone)]
pub struct LobbyInfo{
    members: Vec<String>
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
    members: HashMap<SessionId, mpsc::Sender<ParticipantMsg>>,
}
/// A client's handle to the lobby.
/// Destroying this is equivalent to exiting the lobby.
pub struct LobbyHandle{
    pub broadcast_rx: broadcast::Receiver<ParticipantMsg>,
    pub individual_rx: mpsc::Receiver<ParticipantMsg>,
    // The session associated with this handle.
    sessionid: SessionId
}

//
impl Lobby{
    pub fn new()->Self{
        let (tx, _ ) = broadcast::channel(64);
        Self {
            sync: RwLock::new(LobbySync {
                log: vec![],
                members: HashMap::new()
            }),
            broadcast_tx: tx
        }
    }
    // Joins the lobby.
    // Registers the sessionid in the lobby struct, and returns a handle that receives both broadcast and individual messages.
    // For a lobby participant to send to the lobby, access through the global LOBBY
    pub async fn join(&self, session: &ActiveSession)->LobbyHandle{
        // Send a join message to all other participants
        let announcement = format!(">>> {} has joined", session.user.username);
        self.broadcast_tx.send(ParticipantMsg::Message(ChatMsg::Server(announcement)));

        // Create the lobby handle
        let broadcast_rx = self.broadcast_tx.subscribe();
        let (individual_tx, individual_rx) = mpsc::channel(64);
                // Send welcome
                let welcome = format!(">>> Welcome, {}.", session.user.username);
                individual_tx.send(ParticipantMsg::Message(ChatMsg::Server(welcome))).await;
        self.sync.write().await.members.insert(session.user.id, individual_tx);
        let handle = LobbyHandle{ broadcast_rx, individual_rx, sessionid: session.user.id };

        // update_participants
        self.update_lobby_participants().await;
        return handle;
    }
    // Removes a member & broadcasts the new lobby participant table
    pub async fn remove(&self, sessionid: SessionId){
        let Some(x) = self.sync.write().await.members.remove(&sessionid) else {
            warn!("Attempt to remove non-existent session {} from the lobby", sessionid);
            return;
        };

        let announcement = format!(">>> {} has left.", "<TODO: Usename>");
        self.broadcast_tx.send(ParticipantMsg::Message(ChatMsg::Server(announcement)));

        info!("Removed session {}", sessionid);
        self.update_lobby_participants().await;
    }
    // Broadcasts a list of lobby participants to all clients
    async fn update_lobby_participants(&self){
        // TODO: Rewire data such that I get usernames and not just session ids.
        // The lobby should be able to read the session info but never prevent a session being destroyed (weak ptr).
        // This means sessions need to be reference-counted, I think.
        // Network programming is so different to your run-of-the-mill sequence of operations.
        let list = self.sync.read().await.members.iter().map(|x| format!("TODO{}", x.0.0 % 100)).collect::<Vec<_>>();
        let packet = packets::PktS2C_LobbyInfo::new(list).encode();
        self.broadcast_tx.send(ParticipantMsg::RawPacket(packet));
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