use std::sync::Mutex;
use lazy_static::lazy_static;
use tokio::sync::broadcast;

lazy_static!{
    pub static ref CHAT: Mutex<ChatRoom> = Mutex::new(ChatRoom::new());
}

type Message = String;

/// Represents the active chat room
pub struct ChatRoom{
    // log: Vec<String>,
    broadcast_tx: broadcast::Sender<Message>,
}
impl ChatRoom{
    pub fn new()->Self{
        let (broadcast_tx, _) = broadcast::channel(100);
        Self { broadcast_tx }
    }
    // Returns a receiver that contains other participant messages
    // And a sender that can be used to push your messages out.
    pub fn join(&mut self)->(broadcast::Receiver<Message>, broadcast::Sender<Message>){
        let broadcast_tx = self.broadcast_tx.clone();
        let broadcast_rx = broadcast_tx.subscribe();
        return (broadcast_rx, broadcast_tx);
    }
}