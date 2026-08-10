# Epson TM-T82III 标签可行性验证

此目录是独立验证程序，不接入 Amstock 前后端。它通过 RAW TCP `9100` 端口向
`192.168.31.114` 发送 ESC/POS 栅格图像。

标签设计为 75 mm 纸宽、30 mm 走纸长度：左侧为包含完整编号的二维码，右侧为
“物资编号”或“容器编号”及完整编号。TM-T82III 为 203 dpi；程序使用 576 点
（约 72 mm）的可打印宽度，在 75 mm 纸上保留少量物理边距。30 mm 换算后为
240 点。

## 环境

虚拟环境和依赖已经安装。以后需要重建时执行：

```bash
cd printer
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
```

## 使用

先生成图片预览，不连接打印机：

```bash
cd printer
.venv/bin/python label_printer.py preview M-01-85-862390 --kind item
.venv/bin/python label_printer.py preview A-00-67-425678 --kind container \
  --output output/container-label.png
```

仅验证打印机的 `9100` 端口是否可达，不打印：

```bash
.venv/bin/python label_printer.py check
```

确认预览后打印并部分切纸：

```bash
.venv/bin/python label_printer.py print M-01-85-862390 --kind item
```

如果首次实打不希望切纸，可追加 `--no-cut`。打印机 IP、端口、超时也可以在
子命令之前覆盖，例如：

```bash
.venv/bin/python label_printer.py --host 192.168.31.114 --timeout 5 print \
  M-01-85-862390 --kind item
```

## 30 mm 长度说明

画布严格按 `round(30 / 25.4 × 203) = 240` 点生成。自动切刀与打印头之间存在
机械距离，ESC/POS 的走纸到切刀动作也受打印机固件和“减少上边距”等设置影响，
因此第一次实打后应测量“切口到切口”的实际长度。如果存在固定偏差，可临时用
`--length-mm` 做校准；正式接入项目时应把实测补偿值固化为打印机配置。

## 测试

测试不会连接或操作真实打印机，只验证图像尺寸和 ESC/POS 数据编码：

```bash
.venv/bin/python -m unittest -v
```
