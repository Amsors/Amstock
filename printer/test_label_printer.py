import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from escpos.printer import Dummy
from PIL import Image, ImageDraw, ImageFont

from label_printer import (
    DPI,
    B1_ROW_HEIGHT_DOTS,
    CHILD_LIST_PADDING_DOTS,
    FONT_MONO_BOLD_CANDIDATES,
    FONT_REGULAR_CANDIDATES,
    PRINT_WIDTH_DOTS,
    LabelRequest,
    find_font,
    layout_name_lines,
    make_qr,
    mm_to_dots,
    parse_label_request,
    render_label,
    render_request,
    save_preview,
)


class LabelPrinterTests(unittest.TestCase):
    def test_20_mm_label_dimensions_and_monochrome_mode(self) -> None:
        label = render_label("M-01-85-862390", "item")

        self.assertEqual((PRINT_WIDTH_DOTS, mm_to_dots(20)), label.size)
        self.assertEqual("1", label.mode)

    def test_item_and_container_use_the_same_compact_layout(self) -> None:
        item = render_label("M-01-85-862390", "item")
        container = render_label("M-01-85-862390", "container")

        self.assertEqual(item.tobytes(), container.tobytes())

    def test_json_a1_preserves_the_legacy_layout(self) -> None:
        request = parse_label_request(
            {
                "schema_version": 1,
                "style": "A1",
                "kind": "item",
                "identifier": "M-01-85-862390",
            }
        )

        self.assertEqual(
            render_label(request.identifier, request.kind).tobytes(),
            render_request(request).tobytes(),
        )

    def test_qr_symbol_uses_most_of_the_unchanged_a1_height(self) -> None:
        qr = make_qr("M-01-85-862390", mm_to_dots(20) - 4)
        black_pixels = qr.point(lambda value: 255 - value)
        black_box = black_pixels.getbbox()

        self.assertIsNotNone(black_box)
        assert black_box is not None
        self.assertGreaterEqual(black_box[2] - black_box[0], 120)
        self.assertLessEqual(qr.height, mm_to_dots(20) - 4)

    def test_child_identifier_font_is_monospaced(self) -> None:
        draw = ImageDraw.Draw(Image.new("L", (300, 100), 255))
        font_path = find_font(FONT_MONO_BOLD_CANDIDATES)
        font = ImageFont.truetype(font_path, 22)

        self.assertEqual(
            draw.textlength("111111", font=font),
            draw.textlength("WWWWWW", font=font),
        )

    def test_a2_is_30_mm_and_accepts_a_long_name(self) -> None:
        request = LabelRequest(
            style="A2",
            kind="item",
            identifier="M-01-85-862390",
            name="这是一个需要换成两行并且最终可能需要截断的超长物品名称测试文本",
        )

        label = render_request(request)

        self.assertEqual((PRINT_WIDTH_DOTS, mm_to_dots(30)), label.size)
        self.assertEqual("1", label.mode)

    def test_name_prefers_one_line_then_truncates_after_two(self) -> None:
        draw = ImageDraw.Draw(Image.new("L", (PRINT_WIDTH_DOTS, 100), 255))
        font_path = find_font(FONT_REGULAR_CANDIDATES)

        short_lines, _ = layout_name_lines(draw, "尖头镊子", font_path, 540)
        long_lines, _ = layout_name_lines(
            draw,
            "这是一个非常长的物品名称" * 12,
            font_path,
            540,
        )

        self.assertEqual(["尖头镊子"], short_lines)
        self.assertEqual(2, len(long_lines))
        self.assertTrue(long_lines[1].endswith("...."))

    def test_b1_uses_two_compact_columns_and_variable_length(self) -> None:
        request = parse_label_request(
            {
                "schema_version": 1,
                "style": "B1",
                "kind": "container",
                "identifier": "C-00-00-000001",
                "name": "元件箱",
                "children": [
                    {"identifier": f"I-00-00-{index:06d}"} for index in range(5)
                ],
            }
        )

        label = render_request(request)

        self.assertEqual(PRINT_WIDTH_DOTS, label.width)
        self.assertEqual(
            mm_to_dots(30) + CHILD_LIST_PADDING_DOTS + 3 * B1_ROW_HEIGHT_DOTS,
            label.height,
        )

    def test_b2_is_taller_than_b1_for_the_same_children(self) -> None:
        children = [
            {"identifier": f"I-00-00-{index:06d}", "name": f"物品 {index}"}
            for index in range(4)
        ]
        common = {
            "schema_version": 1,
            "kind": "container",
            "identifier": "C-00-00-000001",
            "name": "元件箱",
            "children": children,
        }
        b1 = render_request(parse_label_request({**common, "style": "B1"}))
        b2 = render_request(parse_label_request({**common, "style": "B2"}))

        self.assertGreater(b2.height, b1.height)

    def test_b_styles_reject_items(self) -> None:
        with self.assertRaisesRegex(ValueError, "仅适用于 container"):
            parse_label_request(
                {
                    "schema_version": 1,
                    "style": "B1",
                    "kind": "item",
                    "identifier": "I-00-00-000001",
                    "name": "物品",
                }
            )

    def test_b2_requires_every_child_name(self) -> None:
        with self.assertRaisesRegex(ValueError, r"children\[0\]\.name"):
            parse_label_request(
                {
                    "schema_version": 1,
                    "style": "B2",
                    "kind": "container",
                    "identifier": "C-00-00-000001",
                    "name": "元件箱",
                    "children": [{"identifier": "I-00-00-000002"}],
                }
            )

    def test_container_label_can_be_encoded_by_python_escpos(self) -> None:
        label = render_label("A-00-67-425678", "container")
        printer = Dummy(profile="TM-T20II")

        printer.image(label, impl="bitImageRaster")
        printer.cut(mode="PART", feed=False)

        self.assertGreater(len(printer.output), 1000)
        self.assertTrue(printer.output.endswith(b"\x1dVB\x00"))

    def test_dynamic_b2_label_can_be_encoded_by_python_escpos(self) -> None:
        request = parse_label_request(
            {
                "schema_version": 1,
                "style": "B2",
                "kind": "container",
                "identifier": "C-00-00-000001",
                "name": "元件箱",
                "children": [
                    {"identifier": "I-00-00-000002", "name": "电阻"},
                    {"identifier": "I-00-00-000003", "name": "电容"},
                ],
            }
        )
        printer = Dummy(profile="TM-T20II")

        printer.image(render_request(request), impl="bitImageRaster")

        self.assertGreater(len(printer.output), 1000)

    def test_preview_has_203_dpi_metadata(self) -> None:
        label = render_label("I-34-00-000000", "item")
        with TemporaryDirectory() as directory:
            output = Path(directory) / "label.png"
            save_preview(label, output, open_after_save=False)

            from PIL import Image

            with Image.open(output) as saved:
                self.assertEqual(label.size, saved.size)
                self.assertAlmostEqual(DPI, saved.info["dpi"][0], delta=0.1)


if __name__ == "__main__":
    unittest.main()
