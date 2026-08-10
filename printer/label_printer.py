#!/usr/bin/env python3
"""75 mm Epson ESC/POS material/container label feasibility demo."""

from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence

import qrcode
from escpos.printer import Network
from PIL import Image, ImageChops, ImageDraw, ImageFont


DEFAULT_HOST = "192.168.31.114"
DEFAULT_PORT = 9100
DEFAULT_TIMEOUT_SECONDS = 3.0

# TM-T82III prints at 203 dpi. Its 80 mm-class mechanism exposes a 72 mm,
# 576-dot printable area; 75 mm paper therefore retains a small side margin.
DPI = 203
PRINT_WIDTH_DOTS = 576
DEFAULT_LENGTH_MM = 20.0
A1_LENGTH_MM = 20.0
A2_LENGTH_MM = 30.0
NAME_AREA_MM = A2_LENGTH_MM - A1_LENGTH_MM
SUPPORTED_STYLES = ("A1", "A2", "B1", "B2")
SCHEMA_VERSION = 1
CHILD_LIST_PADDING_DOTS = 10
B1_ROW_HEIGHT_DOTS = 30
B2_ROW_HEIGHT_DOTS = 34

FONT_REGULAR_CANDIDATES = (
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
)
FONT_BOLD_CANDIDATES = (
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
)
FONT_MONO_BOLD_CANDIDATES = (
    "/usr/share/fonts/truetype/noto/NotoSansMono-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
    "/usr/share/fonts/opentype/urw-base35/NimbusMonoPS-Bold.otf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Bold.ttf",
)

KIND_NAMES = {
    "item": "物资",
    "container": "容器",
}


@dataclass(frozen=True)
class ChildLabel:
    """One already-resolved and already-ordered child row from the backend."""

    identifier: str
    name: str = ""


@dataclass(frozen=True)
class LabelRequest:
    """Rendering-only input. It deliberately contains no project data model."""

    style: str
    kind: str
    identifier: str
    name: str = ""
    children: tuple[ChildLabel, ...] = ()
    schema_version: int = SCHEMA_VERSION


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


def _required_text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field} 必须是非空字符串。")
    return value.strip()


def parse_label_request(payload: Mapping[str, Any]) -> LabelRequest:
    """Validate and convert the versioned JSON contract into render input."""
    if not isinstance(payload, Mapping):
        raise ValueError("JSON 顶层必须是对象。")

    version = payload.get("schema_version")
    if (
        not isinstance(version, int)
        or isinstance(version, bool)
        or version != SCHEMA_VERSION
    ):
        raise ValueError(
            f"不支持的 schema_version：{version!r}，当前仅支持 {SCHEMA_VERSION}。"
        )

    style = _required_text(payload.get("style"), "style").upper()
    if style not in SUPPORTED_STYLES:
        raise ValueError(
            f"未知样式：{style!r}，可选值为 {', '.join(SUPPORTED_STYLES)}。"
        )

    kind = _required_text(payload.get("kind"), "kind").lower()
    if kind not in KIND_NAMES:
        raise ValueError(f"未知类型：{kind!r}，可选值为 item、container。")

    identifier = _required_text(payload.get("identifier"), "identifier")
    name_value = payload.get("name", "")
    if not isinstance(name_value, str):
        raise ValueError("name 必须是字符串。")
    name = " ".join(name_value.split())
    if style != "A1" and not name:
        raise ValueError(f"{style} 样式需要非空的 name。")

    children_value = payload.get("children", [])
    if not isinstance(children_value, list):
        raise ValueError("children 必须是数组。")
    if style in ("A1", "A2") and children_value:
        raise ValueError(f"{style} 样式不接受 children；请使用 B1 或 B2。")
    if style in ("B1", "B2") and kind != "container":
        raise ValueError(f"{style} 样式仅适用于 container。")

    children: list[ChildLabel] = []
    for index, child_value in enumerate(children_value):
        field = f"children[{index}]"
        if not isinstance(child_value, Mapping):
            raise ValueError(f"{field} 必须是对象。")
        child_identifier = _required_text(
            child_value.get("identifier"), f"{field}.identifier"
        )
        child_name_value = child_value.get("name", "")
        if not isinstance(child_name_value, str):
            raise ValueError(f"{field}.name 必须是字符串。")
        child_name = " ".join(child_name_value.split())
        if style == "B2" and not child_name:
            raise ValueError(f"B2 样式需要非空的 {field}.name。")
        children.append(ChildLabel(child_identifier, child_name))

    return LabelRequest(
        schema_version=version,
        style=style,
        kind=kind,
        identifier=identifier,
        name=name,
        children=tuple(children),
    )


