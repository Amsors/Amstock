#!/usr/bin/env python3
"""Listen to a Linux USB barcode scanner and open an Amstock display URL."""

from __future__ import annotations

import argparse
import fcntl
import os
import re
import struct
import sys
import time
import webbrowser
from typing import BinaryIO
from urllib.parse import quote, urljoin, urlsplit


DEFAULT_DEVICE = "/dev/amstock_usb_scanner"
DEFAULT_BASE_URL = "https://amstock.amsors.top"
CODE_PATTERN = re.compile(r"^[A-Z]-[0-9]{2}-[0-9]{2}-[0-9]{6}$")

# Linux input-event constants. input_event contains timeval, type, code and value.
INPUT_EVENT = struct.Struct("@llHHI")
EV_KEY = 0x01
KEY_ENTER = 28
KEY_KPENTER = 96
KEY_BACKSPACE = 14
KEY_ESC = 1
KEY_LEFTSHIFT = 42
KEY_RIGHTSHIFT = 54
SHIFT_KEYS = {KEY_LEFTSHIFT, KEY_RIGHTSHIFT}

# _IOW('E', 0x90, int), from linux/input.h.
EVIOCGVERSION = 0x80044501
EVIOCGRAB = 0x40044590

KEY_CHARACTERS = {
    2: "1",
    3: "2",
    4: "3",
    5: "4",
    6: "5",
    7: "6",
    8: "7",
    9: "8",
    10: "9",
    11: "0",
    12: "-",
    16: "q",
    17: "w",
    18: "e",
    19: "r",
    20: "t",
    21: "y",
    22: "u",
    23: "i",
    24: "o",
    25: "p",
    30: "a",
    31: "s",
    32: "d",
    33: "f",
    34: "g",
    35: "h",
    36: "j",
    37: "k",
    38: "l",
    44: "z",
    45: "x",
    46: "c",
    47: "v",
    48: "b",
    49: "n",
    50: "m",
    # Numeric keypad keys, for scanners configured to emit keypad digits.
    71: "7",
    72: "8",
    73: "9",
    74: "-",
    75: "4",
    76: "5",
    77: "6",
    79: "1",
    80: "2",
    81: "3",
    82: "0",
}


def normalize_code(value: str) -> str | None:
    """Return a canonical Amstock code, or None when the scan is invalid."""
    candidate = value.strip().upper()
    return candidate if CODE_PATTERN.fullmatch(candidate) else None


def build_lookup_url(base_url: str, code: str) -> str:
    """Build the frontend's scanner-facing item display URL."""
    base_url = base_url.strip()
    parsed = urlsplit(base_url)
    if parsed.scheme not in {"http", "https"} or not parsed.netloc:
        raise ValueError("服务地址必须是完整的 http:// 或 https:// URL")

    return urljoin(base_url.rstrip("/") + "/", f"display/{quote(code, safe='')}")


def _read_exact(device: BinaryIO, size: int) -> bytes:
    data = bytearray()
    while len(data) < size:
        chunk = device.read(size - len(data))
        if not chunk:
            raise EOFError("扫码设备已断开")
        data.extend(chunk)
    return bytes(data)


def scan_codes(device: BinaryIO):
    """Yield complete, valid codes from a Linux evdev stream."""
    characters: list[str] = []
    shifts_down: set[int] = set()
    invalid_key_seen = False

    while True:
        raw = _read_exact(device, INPUT_EVENT.size)
        _seconds, _microseconds, event_type, key_code, value = INPUT_EVENT.unpack(raw)
        if event_type != EV_KEY:
            continue

        if key_code in SHIFT_KEYS:
            if value == 1:
                shifts_down.add(key_code)
            elif value == 0:
                shifts_down.discard(key_code)
            continue

        # Only key-down events contribute characters. Releases and repeats do not.
        if value != 1:
            continue

        if key_code in {KEY_ENTER, KEY_KPENTER}:
            raw_code = "".join(characters)
            characters.clear()
            shifts_down.clear()
            if not invalid_key_seen:
                code = normalize_code(raw_code)
                if code is not None:
                    yield code
                elif raw_code:
                    print(f"忽略非法扫码内容：{raw_code!r}", file=sys.stderr, flush=True)
            else:
                print("忽略包含不支持按键的扫码内容", file=sys.stderr, flush=True)
            invalid_key_seen = False
            continue

        if key_code == KEY_ESC:
            characters.clear()
            invalid_key_seen = False
            continue

        if key_code == KEY_BACKSPACE:
            if characters:
                characters.pop()
            continue

        character = KEY_CHARACTERS.get(key_code)
        if character is None:
            invalid_key_seen = True
            continue

        if shifts_down and character.isalpha():
            character = character.upper()
        characters.append(character)
        if len(characters) > 64:
            characters.clear()
            invalid_key_seen = True


