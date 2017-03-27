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

## 项目进展
目前还在技术验证阶段，暂时实现了基本的登录和联系人的消息收发，下一步准备支持群相关的功能。

## 开发进度
- [x] 登录
- [x] 收发文字消息
- [ ] 收发群聊天消息
- [ ] 显示图片消息
- [ ] 发送文件与图片
- [ ] 用户头像
- [ ] 公众号
- [ ] 表情

## 协议
