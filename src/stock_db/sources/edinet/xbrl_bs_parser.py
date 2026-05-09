"""EDINET XBRL artifact parser — thin Python wrapper around Rust core."""

from __future__ import annotations

import logging
import re

from stock_db._edinet_xbrl import parse_inventories as _rust_parse_inventories

logger = logging.getLogger(__name__)


class InventoriesTagMismatchError(RuntimeError):
    """Raised when the inventories total cannot be determined safely."""


def parse_xbrl_bs(xbrl_path: str) -> dict[str, dict[str, float | None]]:
    """Parse an EDINET XBRL artifact dir or single `.xbrl` file."""
    try:
        return _rust_parse_inventories(xbrl_path)
    except RuntimeError as exc:
        raise InventoriesTagMismatchError(str(exc)) from exc


_NONFRACTION_TAG_RE = re.compile(r"<ix:nonfraction\b", re.IGNORECASE)
_FISCAL_END_RE = re.compile(
    r'<ix:nonnumeric[^>]*name="jpdei_cor:CurrentFiscalYearEndDateDEI"[^>]*>'
    r"(\d{4})[年/-](\d{1,2})",
    re.IGNORECASE,
)


def is_valid_xbrl_text(content: str) -> bool:
    """Return True when the payload looks like a parseable EDINET iXBRL body."""
    return _NONFRACTION_TAG_RE.search(content) is not None and _FISCAL_END_RE.search(content) is not None


def is_valid_xbrl_path(path: str | None) -> bool:
    """Return True when the saved XBRL artifact exists and passes minimal validation."""
    if path is None:
        return False
    from pathlib import Path

    xbrl_path = Path(path)
    if xbrl_path.is_file():
        return False
    if not xbrl_path.is_dir():
        return False

    zip_path = xbrl_path.parent / f"{xbrl_path.name}.zip"
    if zip_path.is_file():
        for pattern in ("*.xbrl", "*.xhtml", "*.html", "*.htm"):
            for candidate in sorted(xbrl_path.rglob(pattern)):
                if candidate.suffix.lower() == ".xbrl":
                    return True
                try:
                    if is_valid_xbrl_text(candidate.read_text(encoding="utf-8")):
                        return True
                except OSError:
                    logger.debug("Skipping unreadable candidate %s", candidate, exc_info=True)
        return False

    return False
