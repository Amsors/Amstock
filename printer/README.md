# Epson TM-T82III 标签渲染与打印验证

此目录包含由 Amstock Rust 后端调用的 Python 标签渲染与打印模块。Python 只负责
把后端已经准备好的文本绘制成 ESC/POS 栅格图像，不读取项目数据库，
不解析编号，不展开容器层级，也不对子项进行排序。

目标打印机为 Epson TM-T82III（203 dpi），纸宽 75 mm。程序使用打印机的
576 点（约 72 mm）可打印宽度，在纸张两侧保留少量物理边距。

## 四种样式

| 样式 | 可用对象 | 主体长度 | 设计 |
| --- | --- | ---: | --- |
| A1 | 物品、容器 | 20 mm / 160 点 | 左侧放大的二维码，右侧完整编号 |
| A2 | 物品、容器 | 30 mm / 240 点 | A1 上方增加 10 mm 名称区 |
| B1 | 仅容器 | 动态 | A2 下方以双列紧凑列出子项编号 |
| B2 | 仅容器 | 动态 | A2 下方每行列出一个子项的编号和名称 |

A1 的二维码在不改变 20 mm 栏目高度的前提下尽量占满高度，并利用二维码图片
外侧的画布留白共同构成扫描静区。A2/B1/B2 复用同一个二维码和编号栏目。

A2/B1/B2 的顶部名称优先缩放为单行。单行仍放不下时改为两行；完整名称超出
两行容量时，第二行末尾用 `....` 截断。B1 每行放两个编号；B2 的编号列使用
固定宽度，所有子项编号均使用等宽粗体；名称占用剩余宽度，过长的子项名称也会
以 `....` 截断。这样可避免为少量内容预留固定空白，并控制 B1/B2 的走纸长度。

B1 子项区约为 `1.25 mm + ceil(子项数 / 2) × 3.75 mm`；B2 子项区约为
`1.25 mm + 子项数 × 4.25 mm`。两者都只在有子项时追加该区域。

B1/B2 的 `children` 可以是直接子项，也可以是 Rust 后端递归展开后的全部后代；
渲染器不理解两者的区别。最终显示内容及顺序完全以数组为准。

## JSON 交互契约

请求是一个 UTF-8 JSON 对象：

```json
{
  "schema_version": 1,
  "style": "B2",
  "kind": "container",
  "identifier": "C-02-10-000128",
  "name": "电子元件一号收纳箱",
  "children": [
    {
      "identifier": "R-01-01-000129",
      "name": "100 Ω 金属膜电阻"
    },
    {
      "identifier": "C-02-11-000131",
      "name": "贴片电容小盒"
    }
  ]
}
```

字段约束：

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `schema_version` | integer | 必须为 `1`，用于以后兼容升级 |
| `style` | string | `A1`、`A2`、`B1`、`B2` 之一；解析时不区分大小写 |
| `kind` | string | `item` 或 `container`；B1/B2 必须为 `container` |
| `identifier` | string | 顶部二维码和编号区显示的非空文本 |
| `name` | string | A2/B1/B2 必填且非空；A1 可省略 |
| `children` | array | B1/B2 可传；A1/A2 必须省略或传空数组 |
| `children[].identifier` | string | B1/B2 均必填且非空 |
| `children[].name` | string | B2 必填且非空；B1 可省略且不会绘制 |

边界约定：

- Rust 后端负责选出“子容器 + 子物品”、确定是否递归、生成最终文字并排序。
- Python 保持 `children` 原顺序，只做参数校验、文字测量、截断和绘制。
- QR 内容就是顶层 `identifier` 原文；子项不生成二维码。
- B1/B2 的空 `children` 合法，此时输出退化为 30 mm 的 A2 画面。
- JSON 中额外字段目前会被忽略，便于契约向后兼容；已有字段的类型和约束仍会
  严格校验。

可直接使用 [examples](examples) 中的四份示例请求。

## Rust 调用入口

`amstock_printer.py` 是面向后端的稳定单次调用接口。它从 stdin 读取一份上述 JSON，
以退出码表示成功或失败，并只在 stdout 输出一份 JSON 结果；错误诊断写入 stderr。

```bash
cat examples/b2.json | .venv/bin/python amstock_printer.py \
  --mode preview --output-dir output --no-open

cat examples/a1.json | .venv/bin/python amstock_printer.py \
  --mode printer --host 192.168.31.114 --port 9100
```

`preview` 会保存 PNG，并在未指定 `--no-open` 时调用系统看图程序；`printer` 会连接
真实打印机。Rust 后端启动参数负责选择这两种模式。

由于目标打印机在栅格图像上方固定吐出约 13 mm 的空白纸，`printer` 模式会在发送
前自动裁掉图像顶部的全部纯白行，使首行栅格即包含打印内容，不再叠加软件侧的顶部
余量。该处理只影响实机发送：PNG 预览仍保留 A1/A2 的标准 20/30 mm 画布；硬件产生
的空白也足以充当 A1 二维码上方的扫描静区。

## 环境

虚拟环境和依赖已经安装。以后需要重建时执行：

```bash
cd printer
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
```

系统需要安装支持中文的 Noto Sans CJK 或 Droid Sans Fallback 字体。

## JSON 预览（dry run）

`preview_label.py` 读取 JSON、输出 PNG，并默认调用当前操作系统的看图程序打开：

```bash
cd printer
.venv/bin/python preview_label.py examples/a1.json
.venv/bin/python preview_label.py examples/a2.json
.venv/bin/python preview_label.py examples/b1.json
.venv/bin/python preview_label.py examples/b2.json
```

默认输出到 `output/<JSON文件名>-preview.png`。可指定路径，或在 CI/无桌面环境中
禁止自动打开：

```bash
.venv/bin/python preview_label.py examples/b2.json \
  --output output/my-b2.png --no-open
```

## 原有 A1 打印验证命令

原命令继续保留，用于直接验证 A1 和真实打印机：

```bash
# 生成并打开 A1 预览
.venv/bin/python label_printer.py preview M-01-85-862390 --kind item

# 只检测 TCP 9100 连接，不打印
.venv/bin/python label_printer.py check

# 发送 A1 并部分切纸
.venv/bin/python label_printer.py print M-01-85-862390 --kind item
```

如果首次实打不希望切纸，可追加 `--no-cut`。打印机 IP、端口、超时可以在
子命令前覆盖。原命令的 `--length-mm` 也继续保留，仅用于 A1 实机走纸补偿；
JSON 契约中的 A1/A2 主体长度固定为 20/30 mm。

自动切刀与打印头之间存在机械距离，切口到切口的实际长度也受固件设置影响，
因此首次实打后仍应测量并在最终后端打印配置中固化补偿值。

## 测试

测试不会连接或操作真实打印机，也不会打开图片：

```bash
cd printer
.venv/bin/python -m unittest -v
```
