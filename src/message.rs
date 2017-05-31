
extern crate std;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use serde_json::Value;
use chatroom::ChatRoom;
use user::User;

lazy_static!{
    pub static ref SRV_MSG: (Mutex<Sender<SrvMsg>>, Mutex<Receiver<SrvMsg>>) = {
        let (tx, rx) = channel();
        (Mutex::new(tx), Mutex::new(rx))
    };
}

#[derive(Debug)]
pub enum SrvMsg {
    ShowMessageBox(String),
    ShowVerifyImage(String),
    AddContact(User),
    AddGroup(ChatRoom),
    MessageReceived(Value),
    AppendImageMessage(i32, Value),
    RefreshChatMembers(String),
    YieldEvent,
    QuitEvent,
}

pub fn send_server_message(m: SrvMsg) {
    SRV_MSG.0.lock().unwrap().send(m).unwrap();
}

