#!/usr/bin/env python3
"""Render a label request JSON file, save it as PNG, and open the preview."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import qrcode

from label_printer import DPI, load_label_request, render_request, save_preview


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="从 JSON 文件生成 Amstock 标签预览图并用系统看图程序打开"
    )
    parser.add_argument("json_file", type=Path, help="UTF-8 编码的标签请求 JSON 文件")
    parser.add_argument(
        "--output",
        type=Path,
        help="输出 PNG 路径（默认：output/<JSON 文件名>-preview.png）",
    )
    parser.add_argument(
        "--no-open",
        action="store_true",
        help="只生成图片，不调用系统看图程序（适合测试或无桌面环境）",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    output = args.output or Path("output") / f"{args.json_file.stem}-preview.png"

    try:
        request = load_label_request(args.json_file)
        image = render_request(request)
        save_preview(image, output, open_after_save=not args.no_open)
        length_mm = image.height / DPI * 25.4
        print(
            f"{request.style} 预览已保存：{output} "
            f"({image.width}×{image.height} dots, {length_mm:.1f} mm, {DPI} dpi)"
        )
        return 0
    except (
        OSError,
        ValueError,
        json.JSONDecodeError,
        qrcode.exceptions.DataOverflowError,
    ) as error:
        print(f"错误：{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
