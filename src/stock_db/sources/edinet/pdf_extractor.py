"""Extract text from EDINET PDFs and convert to Markdown."""

from __future__ import annotations

import logging
from pathlib import Path

from pypdf import PdfReader

logger = logging.getLogger("stock_db.sources.edinet.pdf_extractor")


def extract_markdown(pdf_path: Path) -> str:
    """Extract text from a PDF and return Markdown-formatted string."""
    reader = PdfReader(str(pdf_path))
    page_count = len(reader.pages)

    pages: list[str] = []
    for page in reader.pages:
        text = page.extract_text() or ""
        if text.strip():
            pages.append(text.strip())

    header = f"---\npages: {page_count}\n---\n"
    body = "\n\n".join(pages)
    return header + body
