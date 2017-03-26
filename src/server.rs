
extern crate std;
extern crate hyper;
extern crate hyper_native_tls;
extern crate regex;
extern crate time;

use self::hyper::Client;
use self::hyper::header::{SetCookie, Cookie, Headers};
use self::hyper::net::HttpsConnector;
use self::hyper_native_tls::NativeTlsClient;
use self::regex::Regex;
use user::User;
use serde_json::Value;
use serde_json::Map;
use plugin_pointer::*;
use purple_sys::*;
use std::os::raw::{c_void, c_char, c_int};
use std::io::*;
use std::ffi::{CStr, CString};
use std::ptr::null_mut;
use std::sync::{RwLock, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::fs::{File, OpenOptions};
use std::thread;
use std::fmt::Debug;
use std::collections::BTreeSet;

lazy_static!{
    pub static ref ACCOUNT: RwLock<GlobalPointer> = RwLock::new(GlobalPointer::new());
    // static ref TX: Mutex<Cell<>> = Mutex::new(Cell::new(None));
    static ref SRV_MSG: (Mutex<Sender<SrvMsg>>, Mutex<Receiver<SrvMsg>>) = {let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    // static ref CLT_MSG: (Mutex<Sender<CltMsg>>, Mutex<Receiver<CltMsg>>) = {let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    static ref WECHAT: RwLock<WeChat> = RwLock::new(WeChat::new());
    static ref CLIENT: Client = {
        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        Client::with_connector(connector)
    };
}

#[derive(Debug)]
enum SrvMsg {
    ShowVerifyImage(String),
    AddContact(User),
    MessageReceived(Value),
}

// #[derive(Debug)]
// pub enum CltMsg {
// }

struct WeChat {
    uin: String,
    sid: String,
    skey: String,
    // device_id: String,
    pass_ticket: String,
    headers: Headers,
    user_info: Value,
    sync_keys: Value,

    user_list: BTreeSet<User>,
}

unsafe impl std::marker::Sync for WeChat {}

impl WeChat {
    fn new() -> WeChat {
        let mut headers = Headers::new();
        headers.set_raw("Cookie", vec![vec![]]);
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
            // device_id: String::new(),
            pass_ticket: String::new(),
            headers: headers,
            user_info: Value::Null,
            sync_keys: Value::Null,

            user_list: BTreeSet::new(),
        }
    }

    fn uin(&self) -> &str {
        &self.uin
    }

    fn set_uin(&mut self, uin: &str) {
        self.uin = uin.to_owned();
    }

    fn sid(&self) -> &str {
        &self.sid
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

    fn pass_ticket(&self) -> &str {
        &self.pass_ticket
    }

    fn set_pass_ticket(&mut self, pass_ticket: &str) {
        self.pass_ticket = pass_ticket.to_owned();
    }

    fn sync_key_str(&self) -> String {

        assert!(self.sync_keys.is_array());

        let mut buf = String::new();
        for item in self.sync_keys.as_array().unwrap() {
            let k = item["Key"].as_i64().unwrap();
            let v = item["Val"].as_i64().unwrap();

            buf.push_str(&format!("{}_{}|", k, v));
        }
        buf.pop();

        buf
    }

    fn sync_key(&self) -> Value {
        assert!(self.sync_keys.is_array());

        let count = self.sync_keys
            .as_array()
            .unwrap()
            .len();
        let value = json!({"Count" : count, "List" : self.sync_keys});

        value
    }

    fn set_sync_key(&mut self, json: &Value) {
        if let Value::Array(ref list) = json["List"] {
            self.sync_keys = Value::Array(list.clone());
        }
    }

    fn set_user_info(&mut self, json: &Value) {
        self.user_info = json["User"].clone()
    }

    fn user_name(&self) -> &str {
        self.user_info["UserName"].as_str().unwrap()
    }

    fn set_cookies(&mut self, cookies: &SetCookie) {
        println!("cookies: {:?}", cookies);
        let ref mut jar = self.headers.get_mut::<Cookie>().unwrap();
        for c in cookies.iter() {
            let i = c.split(';').next().unwrap();
            assert!(!i.is_empty());
            jar.push(i.to_owned());
        }

        jar.remove(0);
    }

    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    fn append_user(&mut self, user: &User) {
        if self.user_list.insert(user.clone()) {
            SRV_MSG.0
                .lock()
                .unwrap()
                .send(SrvMsg::AddContact(user.clone()))
                .unwrap();
        }
    }

    fn base_data(&self) -> Value {

        let mut base_obj = Map::with_capacity(4);
        base_obj.insert("Uin".to_owned(), json!(self.uin.parse::<usize>().unwrap()));
        base_obj.insert("Sid".to_owned(), Value::String(self.sid.clone()));
        base_obj.insert("Skey".to_owned(), Value::String(self.skey.clone()));
        base_obj.insert("DeviceID".to_owned(), Value::String(String::new()));

        let mut obj = Map::new();
        obj.insert("BaseRequest".to_owned(), Value::Object(base_obj));

        Value::Object(obj)
    }

    fn status_notify_data(&self) -> Value {

        let mut value = self.base_data();

        value["Code"] = json!(3);
        value["FromUserName"] = json!(self.user_name());
        value["ToUserName"] = json!(self.user_name());
        value["ClientMsgId"] = json!(time_stamp());

        value
    }

    fn message_check_data(&self) -> Value {

        let mut value = self.base_data();

        value["SyncKey"] = self.sync_key().clone();
        value["rr"] = json!(!time_stamp());

        value
    }

    fn message_send_data(&self, who: String, content: String) -> Value {

        let mut id = time_stamp().to_string();
        id.push_str("1234");

        let msg = json!({
            "Type" : 1,
            "Content" : json!(content),
            "FromUserName" : json!(self.user_name()),
            "ToUserName" : json!(who),
            "LocalID" : json!(id),
            "ClientMsgId" : json!(id)
        });

        let mut value = self.base_data();
        value["Msg"] = msg;
        value["Scene"] = json!(0);

        value
    }
}

pub fn start_login() {

    let uuid = get_uuid();
    let file_path = save_qr_file(&uuid);

    // start check login thread
    thread::spawn(|| { check_scan(uuid); });
    SRV_MSG.0
        .lock()
        .unwrap()
        .send(SrvMsg::ShowVerifyImage(file_path))
        .unwrap();
}

fn check_scan(uuid: String) {
    let url = format!("https://login.weixin.qq.com/cgi-bin/mmwebwx-bin/login?uuid={}&tip={}",
                      uuid,
                      1);
    // TODO: check result
    let _ = get(&url);

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
        wechat.set_pass_ticket(&pass_ticket);
        wechat.set_cookies(&cookies);
    }

    // init
    let data = { WECHAT.read().unwrap().base_data() };
    let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxinit?lang=zh_CN&pass_ticket={}&skey={}",
                      pass_ticket,
                      skey);
    let json = post(&url, &data).parse::<Value>().unwrap();
    {
        let mut wechat = WECHAT.write().unwrap();
        wechat.set_sync_key(&json["SyncKey"]);
        wechat.set_user_info(&json);
    }

    // {
    //     let mut wechat = WECHAT.write().unwrap();
    //     let ref contact_list = json["ContactList"].as_array().unwrap();
    //     for contact in *contact_list {
    //         let is_chat = contact["MemberCount"] != json!(0);
    //         if !is_chat {
    //             wechat.append_user(&User::from_json(contact));
    //         }
    //     }
    // }

    // fetch contact list
    thread::spawn(|| fetch_contact());

    // refersh current user name
    unsafe {
        let uname = CString::new(WECHAT.read().unwrap().user_name()).unwrap();
        let alias = CString::new("You").unwrap();
        println!("set usernmae: {:?}", uname);
        purple_account_set_username(ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount,
                                    uname.as_ptr());
        purple_account_set_alias(ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount,
                                 alias.as_ptr());
    }

    // status notify
    let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxstatusnotify?lang=zh_CN&pass_ticket={}",
                      pass_ticket);
    let data = WECHAT.read().unwrap().status_notify_data();
    // TODO: check result
    let _ = post(&url, &data);

    // start message check loop
    thread::spawn(|| sync_check());
}

