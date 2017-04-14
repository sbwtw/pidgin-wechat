mod purple;

pub use purple::*;
use std::ffi::CString;
use std::ptr::null_mut;

impl PurplePluginInfo {
    pub fn new() -> PurplePluginInfo {

        let id = CString::new("prpl-sbwtw-web_wechat").unwrap();
        let name = CString::new("Wechat").unwrap();
        let version = CString::new("0.0.0").unwrap();
        let summary = CString::new("Web wechat protocol implements").unwrap();
        let description = CString::new("Web wechat protocol implements").unwrap();
        let author = CString::new("sbwtw <sbw@sbw.so>").unwrap();
        let home_page = CString::new("http://blog.sbw.so").unwrap();

        PurplePluginInfo {
            magic: PURPLE_PLUGIN_MAGIC,
            major_version: PURPLE_MAJOR_VERSION,
            minor_version: PURPLE_MINOR_VERSION,
            type_: PurplePluginType::PURPLE_PLUGIN_PROTOCOL,
            ui_requirement: null_mut(),
            flags: 0,
            dependencies: null_mut(),
            priority: PURPLE_PRIORITY_DEFAULT as i32,
            id: id.into_raw(),
            name: name.into_raw(),
            version: version.into_raw(),
            summary: summary.into_raw(),
            description: description.into_raw(),
            author: author.into_raw(),
            homepage: home_page.into_raw(),

            load: None,
            unload: None,
            destroy: None,

            ui_info: null_mut(),
            extra_info: null_mut(),
            prefs_info: null_mut(),

            actions: None,
            _purple_reserved1: None,
            _purple_reserved2: None,
            _purple_reserved3: None,
            _purple_reserved4: None,
        }
    }
}

impl PurpleBuddyIconSpec {
    pub fn new() -> PurpleBuddyIconSpec {

        let formats = CString::new("png,jpg,jpeg,gif").unwrap();

        PurpleBuddyIconSpec {
            format: formats.into_raw(),
            min_width: 0,
            min_height: 0,
            max_width: 1024,
            max_height: 1024,
            max_filesize: 0,
            scale_rules: PURPLE_ICON_SCALE_SEND,
        }
    }
}

impl PurplePluginProtocolInfo {
    pub fn new() -> PurplePluginProtocolInfo {
        PurplePluginProtocolInfo {
            options: OPT_PROTO_NO_PASSWORD | OPT_PROTO_IM_IMAGE | OPT_PROTO_CHAT_TOPIC,

            user_splits: null_mut(),
            protocol_options: null_mut(),

            icon_spec: PurpleBuddyIconSpec::new(),

            list_icon: None,
            list_emblem: None,
            status_text: None,
            tooltip_text: None,
            status_types: None,
            blist_node_menu: None,
            chat_info: None,
            chat_info_defaults: None,
            login: None,
            close: None,
            send_im: None,
            set_info: None,
            send_typing: None,
            get_info: None,
            set_status: None,
            set_idle: None,
            change_passwd: None,

            add_buddy: None,
            add_buddies: None,
            remove_buddy: None,
            remove_buddies: None,
            add_permit: None,
            add_deny: None,
            rem_permit: None,
            rem_deny: None,
            set_permit_deny: None,

            join_chat: None,
            reject_chat: None,
            get_chat_name: None,
            chat_invite: None,
            chat_leave: None,
            chat_whisper: None,
            chat_send: None,
            keepalive: None,
            register_user: None,

            get_cb_info: None,
            get_cb_away: None,
            alias_buddy: None,
            group_buddy: None,
            rename_group: None,
            buddy_free: None,
            convo_closed: None,
            normalize: None,
            set_buddy_icon: None,
            remove_group: None,
            get_cb_real_name: None,
            set_chat_topic: None,
            find_blist_chat: None,
            roomlist_get_list: None,
            roomlist_cancel: None,
            roomlist_expand_category: None,
            can_receive_file: None,
            send_file: None,
            new_xfer: None,
            offline_message: None,

            whiteboard_prpl_ops: null_mut(),

            send_raw: None,
            roomlist_room_serialize: None,
            unregister_user: None,
            send_attention: None,
            get_attention_types: None,

            struct_size: std::mem::size_of::<PurplePluginProtocolInfo>() as u64,

            get_account_text_table: None,
            initiate_media: None,
            get_media_caps: None,
            get_moods: None,
            set_public_alias: None,
            get_public_alias: None,
            add_buddy_with_invite: None,
            add_buddies_with_invite: None,
        }
    }
}