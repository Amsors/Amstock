# Amstock Android

这是 `https://amstock.amsors.top` 的薄 WebView 客户端。业务页面、数据、图片和 Cloudflare Access 鉴权仍由服务器负责；APK 只提供 Android 容器以及拍照/选图能力。因此一般的前端与后端更新不需要重新发布 APK。

## 已实现

- 在 App 内完成 Cloudflare Access 邮箱验证码登录并持久化会话 Cookie。
- 网页的图片上传控件会提供系统相机和系统文件选择器；拍摄结果直接交给网页上传。
- 顶部设置按钮可将“选择图片”固定为直接拍照、直接选择文件，或每次询问（默认）。设置只保存在本机。
- 相机新拍的照片会先在本地修正方向并压缩到约 2 MiB 以内，再交给网页上传；文件选择器选取的图片不会被改写。
- 主域名与 `*.cloudflareaccess.com` 登录页留在 App 内，其他网页、电话和邮件链接交给系统应用。
- 仅允许 HTTPS，禁止明文流量；SSL 证书错误不会被绕过。
- 支持网页历史返回、加载进度、断网提示、重试和携带登录 Cookie 的下载。

拍照和压缩产生的临时文件位于 App 缓存目录，上传之后可由系统清理。App 不建立业务数据库，也不把服务端数据同步到手机。下载文件位于 App 的外部文件目录 `Android/data/com.amsors.amstock_app/files/Download`。

## 开发构建

在 Android Studio 中打开本目录，选择手机后运行 `app`。命令行验证：

```bash
./gradlew testDebugUnitTest assembleDebug
```

调试 APK 生成在 `app/build/outputs/apk/debug/app-debug.apk`。

## 发布签名

正式长期安装建议在 Android Studio 使用 **Build > Generate Signed App Bundle or APK > APK** 创建并妥善备份签名密钥。以后只有使用同一个 `applicationId` 和同一把密钥签名，并提高 `versionCode`，才能覆盖安装升级。

只有以下变化通常需要重新发布 APK：原生相机/文件选择逻辑改变、入口域名或 Cloudflare Access 登录域名策略改变、Android 系统兼容性调整。普通网站前后端发布不需要更新 APK。