fn time_stamp() -> i64 {
    time::get_time().sec * 1000
}

fn fetch_contact() {
    let url = {
        let wechat = WECHAT.read().unwrap();
        format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxgetcontact?pass_ticket={}&skey={}&r={}&seq=0", wechat.pass_ticket(), wechat.skey(), time_stamp())
    };

    let result = get(url).parse::<Value>().unwrap();
    let ref member_list = result["MemberList"].as_array().unwrap();

    let mut wechat = WECHAT.write().unwrap();
    for member in *member_list {
        wechat.append_user(&User::from_json(member));
    }
}

fn sync_check() {

    let mut headers = Headers::new();
    {
        let hrs = WECHAT.read().unwrap().headers();
        println!("{:?}", hrs);
        headers.set(hrs.get::<Cookie>().unwrap().clone());
        headers.set_raw("Host", vec![b"webpush.web.wechat.com".to_vec()]);
        headers.set_raw("Accept", vec![b"*/*".to_vec()]);
        headers.set_raw("Referer",
                        vec![b"https://webpush.web.wechat.com/?&lang=zh_CN".to_vec()]);
    }

    println!("{:?}", headers);

    // let uid
    loop {
        let url = {
            let wechat = WECHAT.read().unwrap();
            let ts = time_stamp();
            format!("https://webpush.web.wechat.\
                 com/cgi-bin/mmwebwx-bin/synccheck?sid={}&uin={}&skey={}&deviceid=&synckey={}&r={}&_={}",
                    wechat.sid(),
                    wechat.uin(),
                    wechat.skey(),
                    wechat.sync_key_str(),
                    ts,
                    ts)
        };

        println!("sync check url: {}", url);

        let mut response = CLIENT.get(&url)
            .headers(headers.clone())
            .send()
            .unwrap();
        let mut result = String::new();
        response.read_to_string(&mut result).unwrap();

        let reg = Regex::new(r#"retcode:"(\d+)",selector:"(\d+)""#).unwrap();
        let caps = reg.captures(&result).unwrap();
        let retcode: isize = caps.get(1)
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        let selector: isize = caps.get(2)
            .unwrap()
            .as_str()
            .parse()
            .unwrap();
        println!("{} = {} - {}", result, retcode, selector);

        // check error
        if retcode != 0 {
            break;
        }

        // no new message.
        if selector == 0 {
            continue;
        }

        check_new_message();
    }
}

pub unsafe extern "C" fn send_im(_: *mut PurpleConnection,
                                 who: *const c_char,
                                 msg: *const c_char,
                                 _: PurpleMessageFlags)
                                 -> c_int {

    let who = CStr::from_ptr(who).to_string_lossy().into_owned().to_owned();
    let msg = CStr::from_ptr(msg).to_string_lossy().into_owned().to_owned();

    let (url, data) = {
        let wechat = WECHAT.read().unwrap();
        let url = format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxsendmsg?pass_ticket={}", wechat.pass_ticket());
        let data = wechat.message_send_data(who, msg);

        (url, data)
    };
    // TODO: check result.
    let _ = post(url, &data);

    // CLT_MSG.0
    // .lock()
    // .unwrap()
    // .send(CltMsg::SendMsg(who, msg));

    1
}

fn check_new_message() {

    let (url, data) = {
        let wechat = WECHAT.read().unwrap();
        let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxsync?sid={}&skey={}&pass_ticket={}",
                      wechat.sid(),
                      wechat.skey(),
                      wechat.pass_ticket());

        (url, wechat.message_check_data())
    };

    let result = post(&url, &data);

    // refersh sync check key
    let json: Value = result.parse().unwrap();
    {
        WECHAT.write().unwrap().set_sync_key(&json["SyncCheckKey"]);
    }

    SRV_MSG.0
        .lock()
        .unwrap()
        .send(SrvMsg::MessageReceived(json))
        .unwrap();
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

    caps.get(1)
        .unwrap()
        .as_str()
        .to_owned()
}

