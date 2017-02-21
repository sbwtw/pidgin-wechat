
extern crate purple_sys;
extern crate std;

use std::ffi::CString;
use purple_sys::*;

pub fn debug<T: AsRef<str>>(text: T) {

    let category = CString::new("Wechat: ").unwrap();
    let mut content = String::new();
    content.push_str(text.as_ref());
    content.push('\n');
    let content = CString::new(content.as_bytes()).unwrap();

    unsafe { purple_debug(PURPLE_DEBUG_INFO, category.as_ptr(), content.as_ptr()) };
}