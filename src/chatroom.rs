
extern crate std;

use std::os::raw::c_void;
use std::ffi::CString;
use std::cmp::*;
use std::sync::atomic;
use std::sync::atomic::AtomicUsize;

use pointer::Pointer;
use serde_json::Value;

lazy_static!{
    static ref TOKEN: AtomicUsize = AtomicUsize::new(1);
}

#[derive(Clone, Debug)]
pub struct ChatRoom {
    id: String,
    alias: String,

    token: usize,
    ptr: Pointer,
}

impl ChatRoom {
    pub fn from_json(json: &Value) -> ChatRoom {

        ChatRoom {
            id: json["UserName"].as_str().unwrap().to_owned(),
            alias: json["NickName"].as_str().unwrap().to_owned(),

            token: TOKEN.fetch_add(1, atomic::Ordering::SeqCst),
            ptr: Pointer::new(),
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

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn set_chat_ptr(&mut self, chat: *mut c_void) {
        self.ptr.set(chat);
    }

    pub fn chat_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Ord for ChatRoom {
    fn cmp(&self, other: &ChatRoom) -> Ordering {
        self.token.cmp(&other.token)
    }
}

impl PartialOrd for ChatRoom {
    fn partial_cmp(&self, other: &ChatRoom) -> Option<Ordering> {
        Some(self.token.cmp(&other.token))
    }
}

impl PartialEq for ChatRoom {
    fn eq(&self, other: &ChatRoom) -> bool {
        self.token == other.token
    }
}

impl Eq for ChatRoom {}