fn save_qr_file<T: AsRef<str>>(qr: T) -> String {
    let url = format!("https://login.weixin.qq.com/qrcode/{}", qr.as_ref());
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

    let headers = { WECHAT.read().unwrap().headers() };

    println!("get: {:?}", url);
    let mut response = CLIENT.get(url.as_ref())
        .headers(headers)
        .send()
        .unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    if result.len() > 500 {
        println!("result: {}", &result[0..300]);
    } else {
        println!("result: {}", result);
    }

    result
}

fn post<U: AsRef<str> + Debug>(url: U, data: &Value) -> String {

    let headers = { WECHAT.read().unwrap().headers() };
    println!("post: {:?}\nheaders:{:?}\npost_data: {:?}",
             url,
             headers,
             data);
    let mut response = CLIENT.post(url.as_ref())
        .headers(headers)
        .body(&data.to_string())
        .send()
        .unwrap();
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    if result.len() > 500 {
        println!("result: {}", &result[0..300]);
    } else {
        println!("result: {}", result);
    }

    result
}

unsafe extern "C" fn check_srv(_: *mut c_void) -> c_int {

    let rx = SRV_MSG.1.lock().unwrap();

    while let Ok(m) = rx.try_recv() {
        match m {
            SrvMsg::ShowVerifyImage(path) => show_verify_image(path),
            SrvMsg::AddContact(user) => add_buddy(&user),
            SrvMsg::MessageReceived(json) => append_message(&json),
        }
    }

    1
}

