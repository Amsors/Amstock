import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

from escpos.printer import Dummy

from label_printer import DPI, PRINT_WIDTH_DOTS, mm_to_dots, render_label, save_preview


class LabelPrinterTests(unittest.TestCase):
    def test_30_mm_label_dimensions_and_monochrome_mode(self) -> None:
        label = render_label("M-01-85-862390", "item")

        self.assertEqual((PRINT_WIDTH_DOTS, mm_to_dots(30)), label.size)
        self.assertEqual("1", label.mode)

    def test_container_label_can_be_encoded_by_python_escpos(self) -> None:
        label = render_label("A-00-67-425678", "container")
        printer = Dummy(profile="TM-T20II")

        printer.image(label, impl="bitImageRaster")
        printer.cut(mode="PART", feed=False)

        self.assertGreater(len(printer.output), 1000)
        self.assertTrue(printer.output.endswith(b"\x1dVB\x00"))

    def test_preview_has_203_dpi_metadata(self) -> None:
        label = render_label("I-34-00-000000", "item")
        with TemporaryDirectory() as directory:
            output = Path(directory) / "label.png"
            save_preview(label, output)

            from PIL import Image

            with Image.open(output) as saved:
                self.assertEqual(label.size, saved.size)
                self.assertAlmostEqual(DPI, saved.info["dpi"][0], delta=0.1)


if __name__ == "__main__":
    unittest.main()
