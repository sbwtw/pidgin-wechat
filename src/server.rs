
extern crate std;
extern crate hyper;
extern crate hyper_native_tls;
extern crate regex;

use self::hyper::Client;
use self::hyper::net::HttpsConnector;
use self::hyper_native_tls::NativeTlsClient;
use self::regex::Regex;
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

lazy_static!{
    pub static ref ACCOUNT: RwLock<GlobalPointer> = RwLock::new(GlobalPointer::new());
    // static ref TX: Mutex<Cell<>> = Mutex::new(Cell::new(None));
    static ref CHANNEL: (Mutex<Sender<Message>>, Mutex<Receiver<Message>>) = {let (tx, rx) = channel(); (Mutex::new(tx), Mutex::new(rx))};
    static ref WECHAT: Mutex<Wechat> = Mutex::new(Wechat::new());
}

#[derive(Debug)]
pub enum Message {
    ShowVerifyImage(String),
}

struct Wechat {
    client: Client,
}

impl Wechat {
    pub fn new() -> Wechat {

        let ssl = NativeTlsClient::new().unwrap();
        let connector = HttpsConnector::new(ssl);
        let client = Client::with_connector(connector);

        Wechat { client: client }
    }

    fn login(&mut self) {

        let uuid = self.get_uuid();

        let file_path = self.save_qr_file(uuid);

        CHANNEL.0.lock().unwrap().send(Message::ShowVerifyImage(file_path)).unwrap();
    }

    fn get_uuid(&mut self) -> String {
        let url = "https://login.web.wechat.com/jslogin?appid=wx782c26e4c19acffb";
        let mut response = self.client.get(url).send().unwrap();
        let mut result = String::new();
        response.read_to_string(&mut result);

        let reg = Regex::new(r#"uuid\s+=\s+"([\w=]+)""#).unwrap();
        let caps = reg.captures(&result).unwrap();

        caps.get(1).unwrap().as_str().to_owned()
    }

    fn save_qr_file<T: AsRef<str>>(&mut self, url: T) -> String {
        let url = format!("https://login.weixin.qq.com/qrcode/{}", url.as_ref());
        let mut response = self.client.get(&url).send().unwrap();
        let mut result = Vec::new();
        let size = response.read_to_end(&mut result).unwrap();

        let mut s = String::new();
        s.push_str("aaa");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open("/tmp/qr.png")
            .unwrap();
        let size = file.write(&result).unwrap();
        file.flush();

        "/tmp/qr.png".to_owned()
    }

    fn get<T: AsRef<str>>(&mut self, url: T) -> String {
        let mut response = self.client.get(url.as_ref()).send().unwrap();
        let mut result = String::new();
        response.read_to_string(&mut result);

        result
    }
}

unsafe extern "C" fn t(ptr: *mut c_void) -> c_int {

    let rx = CHANNEL.1.lock().unwrap();

    if let Ok(m) = rx.try_recv() {
        debug(format!("GOT: {:#?}", m));

        match m {
            Message::ShowVerifyImage(path) => show_verify_image(path),
        }
    }

    1
}

pub fn login() {

    unsafe {
        purple_timeout_add(1000, Some(t), null_mut());
    }

    std::thread::spawn(|| { WECHAT.lock().unwrap().login(); });
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