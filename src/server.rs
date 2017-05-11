
extern crate std;
extern crate hyper;
extern crate hyper_native_tls;
extern crate regex;
extern crate time;
extern crate crypto;
extern crate multipart;

use self::hyper::Client;
use self::hyper::client::Request;
use self::hyper::header::{SetCookie, Cookie, Headers};
use self::hyper::net::HttpsConnector;
use self::hyper::method::Method;
use self::hyper_native_tls::NativeTlsClient;
use self::regex::Regex;
use self::crypto::md5::Md5;
use self::crypto::digest::Digest;
use self::multipart::client::Multipart;
use self::time::Duration;
use glib_sys;
use libc;
use user::User;
use chatroom::ChatRoom;
use serde_json::Value;
use serde_json::Map;
use pointer::*;
use purple_sys::*;
use message::*;
use std::borrow::Cow;
use std::os::raw::{c_void, c_char, c_uchar, c_int};
use std::io::*;
use std::ffi::{CStr, CString};
use std::ptr::null_mut;
use std::sync::{RwLock, Mutex};
use std::fs::{File, OpenOptions};
use std::thread;
use std::fmt::Debug;
use std::collections::BTreeSet;

macro_rules! cow_replace {
    ( $item:ident, $src:expr, $dest:expr ) => {
        if $item.contains($src) {
            $item = Cow::Owned($item.replace($src, $dest));
        }
    }
}

lazy_static!{
    pub static ref ACCOUNT: RwLock<Pointer> = RwLock::new(Pointer::new());
    static ref VERIFY_HANDLE: Mutex<Pointer> = Mutex::new(Pointer::new());
    // static ref TX: Mutex<Cell<>> = Mutex::new(Cell::new(None));
    // static ref CLT_MSG: (Mutex<Sender<CltMsg>>, Mutex<Receiver<CltMsg>>) =
    //{let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    static ref WECHAT: RwLock<WeChat> = RwLock::new(WeChat::new());
    static ref CLIENT: Client = {
        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        let mut client = Client::with_connector(connector);
        client.set_read_timeout(Some(Duration::seconds(30).to_std().unwrap()));
        client.set_write_timeout(Some(Duration::seconds(30).to_std().unwrap()));

        client
    };
}

// #[derive(Debug)]
// pub enum CltMsg {
// }

struct WeChat {
    uin: String,
    sid: String,
    skey: String,
    pass_ticket: String,
    headers: Headers,
    user_info: Value,
    sync_keys: Value,
    system_time: i64,

    user_list: BTreeSet<User>,
    chat_list: BTreeSet<ChatRoom>,
}

unsafe impl std::marker::Sync for WeChat {}

