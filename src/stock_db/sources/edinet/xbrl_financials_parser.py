"""EDINET XBRL financials parser — thin Python wrapper around Rust core."""

from __future__ import annotations

from stock_db._edinet_xbrl import parse_financials as _rust_parse_financials


def parse_xbrl_financials(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
    """Parse an EDINET XBRL artifact and return canonical financial_items."""
    try:
        return _rust_parse_financials(xbrl_path)
    except RuntimeError as exc:
        from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError

        raise InventoriesTagMismatchError(str(exc)) from exc
