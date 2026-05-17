"""EDINET XBRL financials parser — thin Python wrapper around Rust core."""

from __future__ import annotations

from stock_db._edinet_xbrl import parse_xbrl_artifact as _rust_parse_xbrl_artifact


def parse_xbrl_financials(xbrl_path: str) -> dict[str, dict[str, dict[str, float | None]]]:
    """Parse an EDINET XBRL artifact and return canonical financial_items."""
    try:
        return _rust_parse_xbrl_artifact(xbrl_path)["financials"]
    except RuntimeError as exc:
        from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError

        raise InventoriesTagMismatchError(str(exc)) from exc
