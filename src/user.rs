
extern crate std;

use std::ffi::CString;
use std::cmp::*;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct User {
    user_name: String,
    nick_name: String,
}

impl User {
    pub fn from_json(json: &Value) -> User {
        User {
            user_name: json["UserName"].as_str().unwrap().to_owned(),
            nick_name: json["NickName"].as_str().unwrap().to_owned(),
        }
    }

    pub fn user_name_str(&self) -> CString {
        CString::new(self.user_name.clone()).unwrap()
    }

    pub fn nick_name_str(&self) -> CString {
        CString::new(self.nick_name.clone()).unwrap()
    }
}

impl Ord for User {
    fn cmp(&self, other: &User) -> Ordering {
        self.user_name.cmp(&other.user_name)
    }
}

impl PartialOrd for User {
    fn partial_cmp(&self, other: &User) -> Option<Ordering> {
        Some(self.user_name.cmp(&other.user_name))
    }
}

impl PartialEq for User {
    fn eq(&self, other: &User) -> bool {
        self.user_name == other.user_name
    }
}

impl Eq for User { }
