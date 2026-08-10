#!/usr/bin/env python3
"""Stable stdin/stdout bridge used by the Amstock Rust backend.

The process accepts one versioned label request as UTF-8 JSON on stdin.  It
prints exactly one machine-readable result object on stdout; diagnostics are
written to stderr and signalled with a non-zero exit status.
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import qrcode

from label_printer import (
    DEFAULT_HOST,
    DEFAULT_PORT,
    DEFAULT_TIMEOUT_SECONDS,
    parse_label_request,
    print_label,
    render_request,
    save_preview,
)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Amstock Rust/Python 标签打印桥接程序")
    parser.add_argument("--mode", choices=("preview", "printer"), required=True)
    parser.add_argument("--output-dir", type=Path, default=Path("output"))
    parser.add_argument("--host", default=DEFAULT_HOST)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT_SECONDS)
    parser.add_argument("--no-open", action="store_true")
    parser.add_argument("--no-cut", action="store_true")
    return parser


def _read_request() -> Any:
    text = sys.stdin.read()
    if not text.strip():
        raise ValueError("stdin 中没有标签请求 JSON。")
    return json.loads(text)


def _preview_path(output_dir: Path, identifier: str, style: str) -> Path:
    safe_identifier = "".join(
        character if character.isalnum() or character in "-_" else "_"
        for character in identifier
    )
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    return output_dir / f"{safe_identifier}-{style.lower()}-{timestamp}.png"


def run(args: argparse.Namespace) -> dict[str, Any]:
    request = parse_label_request(_read_request())
    image = render_request(request)

    if args.mode == "printer":
        print_label(
            image,
            args.host,
            args.port,
            args.timeout,
            cut=not args.no_cut,
        )
        return {
            "schema_version": 1,
            "mode": "printer",
            "style": request.style,
            "identifier": request.identifier,
        }

    output = _preview_path(args.output_dir, request.identifier, request.style)
    save_preview(image, output, open_after_save=not args.no_open)
    return {
        "schema_version": 1,
        "mode": "preview",
        "style": request.style,
        "identifier": request.identifier,
        "output": str(output.resolve()),
    }


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        print(json.dumps(run(args), ensure_ascii=False), flush=True)
        return 0
    except (
        OSError,
        ValueError,
        json.JSONDecodeError,
        qrcode.exceptions.DataOverflowError,
    ) as error:
        print(f"标签处理失败：{error}", file=sys.stderr, flush=True)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
