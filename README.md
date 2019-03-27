# About Pidgin-wechat
`pidgin-wechat` is a protocol plugin for `pidgin/libpurple`. It's based on Web Wechat and supports basic message receiving and picture receiving.

## Screenshot
![pidgin-wechat screenshot](screenshot/2017-04-13-211409_302x579_scrot.png)
![pidgin-wechat screenshot](screenshot/2017-04-17-141051_887x708_scrot.png)

## Build
You can use `cargo` to build it.
```
cargo build --release
```

### Build dependencies
- clang
- libpurple (development package)

> Build dependencies are maybe not fully listed. I have just tested on Archlinux with nightly rust complier.

> You can download the compiled binary file in the [Releases](https://github.com/sbwtw/pidgin-wechat/releases/) page.

## Install
If you build using `cargo`, the binary file is placed at the `target/release` or `target/debug` directory.

To install this plugin, just need to copy the binary to your plugins direcotry and restart `pidgin`.
```
mkdir -p ~/.purple/plugins
cp -f libwechat.so ~/.purple/plugins/
```

## Roadmap
Now this project is still in technical validation. I need to test some wechat features and think about how to implement them in pidgin.

## Progress
- [x] login
- [x] send/receive text message
- [x] send/receive text message in group chat
- [x] receive image message
- [x] receive custom sticker
- [ ] upload file & image
- [ ] buddy icon
- [ ] wechat official accounts
- [ ] rich text message
- [ ] voice message
- [ ] built in sticker
- [ ] system message notify

## Hack
Most common problems:
- login failed, you can see 1101 error code at your terminal, need to relogin.

The log of `pidgin-wechat` is printed to the standard output. Start pidgin in your terminal, and then you can see it.

If it has crashed, you can use `coredumpctl -1 info pidgin` to check core dump info. It's very useful for finding out problems. (Make sure you install the coredump package)

### File list
```
.
├── Cargo.lock
├── Cargo.toml
├── README.md
├── screenshot
│   ├── 2017-04-13-211208_887x708_scrot.png
│   └── 2017-04-13-211409_302x579_scrot.png
└── src
    ├── chatroom.rs                                 struct of chatroom infomation
    ├── message.rs                                  struct of message
    ├── pointer.rs                                  a wrapper of raw C pointer
    ├── server.rs                                   core protocol file
    ├── user.rs                                     struct of user infomation
    └── wechat.rs                                   core pidgin interface file
```

## Q&A
### Why use pidgin/libpurple?
The desktop wechat implementation already exists and maybe has modern UI, but I want to login to all my IMs in the same tool (like IRC, MSN, etc). I choose pidgin because it supports a lot of chat protocols.

### Is red packet/custom sticker supported?
Not supported. Because this program is base on web wechat protocol, if the web wechat doesn't support these features, we can't either.

## LICENSE
This project is licensed under __WTFPL__.
