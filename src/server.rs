
extern crate std;
extern crate hyper;
extern crate hyper_native_tls;
extern crate regex;

use self::hyper::Client;
use self::hyper::header::{SetCookie, Cookie, Header, Headers};
use self::hyper::net::HttpsConnector;
use self::hyper_native_tls::NativeTlsClient;
use self::regex::Regex;
use serde_json::Value;
use serde_json::Map;
use plugin_pointer::*;
use util::*;
use purple_sys::*;
use glib_sys::gpointer;
use std::os::raw::{c_void, c_char, c_int};
use std::io::*;
use std::ffi::CString;
use std::ptr::null_mut;
use std::sync::{RwLock, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::cell::Cell;
use std::marker::Copy;
use std::fs::{File, OpenOptions};
use std::thread;
use std::fmt::Debug;
use std::sync::Arc;

lazy_static!{
    pub static ref ACCOUNT: RwLock<GlobalPointer> = RwLock::new(GlobalPointer::new());
    // static ref TX: Mutex<Cell<>> = Mutex::new(Cell::new(None));
    static ref SRV_MSG: (Mutex<Sender<SrvMsg>>, Mutex<Receiver<SrvMsg>>) = {let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    static ref CLT_MSG: (Mutex<Sender<CltMsg>>, Mutex<Receiver<CltMsg>>) = {let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    static ref WECHAT: RwLock<WeChat> = RwLock::new(WeChat::new());
    static ref CLIENT: Client = {
        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        Client::with_connector(connector)
    };
}

#[derive(Debug)]
pub enum SrvMsg {
    ShowVerifyImage(String),
    AddContact(User),
}

#[derive(Debug)]
pub enum CltMsg {
    SendMsg,
}

struct WeChat {
    uin: String,
    sid: String,
    skey: String,
    device_id: String,
    // pass_ticket: String,
    headers: Headers,
}

unsafe impl std::marker::Sync for WeChat {}

#[derive(Debug)]
struct User {
    user_name: String,
    nick_name: String,
}

impl User {
    fn from_json(json: &Value) -> User {
        User {
            user_name: json["UserName"].as_str().unwrap().to_owned(),
            nick_name: json["NickName"].as_str().unwrap().to_owned(),
        }
    }

    fn user_name_str(&self) -> CString {
        CString::new(self.user_name.clone()).unwrap()
    }

    fn nick_name_str(&self) -> CString {
        CString::new(self.nick_name.clone()).unwrap()
    }
}

impl WeChat {
    fn new() -> WeChat {
        let mut headers = Headers::new();
        headers.set_raw("Cookie", vec![b"".to_vec()]);
        headers.set_raw("ContentType",
                        vec![b"application/json; charset=UTF-8".to_vec()]);
        headers.set_raw("Host", vec![b"web.wechat.com".to_vec()]);
        headers.set_raw("Referer",
                        vec![b"https://web.wechat.com/?&lang=zh_CN".to_vec()]);
        headers.set_raw("Accept",
                        vec![b"application/json, text/plain, */*".to_vec()]);

        WeChat {
            uin: String::new(),
            sid: String::new(),
            skey: String::new(),
            device_id: String::new(),
            headers: headers,
        }
    }

    fn uin(&self) -> &str {
        &self.uin
    }

    fn set_uin(&mut self, uin: &str) {
        self.uin = uin.to_owned();
    }

    fn sid(&self) -> &str {
        &self.uin
    }

    fn set_sid(&mut self, sid: &str) {
        self.sid = sid.to_owned();
    }

    fn skey(&self) -> &str {
        &self.skey
    }

    fn set_skey(&mut self, skey: &str) {
        self.skey = skey.to_owned();
    }

    fn set_cookies(&mut self, cookies: &SetCookie) {
        let mut jar = self.headers.get_mut::<Cookie>().unwrap();
        for c in cookies.iter() {
            let i = c.split(';').next().unwrap();
            jar.push(i.to_owned());
        }
    }

    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    fn base_data(&self) -> Value {

        let mut base_obj = Map::with_capacity(4);
        base_obj.insert("Uin".to_owned(), Value::String(self.uin.clone()));
        base_obj.insert("Sid".to_owned(), Value::String(self.sid.clone()));
        base_obj.insert("Skey".to_owned(), Value::String(self.skey.clone()));
        base_obj.insert("DeviceID".to_owned(), Value::String(String::new()));

        let mut obj = Map::new();
        obj.insert("BaseRequest".to_owned(), Value::Object(base_obj));

        Value::Object(obj)
    }
}

pub fn start_login() {

    let uuid = get_uuid();
    let file_path = save_qr_file(&uuid);

    // start check login thread
    thread::spawn(|| { check_scan(uuid); });
    SRV_MSG.0.lock().unwrap().send(SrvMsg::ShowVerifyImage(file_path)).unwrap();
}

fn check_scan(uuid: String) {
    let url = format!("https://login.weixin.qq.com/cgi-bin/mmwebwx-bin/login?uuid={}&tip={}",
                      uuid,
                      1);

    let result = get(&url);

    let url = format!("https://login.weixin.qq.com/cgi-bin/mmwebwx-bin/login?uuid={}&tip={}",
                      uuid,
                      0);

    // window.code=200;
    // window.redirect_uri="https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxnewloginpage?ticket=Au1vqm4uwpWOIQUCx-bMtOjT@qrticket_0&uuid=oZYNmWV5SQ==&lang=zh_CN&scan=1487767062";
    let result = get(&url);
    let reg = Regex::new(r#"redirect_uri="([^"]+)""#).unwrap();
    let caps = reg.captures(&result).unwrap();
    let uri = caps.get(1).unwrap().as_str();

    // webwxnewloginpage
    let url = format!("{}&fun=new", uri);
    let mut response = CLIENT.get(&url).send().unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    let cookies = response.headers.get::<SetCookie>().unwrap();

    let skey = regex_cap(&result, r#"<skey>(.*)</skey>"#);
    let sid = regex_cap(&result, r#"<wxsid>(.*)</wxsid>"#);
    let uin = regex_cap(&result, r#"<wxuin>(.*)</wxuin>"#);
    let pass_ticket = regex_cap(&result, r#"<pass_ticket>(.*)</pass_ticket>"#);

    {
        let mut wechat = WECHAT.write().unwrap();
        wechat.set_uin(&uin);
        wechat.set_skey(&skey);
        wechat.set_sid(&sid);
        wechat.set_cookies(&cookies);
    }

    // init
    let data = WECHAT.read().unwrap().base_data();
    let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxinit?lang=zh_CN&pass_ticket={}&skey={}",
                      pass_ticket,
                      skey);
    let result = post(&url, &data).parse::<Value>().unwrap();
    let ref contact_list = result["ContactList"].as_array().unwrap();
    for contact in *contact_list {
        let is_chat = contact["MemberCount"] != json!(0);
        if !is_chat {
            let user = User::from_json(contact);

            SRV_MSG.0.lock().unwrap().send(SrvMsg::AddContact(user)).unwrap();
        }
    }

    // start message check loop
    thread::spawn(|| sync_check());
}

fn sync_check() {

    // let uid
}

fn regex_cap<'a>(c: &'a str, r: &str) -> &'a str {
    let reg = Regex::new(r).unwrap();
    let caps = reg.captures(&c).unwrap();

    caps.get(1).unwrap().as_str()
}

fn get_uuid() -> String {
    let url = "https://login.web.wechat.com/jslogin?appid=wx782c26e4c19acffb";
    let mut response = CLIENT.get(url).send().unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();

    let reg = Regex::new(r#"uuid\s+=\s+"([\w=]+)""#).unwrap();
    let caps = reg.captures(&result).unwrap();

    caps.get(1).unwrap().as_str().to_owned()
}

fn save_qr_file<T: AsRef<str>>(url: T) -> String {
    let url = format!("https://login.weixin.qq.com/qrcode/{}", url.as_ref());
    let mut response = CLIENT.get(&url).send().unwrap();
    let mut result = Vec::new();
    response.read_to_end(&mut result).unwrap();

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("/tmp/qr.png")
        .unwrap();
    file.write(&result).unwrap();

    "/tmp/qr.png".to_owned()
}

fn get<T: AsRef<str> + Debug>(url: T) -> String {

    println!("get: {:?}", url);
    let mut response =
        CLIENT.get(url.as_ref()).headers(WECHAT.read().unwrap().headers()).send().unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    println!("result: {}", result);

    result
}

fn post<U: AsRef<str> + Debug>(url: U, data: &Value) -> String {

    let headers = WECHAT.read().unwrap().headers();
    println!("get: {:?}\nheaders:{:?}\npost_data: {:?}",
             url,
             headers,
             data);
    let mut response =
        CLIENT.post(url.as_ref()).headers(headers).body(&data.to_string()).send().unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    println!("result: {}", result);

    result
}

unsafe extern "C" fn check_srv(_: *mut c_void) -> c_int {

    let rx = SRV_MSG.1.lock().unwrap();

    if let Ok(m) = rx.try_recv() {
        debug(format!("GOT: {:#?}", m));

        match m {
            SrvMsg::ShowVerifyImage(path) => show_verify_image(path),
            SrvMsg::AddContact(user) => add_buddy(&user),
        }
    }

    1
}

unsafe fn add_buddy(user: &User) {

    println!("add_buddy: {:?}", user);

    let account = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
    let group_name = CString::new("Wechat").unwrap();
    let group = purple_find_group(group_name.as_ptr());

    let user_name = user.user_name_str();

    let buddy = purple_buddy_new(account, user_name.as_ptr(), user.nick_name_str().as_ptr());
    (*buddy).node.flags = PURPLE_BLIST_NODE_FLAG_NO_SAVE;
    purple_blist_add_buddy(buddy, null_mut(), group, null_mut());

    // set status to available
    let available = CString::new("available").unwrap();
    purple_prpl_got_user_status(account, user_name.as_ptr(), available.as_ptr());
}

pub fn login() {

    unsafe {
        purple_timeout_add(1000, Some(check_srv), null_mut());
    }

    std::thread::spawn(|| { start_login(); });
}

pub unsafe fn show_verify_image<T: AsRef<str>>(path: T) {

    // login qr-code
    let mut qr_image = File::open(path.as_ref()).unwrap();
    let mut buf = Vec::new();
    let qr_image_size = qr_image.read_to_end(&mut buf).unwrap();
    let qr_image_buf = CString::from_vec_unchecked(buf);

    let qr_code_id = CString::new("qrcode").unwrap();
    let qr_code_field = purple_request_field_image_new(qr_code_id.as_ptr(),
                                                       qr_code_id.as_ptr(),
                                                       qr_image_buf.as_ptr(),
                                                       qr_image_size as u64);

    let group = purple_request_field_group_new(null_mut());
    purple_request_field_group_add_field(group, qr_code_field);

    let fields = purple_request_fields_new();
    purple_request_fields_add_group(fields, group);

    let title = CString::new("Scan qr-code to login.").unwrap();
    let ok = CString::new("Ok").unwrap();
    let cancel = CString::new("Cancel").unwrap();
    let account = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
    purple_request_fields(purple_account_get_connection(account) as *mut c_void, // handle
                          title.as_ptr(), // title
                          title.as_ptr(), // primary
                          null_mut(), // secondary
                          fields, // fields
                          ok.as_ptr(), // ok_text
                          Some(ok_cb), // ok_cb
                          cancel.as_ptr(), // cancel_text
                          None, // cancel_cb
                          account, // account
                          null_mut(), // who
                          null_mut(), // conv
                          null_mut()); // user_data
}

extern "C" fn ok_cb() {}