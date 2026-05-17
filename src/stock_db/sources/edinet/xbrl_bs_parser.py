"""EDINET XBRL artifact parser — thin Python wrapper around Rust core."""

from __future__ import annotations

from stock_db._edinet_xbrl import (
    is_valid_xbrl_path as _rust_is_valid_xbrl_path,
    is_valid_xbrl_text as _rust_is_valid_xbrl_text,
    parse_inventories as _rust_parse_inventories,
)

class InventoriesTagMismatchError(RuntimeError):
    """Raised when the inventories total cannot be determined safely."""


def parse_xbrl_bs(xbrl_path: str) -> dict[str, dict[str, float | None]]:
    """Parse an EDINET XBRL artifact dir or single `.xbrl` file."""
    try:
        return _rust_parse_inventories(xbrl_path)
    except RuntimeError as exc:
        raise InventoriesTagMismatchError(str(exc)) from exc


def is_valid_xbrl_text(content: str) -> bool:
    """Return True when the payload looks like a parseable EDINET iXBRL body."""
    return _rust_is_valid_xbrl_text(content)


def is_valid_xbrl_path(path: str | None) -> bool:
    """Return True when the saved XBRL artifact exists and passes minimal validation."""
    return _rust_is_valid_xbrl_path(None if path is None else str(path))
