extern crate purple_sys;
extern crate glib_sys;
extern crate libc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_json;

mod plugin_pointer;
mod server;
mod user;

use purple_sys::*;
use std::os::raw::{c_void, c_char};
use std::ptr::null_mut;
use std::boxed::Box;
use std::ffi::CString;
use std::sync::RwLock;
use plugin_pointer::GlobalPointer;
use server::ACCOUNT;
use server::send_im;

const TRUE: i32 = 1;
const FALSE: i32 = 0;

lazy_static!{
    static ref PLUGIN: RwLock<GlobalPointer> = RwLock::new(GlobalPointer::new());
    static ref ICON_FILE: CString = CString::new("icq").unwrap();
    static ref WECHAT_CATEGORY: CString = CString::new("Wechat").unwrap();
}

fn append_item(list: *mut GList, item: *mut c_void) -> *mut GList {
    unsafe {
        glib_sys::g_list_append(list as *mut glib_sys::GList, item as *mut libc::c_void) as
        *mut GList
    }
}

extern "C" fn list_icon(_: *mut PurpleAccount, _: *mut PurpleBuddy) -> *const c_char {
    ICON_FILE.as_ptr()
}

extern "C" fn status_types(_: *mut PurpleAccount) -> *mut GList {

    let mut list: *mut GList = null_mut();

    let available = CString::new("available").unwrap();
    let available_name = CString::new("Available").unwrap();
    let offline = CString::new("offline").unwrap();
    let offline_name = CString::new("Offline").unwrap();
    let nick = CString::new("nick").unwrap();

    let status = unsafe {
        purple_status_type_new_with_attrs(PURPLE_STATUS_AVAILABLE,
                                          available.as_ptr(),
                                          available_name.as_ptr(),
                                          TRUE,
                                          TRUE,
                                          FALSE,
                                          nick.as_ptr(),
                                          nick.as_ptr(),
                                          purple_value_new(PURPLE_TYPE_STRING),
                                          null_mut() as *mut c_void)
    };
    list = append_item(list, status as *mut c_void);

    let status = unsafe {
        purple_status_type_new_with_attrs(PURPLE_STATUS_OFFLINE,
                                          offline.as_ptr(),
                                          offline_name.as_ptr(),
                                          TRUE,
                                          TRUE,
                                          FALSE,
                                          nick.as_ptr(),
                                          nick.as_ptr(),
                                          purple_value_new(PURPLE_TYPE_STRING),
                                          null_mut() as *mut c_void)
    };
    list = append_item(list, status as *mut c_void);

    list
}

unsafe extern "C" fn login(account: *mut PurpleAccount) {

    ACCOUNT.write().unwrap().set(account as *mut c_void);

    purple_connection_set_state(purple_account_get_connection(account), PURPLE_CONNECTED);

    // clear old buddy list
    // let mut node = purple_blist_get_buddies();
    // while node != null_mut() {
    //     let buddy = (*node).data as *mut PurpleBuddy;

    //     debug("info", format!("{:?}, {:?}", node, (*node).next));
    //     if (*buddy).account == account {
    //         purple_blist_remove_buddy(buddy);
    //     }

    //     node = (*node).next;
    // }

    let group = purple_group_new(WECHAT_CATEGORY.as_ptr());
    (*group).node.flags = PURPLE_BLIST_NODE_FLAG_NO_SAVE;
    purple_blist_add_group(group, null_mut());

    let chat_group = CString::new("Wechat Groups").unwrap();
    let group = purple_group_new(chat_group.as_ptr());
    (*group).node.flags = PURPLE_BLIST_NODE_FLAG_NO_SAVE;
    purple_blist_add_group(group, null_mut());

    let name = CString::new("name").unwrap();
    let buddy = purple_buddy_new(account, name.as_ptr(), null_mut());
    (*buddy).node.flags = PURPLE_BLIST_NODE_FLAG_NO_SAVE;
    purple_blist_add_buddy(buddy, null_mut(), group, null_mut());

    let available = CString::new("available").unwrap();
    purple_prpl_got_user_status(account, name.as_ptr(), available.as_ptr());

    std::thread::spawn(|| { server::login(); });
}

