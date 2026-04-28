from __future__ import annotations

from pathlib import Path

from pypdf import PdfWriter

from stock_db.sources.edinet.pdf_extractor import extract_markdown


def _create_blank_pdf(path: Path, pages: int = 1) -> None:
    writer = PdfWriter()
    for _ in range(pages):
        writer.add_blank_page(width=595, height=842)
    writer.write(str(path))


class TestExtractMarkdown:
    def test_header_contains_page_count(self, tmp_path: Path) -> None:
        pdf_path = tmp_path / "test.pdf"
        _create_blank_pdf(pdf_path, pages=1)

        result = extract_markdown(pdf_path)

        assert "pages: 1" in result

    def test_multi_page_pdf(self, tmp_path: Path) -> None:
        pdf_path = tmp_path / "multi.pdf"
        _create_blank_pdf(pdf_path, pages=3)

        result = extract_markdown(pdf_path)

        assert "pages: 3" in result

    def test_output_starts_with_yaml_header(self, tmp_path: Path) -> None:
        pdf_path = tmp_path / "test.pdf"
        _create_blank_pdf(pdf_path)

        result = extract_markdown(pdf_path)

        assert result.startswith("---\npages: ")
        assert "---\n" in result[3:]

    def test_returns_string_for_empty_pdf(self, tmp_path: Path) -> None:
        pdf_path = tmp_path / "test.pdf"
        _create_blank_pdf(pdf_path)

        result = extract_markdown(pdf_path)

        assert isinstance(result, str)