// unsafe fn conversion(conv_type: PurpleConversationType, name: &str) -> *mut PurpleConversation {
//     let account = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
//     let name = CString::new(name).unwrap();
//     let conv = purple_find_conversation_with_account(conv_type, name.as_ptr(), account);

//     if conv == null_mut() {
//         purple_conversation_new(conv_type, account, name.as_ptr())
//     } else {
//         conv
//     }
// }

// unsafe fn conv_chat(name: &str) -> *mut PurpleConvChat {
//     let conv = conversion(PURPLE_CONV_TYPE_CHAT, name);
//     purple_conversation_get_chat_data(conv)
// }

unsafe fn append_message(json: &Value) {
    if let Value::Array(ref list) = json["AddMsgList"] {

        let self_name = {
            let wechat = WECHAT.read().unwrap();
            wechat.user_name().to_owned()
        };

        let account_ptr = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
        let gc = (*account_ptr).gc;

        for msg in list {
            let msg_type = msg["MsgType"].as_i64().unwrap();
            // 51 is wechat init message
            if msg_type == 51 {
                continue;
            }

            let content = CString::new(msg["Content"].as_str().unwrap()).unwrap();
            let from = msg["FromUserName"].as_str().unwrap();
            let dest = msg["ToUserName"].as_str().unwrap();
            let to = CString::new(dest).unwrap();
            let time = msg["CreateTime"].as_i64().unwrap();

            println!("chat message: {:?}", msg);

            if dest.starts_with("@@") {
                // got chat room message
                // let chat = conv_chat(dest);
                let from = CString::new(from).unwrap();
                // purple_conv_chat_write(chat, from.as_ptr(), content.as_ptr(), PURPLE_MESSAGE_RECV, time);
            } else {
                if self_name != from {
                    let from = CString::new(from).unwrap();
                    serv_got_im(gc,
                                from.as_ptr(),
                                content.as_ptr(),
                                PURPLE_MESSAGE_RECV,
                                time);
                } else {
                    serv_got_im(gc,
                                to.as_ptr(),
                                content.as_ptr(),
                                PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_WHISPER,
                                time);
                }
            }
        }
    }
}

unsafe fn add_buddy(user: &User) {

    println!("add_buddy: {} ({})", user.nick_name(), user.alias());

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

unsafe fn add_chat() {}

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