extern "C" fn chat_info(_: *mut PurpleConnection) -> *mut GList {

    let list: *mut GList = null_mut();

    list
}

extern "C" fn chat_info_defaults(_: *mut PurpleConnection, _: *const c_char) -> *mut GHashTable {

    let table: *mut GHashTable = null_mut();

    table
}

extern "C" fn close(_: *mut PurpleConnection) {
}

extern "C" fn buddy_list(gc: *mut PurpleConnection) -> *mut PurpleRoomlist {

    let fields: *mut GList = null_mut();

    let wechat_field_name = CString::new("wechat").unwrap();
    let wechat_field = unsafe {
        purple_roomlist_field_new(PURPLE_ROOMLIST_FIELD_STRING,
                                  wechat_field_name.as_ptr(),
                                  WECHAT_CATEGORY.as_ptr(),
                                  FALSE)
    };
    let fields = append_item(fields, wechat_field as *mut c_void);

    let buddys = unsafe { purple_roomlist_new(purple_connection_get_account(gc)) };

    unsafe {
        purple_roomlist_set_fields(buddys, fields);
    }

    buddys
}

extern "C" fn callback(plugin: *mut PurplePlugin) -> i32 {

    let title = CString::new("hello world").unwrap();
    let content = CString::new("asdasdasdasda").unwrap();

    unsafe {
        purple_notify_message(plugin as *mut c_void,
                              PurpleNotifyMsgType::PURPLE_NOTIFY_MSG_INFO,
                              content.into_raw(),
                              title.into_raw(),
                              null_mut(),
                              None,
                              null_mut());
    }

    TRUE
}

extern "C" fn action_cb(_: *mut PurplePluginAction) {
    let title = CString::new("hello world").unwrap();
    let content = CString::new("asdasdasdasda").unwrap();
    let a = CString::new("asdasdasdasda").unwrap();
    let b = CString::new("asdasdasdasda").unwrap();

    unsafe {

        purple_debug(PurpleDebugLevel::PURPLE_DEBUG_INFO,
                     a.into_raw(),
                     b.into_raw());

        purple_notify_message(PLUGIN.read().unwrap().as_ptr(),
                              PurpleNotifyMsgType::PURPLE_NOTIFY_MSG_INFO,
                              content.into_raw(),
                              title.into_raw(),
                              null_mut(),
                              None,
                              null_mut());
    };
}

extern "C" fn actions(_: *mut PurplePlugin, _: *mut c_void) -> *mut GList {

    let mut list: *mut GList = null_mut();

    let act_name = CString::new("Action").unwrap();

    let action = unsafe { purple_plugin_action_new(act_name.as_ptr(), Some(action_cb)) };
    list = append_item(list, action as *mut c_void);

    list
}

#[no_mangle]
pub extern "C" fn purple_init_plugin(plugin: *mut PurplePlugin) -> i32 {

    // save plugin pointer
    PLUGIN.write().unwrap().set(plugin as *mut c_void);

    let mut info = Box::new(PurplePluginInfo::new());
    let mut extra_info = Box::new(PurplePluginProtocolInfo::new());

    unsafe {

        extra_info.list_icon = Some(list_icon);
        extra_info.status_types = Some(status_types);
        extra_info.login = Some(login);
        extra_info.close = Some(close);
        extra_info.roomlist_get_list = Some(buddy_list);
        extra_info.chat_info = Some(chat_info);
        extra_info.chat_info_defaults = Some(chat_info_defaults);
        extra_info.send_im = Some(send_im);

        info.load = Some(callback);
        info.actions = Some(actions);
        info.extra_info = Box::into_raw(extra_info) as *mut c_void;

        (*plugin).info = Box::into_raw(info);
    };

    unsafe { purple_plugin_register(plugin) }
}