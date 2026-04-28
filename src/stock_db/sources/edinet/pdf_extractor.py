"""Extract text from EDINET PDFs and convert to Markdown."""

from __future__ import annotations

import logging
from io import StringIO
from pathlib import Path

from pdfminer.high_level import extract_text as pdfminer_extract
from pypdf import PdfReader

logger = logging.getLogger("stock_db.sources.edinet.pdf_extractor")


def _page_count(pdf_path: Path) -> int:
    reader = PdfReader(str(pdf_path))
    return len(reader.pages)


def extract_markdown(pdf_path: Path) -> str:
    """Extract text from a PDF and return Markdown-formatted string."""
    page_count = _page_count(pdf_path)

    text = pdfminer_extract(str(pdf_path))

    header = f"---\npages: {page_count}\n---\n"
    body = text.strip() if text.strip() else ""
    return header + body
