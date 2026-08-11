import io
import unittest

import amstock_scanner


def key_event(code: int, value: int = 1) -> bytes:
    return amstock_scanner.INPUT_EVENT.pack(0, 0, amstock_scanner.EV_KEY, code, value)


class ScannerTest(unittest.TestCase):
    def test_normalizes_valid_code(self):
        self.assertEqual(amstock_scanner.normalize_code("a-12-34-567890"), "A-12-34-567890")

    def test_rejects_invalid_code(self):
        self.assertIsNone(amstock_scanner.normalize_code("A-1-34-567890"))
        self.assertIsNone(amstock_scanner.normalize_code("AA-12-34-567890"))

    def test_builds_lookup_url(self):
        self.assertEqual(
            amstock_scanner.build_lookup_url("http://localhost:43691", "A-12-34-567890"),
            "http://localhost:43691/display/A-12-34-567890",
        )

    def test_reads_keyboard_events_until_enter(self):
        key_codes = [
            30, 12, 2, 3, 12, 4, 5, 12, 6, 7, 8, 9, 10, 11, amstock_scanner.KEY_ENTER
        ]
        stream = io.BytesIO(b"".join(key_event(code) for code in key_codes))
        self.assertEqual(next(amstock_scanner.scan_codes(stream)), "A-12-34-567890")

    def test_ignores_invalid_scan_then_reads_next(self):
        invalid = [30, 12, 2, amstock_scanner.KEY_ENTER]
        valid = [30, 12, 2, 3, 12, 4, 5, 12, 6, 7, 8, 9, 10, 11, amstock_scanner.KEY_ENTER]
        stream = io.BytesIO(b"".join(key_event(code) for code in invalid + valid))
        self.assertEqual(next(amstock_scanner.scan_codes(stream)), "A-12-34-567890")


if __name__ == "__main__":
    unittest.main()
