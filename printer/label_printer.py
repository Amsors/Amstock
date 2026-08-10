#!/usr/bin/env python3
"""75 mm Epson ESC/POS material/container label feasibility demo."""

from __future__ import annotations

import argparse
import socket
import sys
from pathlib import Path

import qrcode
from escpos.printer import Network
from PIL import Image, ImageDraw, ImageFont


DEFAULT_HOST = "192.168.31.114"
DEFAULT_PORT = 9100
DEFAULT_TIMEOUT_SECONDS = 3.0

# TM-T82III prints at 203 dpi. Its 80 mm-class mechanism exposes a 72 mm,
# 576-dot printable area; 75 mm paper therefore retains a small side margin.
DPI = 203
PRINT_WIDTH_DOTS = 576
DEFAULT_LENGTH_MM = 30.0

FONT_REGULAR_CANDIDATES = (
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
)
FONT_BOLD_CANDIDATES = (
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
)

KIND_NAMES = {
    "item": "物资",
    "container": "容器",
}


def mm_to_dots(mm: float) -> int:
    """Convert millimetres to 203 dpi dots."""
    return round(mm / 25.4 * DPI)


def find_font(candidates: tuple[str, ...]) -> str:
    for candidate in candidates:
        if Path(candidate).is_file():
            return candidate
    raise FileNotFoundError(
        "未找到中文字体；请安装 Noto Sans CJK，或修改脚本中的 FONT_*_CANDIDATES。"
    )


def fitted_font(
    draw: ImageDraw.ImageDraw,
    text: str,
    font_path: str,
    max_width: int,
    max_size: int,
    min_size: int = 18,
) -> ImageFont.FreeTypeFont:
    """Return the largest font that keeps text on one line."""
    for size in range(max_size, min_size - 1, -1):
        font = ImageFont.truetype(font_path, size)
        box = draw.textbbox((0, 0), text, font=font)
        if box[2] - box[0] <= max_width:
            return font
    raise ValueError(f"编号过长，无法放入标签：{text!r}")


def paste_centered_text(
    draw: ImageDraw.ImageDraw,
    area: tuple[int, int, int, int],
    text: str,
    font: ImageFont.FreeTypeFont,
) -> None:
    left, top, right, bottom = area
    box = draw.textbbox((0, 0), text, font=font)
    width = box[2] - box[0]
    height = box[3] - box[1]
    x = left + (right - left - width) // 2 - box[0]
    y = top + (bottom - top - height) // 2 - box[1]
    draw.text((x, y), text, fill=0, font=font)


def make_qr(data: str, max_side: int) -> Image.Image:
    """Build a crisp QR image with an ISO-style four-module quiet zone."""
    qr = qrcode.QRCode(
        version=None,
        error_correction=qrcode.constants.ERROR_CORRECT_M,
        box_size=1,
        border=4,
    )
    qr.add_data(data)
    qr.make(fit=True)

    modules_with_border = qr.modules_count + 8
    box_size = max_side // modules_with_border
    if box_size < 3:
        raise ValueError("编号内容过长，二维码模块小于 3 点，无法可靠热敏打印。")

    qr_image = qr.make_image(
        fill_color="black",
        back_color="white",
    ).convert("L")
    side = modules_with_border * box_size
    return qr_image.resize((side, side), Image.Resampling.NEAREST)