def open_browser(url: str) -> bool:
    """Open a URL with the desktop's configured default browser."""
    return webbrowser.open(url, new=0, autoraise=True)


class ScannerDeviceError(RuntimeError):
    """The configured path is not a usable scanner event device."""


def verify_evdev(device: BinaryIO, device_path: str) -> None:
    """Fail early when a symlink points at USB/hidraw instead of input/event*."""
    version = bytearray(4)
    try:
        fcntl.ioctl(device.fileno(), EVIOCGVERSION, version, True)
    except OSError as error:
        target = os.path.realpath(device_path)
        raise ScannerDeviceError(
            f"{device_path} -> {target} 不是 Linux evdev 事件设备；"
            "udev 链接必须指向 /dev/input/event*"
            f"（原始错误：{error}）"
        ) from error


def listen_once(
    device_path: str,
    base_url: str,
    *,
    grab: bool,
    dry_run: bool,
    stop_after_one: bool,
) -> None:
    with open(device_path, "rb", buffering=0) as device:
        verify_evdev(device, device_path)
        if grab:
            try:
                fcntl.ioctl(device.fileno(), EVIOCGRAB, 1)
            except OSError as error:
                raise ScannerDeviceError(
                    f"无法独占扫码设备（原始错误：{error}）；"
                    "设备可能已被其他程序独占，必要时可明确使用 --no-grab"
                ) from error

        print(f"正在监听 {device_path}，等待扫码…", flush=True)
        try:
            for code in scan_codes(device):
                url = build_lookup_url(base_url, code)
                print(f"识别到 {code} -> {url}", flush=True)
                if not dry_run and not open_browser(url):
                    print("系统未能启动默认浏览器", file=sys.stderr, flush=True)
                if stop_after_one:
                    return
        finally:
            if grab:
                try:
                    fcntl.ioctl(device.fileno(), EVIOCGRAB, 0)
                except OSError:
                    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="监听 USB 扫码枪，并在默认浏览器打开 Amstock 物资展示页。"
    )
    parser.add_argument(
        "--device",
        default=os.environ.get("AMSTOCK_SCANNER_DEVICE", DEFAULT_DEVICE),
        help=f"evdev 设备路径（默认：{DEFAULT_DEVICE}）",
    )
    parser.add_argument(
        "--base-url",
        default=os.environ.get("AMSTOCK_BASE_URL", DEFAULT_BASE_URL),
        help=f"Amstock 服务根地址（默认：{DEFAULT_BASE_URL}）",
    )
    parser.add_argument(
        "--no-grab",
        action="store_true",
        help="不独占设备（扫码内容也可能被输入当前聚焦的应用）",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="只输出识别结果和 URL，不启动浏览器",
    )
    parser.add_argument(
        "--once",
        action="store_true",
        help="成功识别一次后退出",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        # Validate configuration before waiting for the first scan.
        build_lookup_url(args.base_url, "A-00-00-000000")
    except ValueError as error:
        print(f"配置错误：{error}", file=sys.stderr)
        return 2

    retry_delay = 1.0
    while True:
        try:
            listen_once(
                args.device,
                args.base_url,
                grab=not args.no_grab,
                dry_run=args.dry_run,
                stop_after_one=args.once,
            )
            return 0
        except KeyboardInterrupt:
            print("\n已停止监听。", flush=True)
            return 0
        except PermissionError as error:
            target = os.path.realpath(args.device)
            print(
                f"设备权限错误：无法读取 {args.device} -> {target}（{error}）",
                file=sys.stderr,
            )
            return 1
        except ScannerDeviceError as error:
            print(f"扫码设备配置错误：{error}", file=sys.stderr)
            return 1
        except (FileNotFoundError, EOFError, OSError) as error:
            if args.once:
                print(f"无法监听设备：{error}", file=sys.stderr)
                return 1
            print(f"设备暂不可用：{error}；{retry_delay:g} 秒后重试…", file=sys.stderr)
            try:
                time.sleep(retry_delay)
            except KeyboardInterrupt:
                print("\n已停止监听。", flush=True)
                return 0


if __name__ == "__main__":
    raise SystemExit(main())
