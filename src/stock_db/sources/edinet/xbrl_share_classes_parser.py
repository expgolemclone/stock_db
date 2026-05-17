"""EDINET XBRL share class parser — thin Python wrapper around Rust core."""

from __future__ import annotations

from typing import TypedDict

from stock_db._edinet_xbrl import parse_xbrl_artifact as _rust_parse_xbrl_artifact


class ShareClassRow(TypedDict):
    period: str
    class_key: str
    class_name: str
    shares: float
    is_preferred: bool
    source_kind: str


def parse_xbrl_share_classes(xbrl_path: str) -> list[ShareClassRow]:
    """Parse an EDINET XBRL artifact and return share-class issued share counts."""
    return _rust_parse_xbrl_artifact(xbrl_path)["share_classes"]