def render_label(
    identifier: str,
    kind: str,
    length_mm: float = DEFAULT_LENGTH_MM,
) -> Image.Image:
    """Render one monochrome 75 x length_mm label for ESC/POS raster output."""
    identifier = identifier.strip()
    if not identifier:
        raise ValueError("编号不能为空。")
    if kind not in KIND_NAMES:
        raise ValueError(f"未知类型：{kind}")
    if not 20 <= length_mm <= 100:
        raise ValueError("标签长度必须在 20–100 mm 之间。")

    height = mm_to_dots(length_mm)
    canvas = Image.new("L", (PRINT_WIDTH_DOTS, height), 255)
    draw = ImageDraw.Draw(canvas)

    outer_margin = max(8, mm_to_dots(1.5))
    qr_max_side = height - outer_margin * 2
    qr_image = make_qr(identifier, qr_max_side)
    qr_x = outer_margin + (qr_max_side - qr_image.width) // 2
    qr_y = (height - qr_image.height) // 2
    canvas.paste(qr_image, (qr_x, qr_y))

    divider_x = outer_margin + qr_max_side + mm_to_dots(1.5)
    right_left = divider_x + mm_to_dots(2)
    right_right = PRINT_WIDTH_DOTS - outer_margin
    right_width = right_right - right_left
    if right_width < 120:
        raise ValueError("当前标签长度使二维码占用过宽，右侧编号区域不足。")

    draw.line(
        (divider_x, outer_margin, divider_x, height - outer_margin),
        fill=0,
        width=2,
    )

    regular_path = find_font(FONT_REGULAR_CANDIDATES)
    bold_path = find_font(FONT_BOLD_CANDIDATES)
    title_font = ImageFont.truetype(regular_path, min(30, max(20, height // 8)))
    number_font = fitted_font(
        draw,
        identifier,
        bold_path,
        max_width=right_width,
        max_size=min(48, height // 5),
        min_size=20,
    )

    title_bottom = height * 44 // 100
    paste_centered_text(
        draw,
        (right_left, outer_margin, right_right, title_bottom),
        f"{KIND_NAMES[kind]}编号",
        title_font,
    )
    draw.line(
        (right_left, title_bottom, right_right, title_bottom),
        fill=0,
        width=1,
    )
    paste_centered_text(
        draw,
        (right_left, title_bottom + 1, right_right, height - outer_margin),
        identifier,
        number_font,
    )

    # ESC/POS is a one-bit device. Explicit conversion keeps preview and print
    # output identical and avoids grey antialiasing surprises.
    return canvas.convert("1", dither=Image.Dither.NONE)


def save_preview(image: Image.Image, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output, dpi=(DPI, DPI))


def check_connection(host: str, port: int, timeout: float) -> None:
    with socket.create_connection((host, port), timeout=timeout):
        pass


def print_label(
    image: Image.Image,
    host: str,
    port: int,
    timeout: float,
    cut: bool,
) -> None:
    printer: Network | None = None
    try:
        # TM-T20II is the closest bundled python-escpos profile: 203 dpi,
        # 576-dot media width and the same standard Epson ESC/POS raster/cut
        # capabilities needed by this proof of concept.
        printer = Network(
            host,
            port=port,
            timeout=timeout,
            profile="TM-T20II",
        )
        printer.image(image, impl="bitImageRaster")
        if cut:
            # feed=False avoids python-escpos's additional six-line feed. The
            # Epson feed-and-partial-cut command still advances to the cutter.
            printer.cut(mode="PART", feed=False)
    finally:
        if printer is not None:
            printer.close()


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Epson TM-T82III 物资/容器标签可行性验证工具"
    )
    parser.add_argument("--host", default=DEFAULT_HOST, help="打印机 IP")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="RAW TCP 端口")
    parser.add_argument(
        "--timeout",
        type=float,
        default=DEFAULT_TIMEOUT_SECONDS,
        help="网络超时秒数",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("check", help="只检测 TCP 连接，不发送打印数据")

    for command, help_text in (
        ("preview", "生成 PNG 标签预览"),
        ("print", "生成并发送一张真实标签"),
    ):
        child = subparsers.add_parser(command, help=help_text)
        child.add_argument("identifier", help="二维码及右侧文字使用的完整编号")
        child.add_argument(
            "--kind",
            choices=tuple(KIND_NAMES),
            default="item",
            help="item=物资，container=容器（默认：item）",
        )
        child.add_argument(
            "--length-mm",
            type=float,
            default=DEFAULT_LENGTH_MM,
            help="标签走纸方向长度（默认：30 mm）",
        )
        child.add_argument(
            "--output",
            type=Path,
            default=Path("output/label-preview.png"),
            help="预览 PNG 路径",
        )
        if command == "print":
            child.add_argument("--no-cut", action="store_true", help="打印后不切纸")

    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "check":
            check_connection(args.host, args.port, args.timeout)
            print(f"连接成功：{args.host}:{args.port}")
            return 0

        image = render_label(args.identifier, args.kind, args.length_mm)
        save_preview(image, args.output)
        print(
            f"预览已保存：{args.output} "
            f"({image.width}×{image.height} dots, {DPI} dpi)"
        )

        if args.command == "print":
            print_label(
                image,
                args.host,
                args.port,
                args.timeout,
                cut=not args.no_cut,
            )
            print(f"打印数据已发送：{args.host}:{args.port}")
        return 0
    except (OSError, ValueError, qrcode.exceptions.DataOverflowError) as error:
        print(f"错误：{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
