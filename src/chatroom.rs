
extern crate std;

use std::ffi::CString;
use std::cmp::*;
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;

use serde_json::Value;

lazy_static!{
    static ref TOKEN: AtomicUsize = AtomicUsize::new(1);
}

#[derive(Debug, Clone)]
pub struct ChatRoom {
    id: String,
    alias: String,

    token: usize,
}

impl ChatRoom {
    pub fn from_json(json: &Value) -> ChatRoom {

        ChatRoom {
            id: json["UserName"].as_str().unwrap().to_owned(),
            alias: json["NickName"].as_str().unwrap().to_owned(),

            token: TOKEN.fetch_add(1, atomic::Ordering::SeqCst),
        }
    }

    pub fn token(&self) -> usize {
        self.token
    }

    pub fn id_cstring(&self) -> CString {
        CString::new(self.id.clone()).unwrap()
    }

    pub fn alias_cstring(&self) -> CString {
        CString::new(self.alias.clone()).unwrap()
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }
}

impl Ord for ChatRoom {
    fn cmp(&self, other: &ChatRoom) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for ChatRoom {
    fn partial_cmp(&self, other: &ChatRoom) -> Option<Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl PartialEq for ChatRoom {
    fn eq(&self, other: &ChatRoom) -> bool {
        self.id == other.id
    }
}

impl Eq for ChatRoom {}
