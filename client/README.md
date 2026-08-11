# USB 扫码展示客户端

`amstock_scanner.py` 直接读取 Linux `evdev` 键盘事件。当扫码枪输入类似
`A-12-34-567890` 的合法完整编号并发送回车后，脚本会用系统默认浏览器打开：

```text
https://amstock.amsors.top/display/A-12-34-567890
```

只使用 Python 标准库，无需安装额外依赖。运行：

```bash
./client/amstock_scanner.py
```

默认设备为 `/dev/amstock_usb_scanner`，默认服务地址为
`https://amstock.amsors.top`。本地开发时可以覆盖地址：

```bash
./client/amstock_scanner.py --base-url http://localhost:43691
```

也可以设置 `AMSTOCK_SCANNER_DEVICE` 和 `AMSTOCK_BASE_URL` 环境变量。
执行 `./client/amstock_scanner.py --help` 可以查看测试模式等参数。

脚本默认通过 `EVIOCGRAB` 独占扫码设备，避免内容同时输入到当前窗口。运行用户必须
对设备具有读取权限；建议通过 udev 规则把实际设备分配给专用用户或 `input` 组，
不要长期使用 root 运行。如果确实不希望独占，可以传入 `--no-grab`，但此时扫码内容
及末尾回车也会发送到当前聚焦的应用。

设备链接必须指向扫码枪的 `/dev/input/event*` 键盘事件节点，不能指向
`/dev/bus/usb/*`、`/dev/hidraw*` 或 USB 总设备节点。脚本启动时会主动检查这一点。

该地址打开不带站点导航和查询控件的独立物资展示页，显示完整物资信息和父容器链条，
并适配大屏与手机。若浏览器跳转到登录页，请先在同一个系统浏览器中登录 Amstock；
展示接口仍然要求有效登录会话。

当前版本只提供前台运行的监听脚本，不安装 systemd 服务，也不配置开机自启。

运行单元测试：

```bash
python3 -m unittest discover -s client -v
```