impl WeChat {
    fn new() -> WeChat {
        let mut headers = Headers::new();
        headers.set_raw("Cookie", vec![vec![]]);
        headers.set_raw("ContentType",
                        vec![b"application/json; charset=UTF-8".to_vec()]);
        headers.set_raw("Host", vec![b"web.wechat.com".to_vec()]);
        headers.set_raw("Referer", vec![b"https://web.wechat.com/".to_vec()]);
        headers.set_raw("Accept",
                        vec![b"application/json, text/plain, */*".to_vec()]);

        WeChat {
            uin: String::new(),
            sid: String::new(),
            skey: String::new(),
            pass_ticket: String::new(),
            headers: headers,
            user_info: Value::Null,
            sync_keys: Value::Null,
            system_time: 0,

            user_list: BTreeSet::new(),
            chat_list: BTreeSet::new(),
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

    fn device_id(&self) -> String {
        format!("e56{}", time_stamp())
    }

    fn system_time(&mut self) -> i64 {
        let t = self.system_time;
        self.system_time += 1;

        t
    }

    fn set_system_time(&mut self, time: &Value) {
        println!("set sys time: {:?}", time);
        self.system_time = time.as_i64().unwrap();
    }

    fn data_ticket(&self) -> Option<&str> {

        let headers = self.headers.get::<Cookie>();
        if headers.is_none() {
            return None;
        }

        for c in headers.unwrap().iter() {
            let mut split = c.split('=');

            if split.next() == Some("webwx_data_ticket") {
                return split.next();
            }
        }

        None
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

        let count = self.sync_keys.as_array().unwrap().len();
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

    fn nick_name(&self) -> &str {
        self.user_info["NickName"].as_str().unwrap()
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
            send_server_message(SrvMsg::AddContact(user.clone()));
        }
    }

    fn append_chat(&mut self, chat: &ChatRoom) {
        if self.chat_list.insert(chat.clone()) {
            send_server_message(SrvMsg::AddGroup(chat.clone()));
        }
    }

    fn set_chat_ptr(&mut self, chat: &ChatRoom, chat_ptr: *mut PurpleChat) {
        if let Some(mut c) = self.chat_list.take(chat) {
            c.set_chat_ptr(chat_ptr as *mut c_void);
            assert!(self.chat_list.insert(c));
        } else {
            println!("set chat ptr error, {:?}", chat);
        }
    }

    fn find_chat_by_token(&self, token: usize) -> Option<&ChatRoom> {
        for ref c in self.chat_list.iter() {
            if c.token() == token {
                return Some(c);
            }
        }

        None
    }

    fn find_chat_by_id(&self, id: &str) -> Option<&ChatRoom> {
        for ref c in self.chat_list.iter() {
            if c.id() == id {
                return Some(c);
            }
        }

        None
    }

    fn find_chat_token(&self, id: &str) -> usize {

        if let Some(c) = self.find_chat_by_id(id) {
            return c.token();
        }

        0
    }

    fn find_chat_ptr(&self, id: &str) -> *mut PurpleChat {

        if let Some(c) = self.find_chat_by_id(id) {
            return c.chat_ptr() as *mut PurpleChat;
        }

        null_mut()
    }

    fn base_data(&self) -> Value {

        let mut base_obj = Map::with_capacity(4);
        base_obj.insert("Uin".to_owned(), json!(self.uin.parse::<usize>().unwrap()));
        base_obj.insert("Sid".to_owned(), json!(self.sid));
        base_obj.insert("Skey".to_owned(), json!(self.skey));
        base_obj.insert("DeviceID".to_owned(), json!(self.device_id()));

        let mut obj = Map::new();
        obj.insert("BaseRequest".to_owned(), Value::Object(base_obj));

        Value::Object(obj)
    }

    fn image_upload_data(&self, img_size: usize, img_md5: &str, dest_name: &str) -> Value {
        let mut value = self.base_data();

        value["UploadType"] = json!(2);
        value["ClientMediaId"] = json!(time_stamp());
        value["TotalLen"] = json!(img_size);
        value["StartPos"] = json!(0);
        value["DataLen"] = json!(img_size);
        value["MediaType"] = json!(4);
        value["FromUserName"] = json!(self.user_name());
        value["ToUserName"] = json!(dest_name);
        value["FileMd5"] = json!(img_md5);

        value
    }

    fn status_notify_data(&self) -> Value {

        let mut value = self.base_data();

        value["Code"] = json!(3);
        value["FromUserName"] = json!(self.user_name());
        value["ToUserName"] = json!(self.user_name());
        value["ClientMsgId"] = json!(time_stamp());

        value
    }

    fn group_info_data(&self, groups: &[String]) -> Value {

        let mut list = vec![];
        for id in groups {
            let item = json!({ "UserName": json!(id),
                               "ChatRoomId": json!("") });
            list.push(item);
        }

        let mut value = self.base_data();
        value["Count"] = json!(groups.len());
        value["List"] = json!(list);

        value
    }

    fn message_check_data(&self) -> Value {

        let mut value = self.base_data();

        value["SyncKey"] = self.sync_key().clone();
        value["rr"] = json!(!time_stamp());

        value
    }

    fn message_send_data(&self, who: &str, content: &str) -> Value {

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
    let url = format!("https://login.web.wechat.com/qrcode/{}", uuid);
    let file_path = save_image(&url);
    // let file_path = save_qr_file(&uuid);

    // start check login thread
    thread::spawn(|| { check_scan(uuid); });
    send_server_message(SrvMsg::ShowVerifyImage(file_path));
}

fn check_scan(uuid: String) {
    let url = format!("https://login.web.wechat.com/cgi-bin/mmwebwx-bin/login?uuid={}&tip={}",
                      uuid,
                      1);
    // TODO: check result
    let _ = get(&url);

    let url = format!("https://login.web.wechat.com/cgi-bin/mmwebwx-bin/login?uuid={}&tip={}",
                      uuid,
                      0);

    let result = get(&url).unwrap();
    let reg = Regex::new(r#"redirect_uri="([^"]+)""#).unwrap();
    let caps = reg.captures(&result).expect("Can't find redirect uri");
    let uri = caps.get(1).unwrap().as_str();

    // scan successful, close dialog
    let vh = VERIFY_HANDLE.lock().unwrap();
    unsafe { purple_request_close(PURPLE_REQUEST_FIELDS, vh.as_ptr()) };

    // webwxnewloginpage
    let url = format!("{}&fun=new&version=v2", uri);
    println!("login with: {}", url);
    let mut response = CLIENT.get(&url).send().expect("login failed");
    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    println!("login result: {}", result);
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
    let data = {
        WECHAT.read().unwrap().base_data()
    };
    let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxinit?lang=zh_CN&pass_ticket={}&skey={}&r={}",
                      pass_ticket,
                      skey, time_stamp());
    let json = post(&url, &data)
        .unwrap()
        .parse::<Value>()
        .expect("init failed");
    {
        let mut wechat = WECHAT.write().unwrap();
        wechat.set_sync_key(&json["SyncKey"]);
        wechat.set_system_time(&json["SystemTime"]);
        wechat.set_user_info(&json);
    }

    let ref contact_list = json["ContactList"].as_array().unwrap();
    let mut groups = vec![];
    for contact in *contact_list {
        let name = contact["UserName"].as_str().unwrap();
        if name.starts_with("@@") {
            groups.push(name.to_owned());
        }
    }

    // status notify
    let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxstatusnotify?pass_ticket={}",
                      pass_ticket);
    let data = WECHAT.read().unwrap().status_notify_data();
    // TODO: check result
    let r = post(&url, &data).unwrap();
    println!("status notify result: {}", r);

    // get contact
    let (url, data) = {
        let wechat = WECHAT.read().unwrap();
        let url = format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/\
                           webwxbatchgetcontact?type=ex&r={}&pass_ticket={}",
        time_stamp(), wechat.pass_ticket());
        let data = wechat.group_info_data(&groups[..]);

        (url, data)
    };
    let result = post(&url, &data);
    let json = result.unwrap().parse::<Value>().unwrap();
    let ref groups = json["ContactList"].as_array().unwrap();
    if groups.len() != 0 {
        let mut wechat = WECHAT.write().unwrap();
        for group in *groups {
            wechat.append_chat(&ChatRoom::from_json(group));
        }
    }

    // refersh current user name
    unsafe {
        let uname = CString::new(WECHAT.read().unwrap().user_name()).unwrap();
        let alias = CString::new(WECHAT.read().unwrap().nick_name()).unwrap();
        println!("update account info: {:?} {:?}", uname, alias);
        purple_account_set_username(ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount,
                                    uname.as_ptr());
        purple_account_set_alias(ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount,
                                 alias.as_ptr());
    }

    // fetch contact list
    thread::spawn(|| fetch_contact());

    // start message check loop
    thread::spawn(|| sync_check());
}

fn time_stamp() -> i64 {
    time::get_time().sec * 1000
}

fn fetch_contact() {
    let url = {
        let wechat = WECHAT.read().unwrap();
        format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/\
                 webwxgetcontact?pass_ticket={}&skey={}&r={}&seq=0",
        wechat.pass_ticket(), wechat.skey(), time_stamp())
    };

