# Pidgin-wechat
pidgin-wechat 是一个用微信网页版接口实现 pidgin 聊天协议的插件。

## 构建
使用 cargo 可以很方便的构建本项目。
```
cargo build --release
```

## 安装
```
cp target/release/libwechat.so ~/.purple/plugins/
```

## 开发进度
- [x] 登录
- [x] 收发文字消息
- [ ] 显示图片消息
- [ ] 用户头像
- [ ] 群聊天
- [ ] 公众号
- [ ] 表情

## 协议