def load_label_request(path: Path) -> LabelRequest:
    """Load one UTF-8 JSON request file."""
    with path.open("r", encoding="utf-8") as source:
        payload = json.load(source)
    return parse_label_request(payload)


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


def fitted_font_or_none(
    draw: ImageDraw.ImageDraw,
    text: str,
    font_path: str,
    max_width: int,
    max_size: int,
    min_size: int,
) -> ImageFont.FreeTypeFont | None:
    """Return a one-line font, or None when text must be wrapped."""
    for size in range(max_size, min_size - 1, -1):
        font = ImageFont.truetype(font_path, size)
        if draw.textlength(text, font=font) <= max_width:
            return font
    return None


def truncate_text(
    draw: ImageDraw.ImageDraw,
    text: str,
    font: ImageFont.FreeTypeFont,
    max_width: int,
    suffix: str = "....",
) -> str:
    """Truncate text to a measured pixel width and append the specified suffix."""
    if draw.textlength(text, font=font) <= max_width:
        return text
    if draw.textlength(suffix, font=font) > max_width:
        return ""
    low, high = 0, len(text)
    while low < high:
        middle = (low + high + 1) // 2
        if draw.textlength(text[:middle] + suffix, font=font) <= max_width:
            low = middle
        else:
            high = middle - 1
    return text[:low].rstrip() + suffix


def layout_name_lines(
    draw: ImageDraw.ImageDraw,
    name: str,
    font_path: str,
    max_width: int,
) -> tuple[list[str], ImageFont.FreeTypeFont]:
    """Prefer one line; otherwise return two balanced, measured lines."""
    one_line_font = fitted_font_or_none(
        draw, name, font_path, max_width, max_size=36, min_size=24
    )
    if one_line_font is not None:
        return [name], one_line_font

    font = ImageFont.truetype(font_path, 27)
    fitting_splits: list[tuple[float, int]] = []
    for split in range(1, len(name)):
        first = name[:split].rstrip()
        second = name[split:].lstrip()
        first_width = draw.textlength(first, font=font)
        second_width = draw.textlength(second, font=font)
        if first_width <= max_width and second_width <= max_width:
            fitting_splits.append((abs(first_width - second_width), split))

    if fitting_splits:
        _, split = min(fitting_splits)
        return [name[:split].rstrip(), name[split:].lstrip()], font

    split = 1
    for candidate in range(1, len(name) + 1):
        if draw.textlength(name[:candidate], font=font) > max_width:
            break
        split = candidate
    first = name[:split].rstrip()
    second = truncate_text(draw, name[split:].lstrip(), font, max_width)
    return [first, second], font


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
    """Build a crisp QR image whose quiet zone is completed by canvas whitespace."""
    qr = qrcode.QRCode(
        version=None,
        error_correction=qrcode.constants.ERROR_CORRECT_M,
        box_size=1,
        # Two internal modules plus the whitespace around the pasted image
        # provide an effective quiet zone while allowing a larger QR symbol.
        border=2,
    )
    qr.add_data(data)
    qr.make(fit=True)

    modules_with_border = qr.modules_count + 4
    box_size = max_side // modules_with_border
    if box_size < 3:
        raise ValueError("编号内容过长，二维码模块小于 3 点，无法可靠热敏打印。")

    qr_image = qr.make_image(
        fill_color="black",
        back_color="white",
    ).convert("L")
    side = modules_with_border * box_size
    return qr_image.resize((side, side), Image.Resampling.NEAREST)