    let result = get(url).unwrap().parse::<Value>().unwrap();
    let ref member_list = result["MemberList"].as_array().unwrap();

    // yield event loop
    {
        send_server_message(SrvMsg::YieldEvent);
    }

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

    let mut client = Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
    client.set_read_timeout(Some(time::Duration::seconds(30).to_std().unwrap()));

    // let uid
    loop {
        let url = {
            let t = {
                WECHAT.write().unwrap().system_time()
            };
            let wechat = WECHAT.read().unwrap();
            let ts = time_stamp();
            format!("https://webpush.web.wechat.com/cgi-bin/mmwebwx-bin/synccheck\
            ?sid={}&uin={}&skey={}&deviceid={}&synckey={}&r={}&_={}",
                    wechat.sid(),
                    wechat.uin(),
                    wechat.skey(),
                    wechat.device_id(),
                    wechat.sync_key_str(),
                    ts,
                    t)
        };

        println!("sync check url: {}", url);

        let mut response = match client.get(&url).headers(headers.clone()).send() {
            Ok(response) => response,
            Err(_) => continue,
        };

        if !response.status.is_success() {
            continue;
        }

        let mut result = String::new();
        response.read_to_string(&mut result).unwrap();

        let reg = Regex::new(r#"retcode:"(\d+)",selector:"(\d+)""#).unwrap();
        let caps = reg.captures(&result).unwrap();
        let retcode: isize = caps.get(1).unwrap().as_str().parse().unwrap();
        let selector: isize = caps.get(2).unwrap().as_str().parse().unwrap();
        println!("{} = {} - {}", result, retcode, selector);

        // logout
        if retcode == 1100 || retcode == 1101 {
            break;
        }

        // no new message.
        if selector == 0 {
            continue;
        }

        check_new_message();
    }

    // show logout message
    let m = "You are already logged in other devices.\nplease logout and restart pidgin."
        .to_owned();
    let m = SrvMsg::ShowMessageBox(m);
    send_server_message(m);
}

unsafe fn show_message_box(message: &str) {

    let message = CString::new(message).unwrap();
    let title = CString::new("Wechat Notice").unwrap();
    let ok_txt = CString::new("Ok").unwrap();
    let cancel_txt = CString::new("Cancel").unwrap();

    let group = purple_request_field_group_new(title.as_ptr());
    let field = purple_request_field_new(message.as_ptr(),
                                         message.as_ptr(),
                                         PURPLE_REQUEST_FIELD_NONE);
    purple_request_field_group_add_field(group, field);
    let fields = purple_request_fields_new();
    purple_request_fields_add_group(fields, group);
    purple_request_fields(null_mut(), // handle
                          title.as_ptr(), // title
                          message.as_ptr(), // primary
                          null_mut(), // secondary
                          fields, // fields
                          ok_txt.as_ptr(), // ok_text
                          Some(ok_cb), // ok_cb
                          cancel_txt.as_ptr(), // cancel_text
                          None, // cancel_cb
                          null_mut(), // account
                          null_mut(), // who
                          null_mut(), // conv
                          null_mut()); // user_data
}

pub unsafe extern "C" fn send_chat(_: *mut PurpleConnection,
                                   id: i32,
                                   msg: *const c_char,
                                   _: PurpleMessageFlags)
                                   -> c_int {

    let msg_cstr = CStr::from_ptr(msg).to_string_lossy().into_owned();

    let wechat = WECHAT.read().unwrap();
    if let Some(chat) = wechat.find_chat_by_token(id as usize) {

        let chat_id = chat.id();
        let conv = conversion(PURPLE_CONV_TYPE_CHAT, &chat_id);
        let chat = purple_conversation_get_chat_data(conv);
        let self_name = CString::new(wechat.user_name()).unwrap();

        purple_conv_chat_write(chat,
                               self_name.as_ptr(),
                               msg,
                               PURPLE_MESSAGE_SEND,
                               time_stamp() / 1000);

        send_message(&chat_id, &msg_cstr);
    } else {
        println!("chat not found {}", msg_cstr);
    }

    0
}

unsafe fn upload_picture(id: usize, dest_name: &str) {
    println!("upload pic for ID = {}", id);

    let image = purple_imgstore_find_by_id(id as i32);
    let img_name = CStr::from_ptr(purple_imgstore_get_filename(image)).to_string_lossy();
    let img_size = purple_imgstore_get_size(image);
    let img_data = purple_imgstore_get_data(image);

    let mut raw_data = Vec::<u8>::with_capacity(img_size);
    raw_data.set_len(img_size);
    std::ptr::copy(img_data as *const c_uchar, raw_data.as_mut_ptr(), img_size);

    let mut md5 = Md5::new();
    md5.input(&raw_data[..]);
    let md5 = md5.result_str();

    let req_json = WECHAT
        .read()
        .unwrap()
        .image_upload_data(img_size, &md5, dest_name);
    let req_json = req_json.to_string();

    println!("filename: {}", img_name);
    println!("filesize: {}", img_size);
    println!("filemd5: {}", md5);
    println!("req_json: {}", req_json);

    let ticket = WECHAT.read().unwrap().data_ticket().unwrap().to_owned();
    let ticket = ticket.replace("%2B", "+");
    let ticket = ticket.replace("%2F", "/");
    println!("ticket = {:?}", ticket);

    let pass_ticket = WECHAT.read().unwrap().pass_ticket().to_owned();
    let pass_ticket = pass_ticket.replace("%2B", "+");
    let pass_ticket = pass_ticket.replace("%2F", "/");

    let url = "https://file.web.wechat.com/cgi-bin/mmwebwx-bin/webwxuploadmedia?f=json";
    let mut req = Request::with_connector(Method::Post,
                                          url.parse().unwrap(),
                                          &HttpsConnector::new(NativeTlsClient::new().unwrap()))
            .unwrap();
    req.headers_mut()
        .set_raw("Referer", vec![b"https://web.wechat.com/".to_vec()]);
    req.headers_mut()
        .set_raw("Origin", vec![b"https://web.wechat.com/".to_vec()]);
    req.headers_mut().set_raw("Accept", vec![b"*/*".to_vec()]);
    req.headers_mut()
        .set_raw("Accept-Language",
                 vec![b"zh-CN,en-US;q=0.7,en;q=0.3".to_vec()]);
    req.headers_mut()
        .set_raw("Accept-Encoding", vec![b"gzip, deflate, br".to_vec()]);
    req.headers_mut()
        .set_raw("User-Agent",
                 vec![b"Mozilla/5.0 (Windows NT 10.0; WOW64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/57.0.2987.110 Safari/537.36".to_vec()]);

    let mut multipart = Multipart::from_request_sized(req).unwrap();
    multipart.write_text("id", "WU_FILE_0").unwrap();
    multipart.write_text("name", &img_name).unwrap();
    multipart.write_text("type", "image/png").unwrap();
    multipart
        .write_text("lastModifiedDate",
                    "Tue May 09 2017 14:21:02 GMT+0800 (CST)")
        .unwrap();
    multipart.write_text("size", img_size.to_string()).unwrap();
    multipart.write_text("mediatype", "pic").unwrap();
    multipart
        .write_text("uploadmediarequest", req_json)
        .unwrap();
    multipart.write_text("webwx_data_ticket", ticket).unwrap();
    multipart.write_text("pass_ticket", pass_ticket).unwrap();
    multipart.write_file("filename", "/tmp/1.png").unwrap();

    let mut response = multipart.send().unwrap();
    let mut res = String::new();
    response.read_to_string(&mut res).unwrap();
    println!("{}", res);
    println!("{:?}", response);
}

fn preprocess_send_message<'a>(msg: &'a str, dest_name: &str) -> Option<Cow<'a, str>> {

    let r = Regex::new(r#"<IMG ID="(\d+)">"#).unwrap();

    if let Some(caps) = r.captures(msg) {
        unsafe {
            upload_picture(caps.get(1).unwrap().as_str().parse().unwrap(), dest_name);
        }
        return None;
    }

    let mut ret = Cow::Borrowed(msg);
    cow_replace!(ret, "<br>", "\n");
    cow_replace!(ret, "&quot;", r#"""#);
    cow_replace!(ret, "&lt;", "<");
    cow_replace!(ret, "&gt;", ">");

    Some(ret)
}

fn send_message(who: &str, msg: &str) {

    println!("send_message: {}: {}", who, msg);

    let msg = match preprocess_send_message(msg, who) {
        Some(m) => m,
        _ => return,
    };

    let (url, data) = {
        let wechat = WECHAT.read().unwrap();
        let url = format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxsendmsg?\
                           pass_ticket={}", wechat.pass_ticket());
        let data = wechat.message_send_data(who, &*msg);

        (url, data)
    };

    // TODO: check result.
    thread::spawn(move || { let _ = post(&url, &data); });
}

pub unsafe extern "C" fn send_im(_: *mut PurpleConnection,
                                 who: *const c_char,
                                 msg: *const c_char,
                                 _: PurpleMessageFlags)
                                 -> c_int {

    let who = CStr::from_ptr(who).to_string_lossy().into_owned();
    let msg = CStr::from_ptr(msg).to_string_lossy().into_owned();

    send_message(&who, &msg);

    1
}

fn check_new_message() {

    let (headers, url, data) = {
        let wechat = WECHAT.read().unwrap();
        let url = format!("https://web.wechat.\
                       com/cgi-bin/mmwebwx-bin/webwxsync?sid={}&skey={}&pass_ticket={}",
                      wechat.sid(),
                      wechat.skey(),
                      wechat.pass_ticket());

        (wechat.headers(), url, wechat.message_check_data())
    };

    let mut client = Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap()));
    client.set_read_timeout(Some(Duration::seconds(5).to_std().unwrap()));
    client.set_write_timeout(Some(Duration::seconds(5).to_std().unwrap()));

    let mut response = match client
              .post(&url)
              .headers(headers)
              .body(&data.to_string())
              .send() {
        Ok(response) => response,
        _ => return,
    };

    if !response.status.is_success() {
        println!("response not success, {}", response.status);
        return;
    }

    let mut buf = String::new();
    response.read_to_string(&mut buf).unwrap();

    // refersh sync check key
    let json: Value = buf.parse().expect("message response parse error");
    {
        WECHAT.write().unwrap().set_sync_key(&json["SyncCheckKey"]);
    }

    send_server_message(SrvMsg::MessageReceived(json));
}