def _draw_a1_section(
    canvas: Image.Image,
    identifier: str,
    top: int,
    height: int,
) -> None:
    """Draw the original QR + identifier layout into a section of a canvas."""
    draw = ImageDraw.Draw(canvas)

    # The QR image, including its internal quiet zone, uses almost the full
    # 20 mm section. The remaining canvas whitespace completes the quiet zone.
    outer_margin = 2
    qr_max_side = height - outer_margin * 2
    qr_image = make_qr(identifier, qr_max_side)
    qr_x = outer_margin + (qr_max_side - qr_image.width) // 2
    qr_y = top + (height - qr_image.height) // 2
    canvas.paste(qr_image, (qr_x, qr_y))

    right_left = outer_margin + qr_max_side + mm_to_dots(2)
    right_right = PRINT_WIDTH_DOTS - outer_margin
    right_width = right_right - right_left
    if right_width < 120:
        raise ValueError("当前标签长度使二维码占用过宽，右侧编号区域不足。")

    bold_path = find_font(FONT_BOLD_CANDIDATES)
    number_font = fitted_font(
        draw,
        identifier,
        bold_path,
        max_width=right_width,
        max_size=min(64, height * 35 // 100),
        min_size=20,
    )

    paste_centered_text(
        draw,
        (
            right_left,
            top + outer_margin,
            right_right,
            top + height - outer_margin,
        ),
        identifier,
        number_font,
    )


def _draw_name_section(canvas: Image.Image, name: str, height: int) -> None:
    draw = ImageDraw.Draw(canvas)
    margin_x = mm_to_dots(1.5)
    max_width = PRINT_WIDTH_DOTS - margin_x * 2
    regular_path = find_font(FONT_REGULAR_CANDIDATES)
    lines, font = layout_name_lines(draw, name, regular_path, max_width)

    if len(lines) == 1:
        paste_centered_text(
            draw,
            (margin_x, 2, PRINT_WIDTH_DOTS - margin_x, height - 3),
            lines[0],
            font,
        )
    else:
        line_height = max(
            draw.textbbox((0, 0), line, font=font)[3]
            - draw.textbbox((0, 0), line, font=font)[1]
            for line in lines
        )
        gap = 2
        block_height = line_height * 2 + gap
        y = max(1, (height - block_height) // 2)
        for line in lines:
            paste_centered_text(
                draw,
                (margin_x, y, PRINT_WIDTH_DOTS - margin_x, y + line_height),
                line,
                font,
            )
            y += line_height + gap

    draw.line((margin_x, height - 1, PRINT_WIDTH_DOTS - margin_x, height - 1), 0)


def _render_a2_base(request: LabelRequest, total_height: int) -> Image.Image:
    canvas = Image.new("L", (PRINT_WIDTH_DOTS, total_height), 255)
    name_height = mm_to_dots(NAME_AREA_MM)
    _draw_name_section(canvas, request.name, name_height)
    _draw_a1_section(canvas, request.identifier, name_height, mm_to_dots(A1_LENGTH_MM))
    return canvas


def _draw_b1_children(
    canvas: Image.Image,
    children: Sequence[ChildLabel],
    top: int,
) -> None:
    if not children:
        return
    draw = ImageDraw.Draw(canvas)
    mono_bold_path = find_font(FONT_MONO_BOLD_CANDIDATES)
    font = ImageFont.truetype(mono_bold_path, 28)
    margin_x = mm_to_dots(1.25)
    padding_y = 5
    gutter = 12
    row_height = B1_ROW_HEIGHT_DOTS
    cell_width = (PRINT_WIDTH_DOTS - margin_x * 2 - gutter) // 2
    draw.line((margin_x, top, PRINT_WIDTH_DOTS - margin_x, top), fill=0)

    for index, child in enumerate(children):
        row, column = divmod(index, 2)
        left = margin_x + column * (cell_width + gutter)
        y_top = top + padding_y + row * row_height
        text = truncate_text(draw, child.identifier, font, cell_width - 8)
        box = draw.textbbox((0, 0), text, font=font)
        text_height = box[3] - box[1]
        y = y_top + (row_height - text_height) // 2 - box[1]
        draw.text((left + 4, y), text, fill=0, font=font)

    divider_x = margin_x + cell_width + gutter // 2
    rows = (len(children) + 1) // 2
    draw.line(
        (divider_x, top + 5, divider_x, top + padding_y + rows * row_height),
        fill=0,
    )


def _draw_b2_children(
    canvas: Image.Image,
    children: Sequence[ChildLabel],
    top: int,
) -> None:
    if not children:
        return
    draw = ImageDraw.Draw(canvas)
    mono_bold_path = find_font(FONT_MONO_BOLD_CANDIDATES)
    regular_path = find_font(FONT_REGULAR_CANDIDATES)
    number_font = ImageFont.truetype(mono_bold_path, 24)
    name_font = ImageFont.truetype(regular_path, 24)
    margin_x = mm_to_dots(1.25)
    padding_y = 5
    row_height = B2_ROW_HEIGHT_DOTS
    number_width = 210
    gap = 10
    name_left = margin_x + number_width + gap
    name_width = PRINT_WIDTH_DOTS - margin_x - name_left
    draw.line((margin_x, top, PRINT_WIDTH_DOTS - margin_x, top), fill=0)

    for index, child in enumerate(children):
        row_top = top + padding_y + index * row_height
        number = truncate_text(draw, child.identifier, number_font, number_width - 8)
        name = truncate_text(draw, child.name, name_font, name_width - 4)
        for text, font, left in (
            (number, number_font, margin_x + 4),
            (name, name_font, name_left),
        ):
            box = draw.textbbox((0, 0), text, font=font)
            text_height = box[3] - box[1]
            y = row_top + (row_height - text_height) // 2 - box[1]
            draw.text((left, y), text, fill=0, font=font)
        if index + 1 < len(children):
            line_y = row_top + row_height
            draw.line((margin_x, line_y, PRINT_WIDTH_DOTS - margin_x, line_y), fill=0)

    divider_x = margin_x + number_width + gap // 2
    draw.line(
        (
            divider_x,
            top + 5,
            divider_x,
            top + padding_y + len(children) * row_height,
        ),
        fill=0,
    )


def render_request(request: LabelRequest) -> Image.Image:
    """Render a validated A1/A2/B1/B2 request as a monochrome raster image."""
    a1_height = mm_to_dots(A1_LENGTH_MM)
    if request.style == "A1":
        canvas = Image.new("L", (PRINT_WIDTH_DOTS, a1_height), 255)
        _draw_a1_section(canvas, request.identifier, 0, a1_height)
    else:
        base_height = mm_to_dots(A2_LENGTH_MM)
        if request.style == "B1" and request.children:
            rows = (len(request.children) + 1) // 2
            total_height = (
                base_height
                + CHILD_LIST_PADDING_DOTS
                + rows * B1_ROW_HEIGHT_DOTS
            )
        elif request.style == "B2" and request.children:
            total_height = (
                base_height
                + CHILD_LIST_PADDING_DOTS
                + len(request.children) * B2_ROW_HEIGHT_DOTS
            )
        else:
            total_height = base_height

        canvas = _render_a2_base(request, total_height)
        if request.style == "B1":
            _draw_b1_children(canvas, request.children, base_height)
        elif request.style == "B2":
            _draw_b2_children(canvas, request.children, base_height)

    # ESC/POS is a one-bit device. Explicit conversion avoids grey
    # antialiasing surprises in both preview and print output.
    return canvas.convert("1", dither=Image.Dither.NONE)


def render_label(
    identifier: str,
    kind: str,
    length_mm: float = DEFAULT_LENGTH_MM,
) -> Image.Image:
    """Render the legacy A1 layout, retaining its optional calibration length."""
    identifier = identifier.strip()
    if not identifier:
        raise ValueError("编号不能为空。")
    if kind not in KIND_NAMES:
        raise ValueError(f"未知类型：{kind}")
    if not 20 <= length_mm <= 100:
        raise ValueError("标签长度必须在 20–100 mm 之间。")

    height = mm_to_dots(length_mm)
    canvas = Image.new("L", (PRINT_WIDTH_DOTS, height), 255)
    _draw_a1_section(canvas, identifier, 0, height)
    return canvas.convert("1", dither=Image.Dither.NONE)


def open_preview(output: Path) -> None:
    """Ask the desktop operating system to open an image without blocking."""
    if sys.platform == "win32":
        os.startfile(output)  # type: ignore[attr-defined]
        return
    command = (
        ["open", str(output)]
        if sys.platform == "darwin"
        else ["xdg-open", str(output)]
    )
    subprocess.Popen(
        command,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )


def save_preview(
    image: Image.Image,
    output: Path,
    *,
    open_after_save: bool = True,
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output, dpi=(DPI, DPI))
    if open_after_save:
        open_preview(output)


def check_connection(host: str, port: int, timeout: float) -> None:
    with socket.create_connection((host, port), timeout=timeout):
        pass


def prepare_print_image(image: Image.Image) -> Image.Image:
    """Remove every fully blank row before the first printed pixel.

    The target printer unavoidably leaves about 13 mm of paper above the
    raster image.  Keeping the renderer's own leading whitespace would add a
    second top margin to that hardware margin.  Previews retain their standard
    dimensions; only the raster sent to the physical printer is trimmed.

    The hardware margin is considerably larger than the QR quiet-zone
    requirement, so it safely replaces the whitespace removed above an A1 QR
    code.  Horizontal whitespace is deliberately left untouched.
    """
    grayscale = image.convert("L")
    ink_bounds = ImageChops.invert(grayscale).getbbox()
    if ink_bounds is None or ink_bounds[1] == 0:
        return image
    return image.crop((0, ink_bounds[1], image.width, image.height))


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
        printer.image(prepare_print_image(image), impl="bitImageRaster")
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
            help="标签走纸方向长度（默认：20 mm）",
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