fn regex_cap<'a>(c: &'a str, r: &str) -> &'a str {
    let reg = Regex::new(r).unwrap();
    let caps = reg.captures(&c).unwrap();

    caps.get(1).unwrap().as_str()
}

fn get_uuid() -> String {
    let url = "https://login.web.wechat.com/jslogin?appid=wx782c26e4c19acffb&redirect_uri=\
               https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxnewloginpage&fun=new&lang=zh_CN";
    let result = get(&url).unwrap();

    let reg = Regex::new(r#"uuid\s*=\s*"([-\w=]+)""#).unwrap();
    let caps = reg.captures(&result).unwrap();

    caps.get(1).unwrap().as_str().to_owned()
}

fn get<T: AsRef<str> + Debug>(url: T) -> Option<String> {

    let headers = {
        WECHAT.read().unwrap().headers()
    };

    println!("get: {:?}", url);
    let mut response = match CLIENT.get(url.as_ref()).headers(headers).send() {
        Ok(response) => response,
        Err(e) => {
            println!("Err: {:?}", e);
            return None;
        }
    };

    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    if result.len() > 500 && response.status.is_success() {
        println!("result: {}", &result[0..300]);
    } else {
        println!("result: {}", result);
    }

    Some(result)
}

fn post<U: AsRef<str> + Debug>(url: U, data: &Value) -> Option<String> {

    let headers = {
        WECHAT.read().unwrap().headers()
    };
    println!("post: {:?}\nheaders:{:?}\npost_data: {:?}",
             url,
             headers,
             data);
    let mut response = match CLIENT
              .post(url.as_ref())
              .headers(headers)
              .body(&data.to_string())
              .send() {
        Ok(response) => response,
        Err(e) => {
            println!("Err: {:?}", e);
            return None;
        }
    };

    let mut result = String::new();
    response.read_to_string(&mut result).unwrap();
    if result.len() > 500 && response.status.is_success() {
        println!("result: {}", &result[0..300]);
    } else {
        println!("result: {}", result);
    }

    Some(result)
}

unsafe extern "C" fn check_srv(_: *mut c_void) -> c_int {

    let rx = SRV_MSG.1.lock().unwrap();

    while let Ok(m) = rx.try_recv() {
        match m {
            SrvMsg::ShowMessageBox(m) => show_message_box(&m),
            SrvMsg::ShowVerifyImage(path) => show_verify_image(path),
            SrvMsg::AddContact(user) => add_buddy(&user),
            SrvMsg::AddGroup(chat) => add_group(&chat),
            SrvMsg::MessageReceived(json) => append_message(&json),
            SrvMsg::AppendImageMessage(id, json) => append_image_message(id, &json),
            SrvMsg::RefreshChatMembers(chat) => refresh_chat_members(&chat),
            SrvMsg::YieldEvent => break,
        }
    }

    1
}

unsafe fn refresh_chat_members(chat: &str) {
    let wechat = WECHAT.read().unwrap();
    let chat = wechat.find_chat_by_id(chat).unwrap();
    let conv = conversion(PURPLE_CONV_TYPE_CHAT, &chat.id());
    let conv_chat = purple_conversation_get_chat_data(conv);

    for member in chat.members() {
        let id = CString::new(member.user_name()).unwrap();
        purple_conv_chat_add_user(conv_chat, id.as_ptr(), null_mut(), PURPLE_CBFLAGS_NONE, 0);
    }
}

unsafe fn add_group(chat: &ChatRoom) {

    println!("add group: {} {}", chat.alias(), chat.token());

    let free = std::mem::transmute::<unsafe extern "C" fn(*mut std::os::raw::c_void),
                                     unsafe extern "C" fn(*mut libc::c_void)>(g_free);

    let hash_table = glib_sys::g_hash_table_new_full(Some(glib_sys::g_str_hash),
                                                     Some(glib_sys::g_str_equal),
                                                     Some(free),
                                                     Some(free)) as
                     *mut GHashTable;

    let id_key = CString::new("ChatId").unwrap();
    let id = chat.id_cstring();
    g_hash_table_insert(hash_table,
                        g_strdup(id_key.as_ptr()) as *mut c_void,
                        g_strdup(id.as_ptr()) as *mut c_void);

    let account = {
        ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount
    };

    let chat_ptr = purple_chat_new(account as *mut PurpleAccount, id.as_ptr(), hash_table);

    {
        WECHAT.write().unwrap().set_chat_ptr(chat, chat_ptr);
    }

    let group_name = CString::new("Wechat Groups").unwrap();
    let group = purple_find_group(group_name.as_ptr());
    purple_blist_add_chat(chat_ptr, group, null_mut());
    purple_blist_node_set_flags(chat_ptr as *mut PurpleBlistNode,
                                PURPLE_BLIST_NODE_FLAG_NO_SAVE);

    // set chat alias if not empty
    if !chat.alias().is_empty() {
        let alias = chat.alias_cstring();
        purple_blist_alias_chat(chat_ptr, alias.as_ptr());
    }
}

pub unsafe extern "C" fn find_blist_chat(_: *mut PurpleAccount,
                                         name: *const c_char)
                                         -> *mut PurpleChat {
    let name = CStr::from_ptr(name);

    let chat_ptr = WECHAT
        .read()
        .unwrap()
        .find_chat_ptr(name.to_string_lossy().to_mut());

    chat_ptr
}

pub fn find_chat_token(id: &str) -> usize {
    let token = WECHAT.read().unwrap().find_chat_token(id);

    token
}

unsafe fn conversion(conv_type: PurpleConversationType, name: &str) -> *mut PurpleConversation {
    let name_cstr = CString::new(name).unwrap();
    let account = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
    let conv = purple_find_conversation_with_account(conv_type, name_cstr.as_ptr(), account);

    if conv != null_mut() {
        return conv;
    }

    // conv is null
    if conv_type == PURPLE_CONV_TYPE_IM {
        return purple_conversation_new(conv_type, account, name_cstr.as_ptr());
    }

    assert!(name.starts_with("@@"));
    let token = find_chat_token(name);

    // search chat
    let gc = (*account).gc;
    let conv = purple_find_chat(gc, token as i32);
    println!("purple_find_chat for token {}, result = {:?}", token, conv);
    if conv != null_mut() {
        return conv;
    }

    // join chat
    let account = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
    println!("join chat {:?}, token = {}", name, token);
    serv_got_joined_chat(purple_account_get_connection(account),
                         token as i32,
                         name_cstr.as_ptr());

    // find again
    let conv = purple_find_conversation_with_account(conv_type, name_cstr.as_ptr(), account);
    // ensure not nullptr
    assert!(conv != null_mut());
    // add members
    send_server_message(SrvMsg::YieldEvent);
    send_server_message(SrvMsg::RefreshChatMembers(name.to_owned()));

    conv
}

fn save_image(url: &str) -> String {

    let headers = {
        WECHAT.read().unwrap().headers()
    };

    let mut response = CLIENT.get(url).headers(headers).send().unwrap();
    let mut result = Vec::new();
    response.read_to_end(&mut result).unwrap();

    println!("fetched image: {} {} {}", url, response.status, result.len());

    save_file(&result)
}

fn save_file(buf: &[u8]) -> String {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("/tmp/img.jpg")
        .unwrap();
    file.write_all(buf).unwrap();

    "/tmp/img.jpg".to_owned()
}

fn append_image_message(id: i32, msg: &Value) {

    let src = msg["FromUserName"].as_str().unwrap();
    let dest = msg["ToUserName"].as_str().unwrap();
    let time = msg["CreateTime"].as_i64().unwrap();
    let img_msg = format!(r#"<IMG ID="{}">"#, id);

    if src.starts_with("@@") {
        let content = msg["Content"].as_str().unwrap();

        // split content to find real sender
        let regex = Regex::new(r#"^(@\w+):.*$"#).unwrap();
        let caps = regex.captures(content).unwrap();
        let sender = caps.get(1).unwrap().as_str();

        append_purple_chat_message(src, dest, sender, &img_msg, time);
    } else if dest.starts_with("@@") {
        append_purple_chat_message(src, dest, dest, &img_msg, time);
    } else {
        append_purple_im_message(src, dest, &img_msg, time);
    }
}

unsafe fn process_image_message(msg: &Value) {

    let msg_id = msg["MsgId"].as_str().unwrap();
    let url = format!("https://web.wechat.com/cgi-bin/mmwebwx-bin/webwxgetmsgimg?&MsgID={}&skey={}",
                       msg_id, WECHAT.read().unwrap().skey());

    let msg = msg.clone();

    thread::spawn(move || {
        let img_path = CString::new(save_image(&url)).unwrap();
        let img = purple_imgstore_new_from_file(img_path.as_ptr());
        let img_data = purple_imgstore_get_data(img);
        let img_size = purple_imgstore_get_size(img);
        let img_filename = purple_imgstore_get_filename(img);

        let id = purple_imgstore_add_with_id(img_data as *mut c_void, img_size, img_filename);

        let sender = SRV_MSG.0.lock().unwrap();
        sender.send(SrvMsg::YieldEvent).unwrap();
        sender.send(SrvMsg::AppendImageMessage(id, msg)).unwrap()
    });
}

unsafe fn process_emoji_image(msg: &Value) {

    let content = msg["Content"].as_str().unwrap();
    let regex = Regex::new(r#"cdnurl\s*=\s*"([^"]+)""#).unwrap();

    let caps = match regex.captures(content) {
        Some(caps) => caps,
        None => return append_text_message(msg),
    };

    let url = caps.get(1).unwrap().as_str().to_owned();
    let msg = msg.clone();

    thread::spawn(move || {

        let mut headers = Headers::new();
        headers.set_raw("Host", vec![b"emoji.qpic.cn".to_vec()]);

        let mut response = CLIENT.get(&url).headers(headers).send().unwrap();
        let mut result = Vec::new();
        response.read_to_end(&mut result).unwrap();

        println!("fetch image: {} {} {}", url, response.status, result.len());

        let img_path = CString::new(save_file(&result)).unwrap();
        let img = purple_imgstore_new_from_file(img_path.as_ptr());
        let img_data = purple_imgstore_get_data(img);
        let img_size = purple_imgstore_get_size(img);
        let img_filename = purple_imgstore_get_filename(img);

        let id = purple_imgstore_add_with_id(img_data as *mut c_void, img_size, img_filename);

        let sender = SRV_MSG.0.lock().unwrap();
        sender.send(SrvMsg::YieldEvent).unwrap();
        sender.send(SrvMsg::AppendImageMessage(id, msg)).unwrap()
    });
}

unsafe fn append_text_message(msg: &Value) {

    let content = msg["Content"].as_str().unwrap();
    let content_cstring = CString::new(content).unwrap();
    let src = msg["FromUserName"].as_str().unwrap();
    let from = CString::new(src).unwrap();
    let dest = msg["ToUserName"].as_str().unwrap();
    let time = msg["CreateTime"].as_i64().unwrap();

    if src.starts_with("@@") {
        let conv = conversion(PURPLE_CONV_TYPE_CHAT, src);
        let chat = purple_conversation_get_chat_data(conv);

        // split content to find real sender
        let regex = Regex::new(r#"^(@\w+):(?:<br/>)*(.*)$"#).unwrap();
        match regex.captures(content) {
            Some(caps) => {
                let sender = CString::new(caps.get(1).unwrap().as_str()).unwrap();
                let content = CString::new(caps.get(2).unwrap().as_str()).unwrap();

                purple_conv_chat_write(chat,
                                       sender.as_ptr(),
                                       content.as_ptr(),
                                       PURPLE_MESSAGE_RECV,
                                       time);
            }
            None => {
                purple_conv_chat_write(chat,
                                       from.as_ptr(),
                                       content_cstring.as_ptr(),
                                       PURPLE_MESSAGE_RECV | PURPLE_MESSAGE_SYSTEM,
                                       time);
            }
        }
    } else if dest.starts_with("@@") {
        let conv = conversion(PURPLE_CONV_TYPE_CHAT, dest);
        let chat = purple_conversation_get_chat_data(conv);
        purple_conv_chat_write(chat,
                               from.as_ptr(),
                               content_cstring.as_ptr(),
                               PURPLE_MESSAGE_SEND,
                               time);
    } else {
        let self_name = {
            let wechat = WECHAT.read().unwrap();
            wechat.user_name().to_owned()
        };

        if self_name != src {
            let account_ptr = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;
            let gc = (*account_ptr).gc;

            serv_got_im(gc,
                        from.as_ptr(),
                        content_cstring.as_ptr(),
                        PURPLE_MESSAGE_RECV,
                        time);
        } else {
            let conv = conversion(PURPLE_CONV_TYPE_IM, dest);
            let im = purple_conversation_get_im_data(conv);
            purple_conv_im_write(im,
                                 from.as_ptr(),
                                 content_cstring.as_ptr(),
                                 PURPLE_MESSAGE_SEND,
                                 time);
        }
    }
}

fn append_purple_chat_message(from: &str, dest: &str, sender: &str, content: &str, time: i64) {

    let content_cstring = CString::new(content).unwrap();
    let from_cstring = CString::new(from).unwrap();
    let has_img = content.contains("<IMG ID=");
    let send_flag = {
        if has_img {
            PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_IMAGES
        } else {
            PURPLE_MESSAGE_SEND
        }
    };
    let recv_flag = {
        if has_img {
            PURPLE_MESSAGE_RECV | PURPLE_MESSAGE_IMAGES
        } else {
            PURPLE_MESSAGE_RECV
        }
    };

    if from.starts_with("@@") {
        let sender_cstring = CString::new(sender).unwrap();
        unsafe {
            let conv = conversion(PURPLE_CONV_TYPE_CHAT, from);
            let chat = purple_conversation_get_chat_data(conv);

            purple_conv_chat_write(chat,
                                   sender_cstring.as_ptr(),
                                   content_cstring.as_ptr(),
                                   recv_flag,
                                   time);
        }
    } else if dest.starts_with("@@") {
        unsafe {
            let conv = conversion(PURPLE_CONV_TYPE_CHAT, dest);
            let chat = purple_conversation_get_chat_data(conv);
            purple_conv_chat_write(chat,
                                   from_cstring.as_ptr(),
                                   content_cstring.as_ptr(),
                                   send_flag,
                                   time);
        }
    }
}

fn append_purple_im_message(from: &str, dest: &str, content: &str, time: i64) {

    let content_cstring = CString::new(content).unwrap();
    let from_cstring = CString::new(from).unwrap();
    let has_img = content.contains("<IMG ID=");
    let send_flag = {
        if has_img {
            PURPLE_MESSAGE_SEND | PURPLE_MESSAGE_IMAGES
        } else {
            PURPLE_MESSAGE_SEND
        }
    };
    let recv_flag = {
        if has_img {
            PURPLE_MESSAGE_RECV | PURPLE_MESSAGE_IMAGES
        } else {
            PURPLE_MESSAGE_RECV
        }
    };

    let self_name = {
        let wechat = WECHAT.read().unwrap();
        wechat.user_name().to_owned()
    };

    if self_name != from {
        let account_ptr = ACCOUNT.read().unwrap().as_ptr() as *mut PurpleAccount;

        unsafe {
            let gc = (*account_ptr).gc;
            serv_got_im(gc,
                        from_cstring.as_ptr(),
                        content_cstring.as_ptr(),
                        recv_flag,
                        time);
        }
    } else {
        unsafe {
            let conv = conversion(PURPLE_CONV_TYPE_IM, dest);
            let im = purple_conversation_get_im_data(conv);
            purple_conv_im_write(im,
                                 from_cstring.as_ptr(),
                                 content_cstring.as_ptr(),
                                 send_flag,
                                 time);
        }
    }
}

fn append_message(json: &Value) {

    if let Value::Array(ref list) = json["AddMsgList"] {
        for msg in list {
            println!("got message =========================\n {}", json);

            let msg_type = msg["MsgType"].as_i64().unwrap();
            match msg_type {
                // 51 is wechat init message
                51 => continue,
                3 => unsafe { process_image_message(msg) },
                47 => unsafe { process_emoji_image(msg) },
                _ => unsafe { append_text_message(msg) },
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
    purple_prpl_got_user_status(account,
                                user_name.as_ptr(),
                                available.as_ptr(),
                                null_mut() as *mut c_void);
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
    let verify_handle = purple_request_fields(purple_account_get_connection(account) as
                                              *mut c_void, // handle
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

    assert!(verify_handle != null_mut());
    let mut vh = VERIFY_HANDLE.lock().unwrap();
    vh.set(verify_handle);
}

extern "C" fn ok_cb() {}

