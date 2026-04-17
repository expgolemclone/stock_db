"""Parse IR BANK /bs page HTML into structured financial data."""

from __future__ import annotations

import re

from bs4 import Tag

_JP_UNITS: dict[str, float] = {
    "兆": 1_000_000_000_000,
    "億": 100_000_000,
    "万": 10_000,
}
_JP_RE = re.compile(
    r"(?:(\d+(?:\.\d+)?)兆)?(?:(\d+(?:\.\d+)?)億)?(?:(\d+(?:\.\d+)?)万)?(\d+)?"
)

_DEBIT_MAP: dict[str, str] = {
    "投資等": "investments",
    "有形固定資産": "tangible_fixed_assets",
    "その他流動資産": "other_current_assets",
    "たな卸資産": "inventories",
    "売上債権": "trade_receivables",
    "現金等": "cash_and_deposits",
    "無形固定資産": "intangible_fixed_assets",
    "その他資産": "other_assets",
}

_CREDIT_MAP: dict[str, str] = {
    "株主資本": "stockholders_equity",
    "その他純資産": "other_equity",
    "固定負債": "non_current_liabilities",
    "その他流動負債": "other_current_liabilities",
    "仕入債務": "trade_payables",
}

_OVERVIEW_DEBIT_MAP: dict[str, str] = {
    "流動資産": "current_assets",
    "固定資産": "fixed_assets",
}

_OVERVIEW_CREDIT_MAP: dict[str, str] = {
    "純資産": "net_assets",
    "固定負債": "non_current_liabilities_total",
    "流動負債": "current_liabilities",
}

_PERIOD_RE = re.compile(r"(\d{4})年(\d{1,2})月")
_PCT_VAL_RE = re.compile(r"([\d.]+)%\s*([\d兆億万]+)")

def parse_japanese_number(text: str) -> float | None:
    """Parse Japanese-formatted number like '2兆8880億6200万' → 2888062000000.0."""
    if not text or text.strip() in ("-", "—", "−", ""):
        return None
    text = text.strip().rstrip("円")
    m = _JP_RE.fullmatch(text)
    if not m:
        return None
    cho = float(m.group(1)) if m.group(1) else 0.0
    oku = float(m.group(2)) if m.group(2) else 0.0
    man = float(m.group(3)) if m.group(3) else 0.0
    base = int(m.group(4)) if m.group(4) else 0
    if cho == 0 and oku == 0 and man == 0 and base == 0:
        return None
    return cho * _JP_UNITS["兆"] + oku * _JP_UNITS["億"] + man * _JP_UNITS["万"] + base


def _parse_period(text: str) -> str | None:
    """'2021年3月' → '2021-03'."""
    m = _PERIOD_RE.search(text)
    if not m:
        return None
    return f"{m.group(1)}-{int(m.group(2)):02d}"


def _table_rows(table: Tag) -> list[list[str]]:
    """Extract text content from all rows in a table."""
    rows: list[list[str]] = []
    for tr in table.find_all("tr"):
        cells = tr.find_all(["th", "td"])
        rows.append([c.get_text(strip=True) for c in cells])
    return rows


def _parse_detail_table(
    rows: list[list[str]],
    item_map: dict[str, str],
    result: dict[str, dict[str, float | None]],
) -> None:
    """Parse 借方/貸方 detail table.

    Header row: [year, item1_jp, item2_jp, ...]
    Data rows:  [period_text, val1, val2, ...]
    """
    if len(rows) < 2:
        return
    header = rows[0]
    col_names: list[str | None] = []
    for cell_text in header[1:]:
        col_names.append(item_map.get(cell_text))

    for row in rows[1:]:
        if not row:
            continue
        period = _parse_period(row[0])
        if period is None:
            continue
        for i, cell_text in enumerate(row[1:]):
            if i >= len(col_names) or col_names[i] is None:
                continue
            value = parse_japanese_number(cell_text)
            result.setdefault(period, {})[col_names[i]] = value  # type: ignore[index]


def _parse_overview_pivot(
    rows: list[list[str]],
    result: dict[str, dict[str, float | None]],
) -> None:
    """Parse pivot-format overview table.

    Header: [年, 固定資産, 流動資産, 純資産, 固定負債, 流動負債]
    Rows: '20XX年X月 借方' / '20XX年X月 貸方' with 'XX% value' cells.
    借方 rows: 固定資産 + 流動資産 have values, others are 0%.
    貸方 rows: 純資産 + 固定負債 + 流動負債 have values, others are 0%.
    """
    if len(rows) < 2:
        return
    header = rows[0]

    col_map: dict[int, str] = {}
    for col_idx in range(1, len(header)):
        label = header[col_idx]
        merged = {**_OVERVIEW_DEBIT_MAP, **_OVERVIEW_CREDIT_MAP}
        if label in merged:
            col_map[col_idx] = merged[label]

    for row in rows[1:]:
        if not row:
            continue
        period = _parse_period(row[0])
        if period is None:
            continue
        for col_idx, en_name in col_map.items():
            if col_idx >= len(row):
                continue
            cell = row[col_idx]
            if cell.startswith("0%"):
                continue
            m = _PCT_VAL_RE.search(cell)
            if m:
                value = parse_japanese_number(m.group(2))
            else:
                value = parse_japanese_number(cell)
            if value is not None:
                result.setdefault(period, {})[en_name] = value


def _parse_overview_vertical(
    rows: list[list[str]],
    result: dict[str, dict[str, float | None]],
) -> None:
    """Parse vertical-format overview table.

    Header: [年度, 借方, 貸方]
    Rows have mixed debit/credit items like '38.39% 4兆60億円'
    with labels spanning the item name in preceding rows or implied.
    """
    if len(rows) < 2:
        return

    for row in rows[1:]:
        if len(row) < 3:
            continue
        period = _parse_period(row[0])
        if period is None:
            continue

        debit_text = row[1]
        credit_text = row[2]

        # Match percentage-value pairs to determine item type
        for text, mapping in [(debit_text, _OVERVIEW_DEBIT_MAP), (credit_text, _OVERVIEW_CREDIT_MAP)]:
            for jp_name, en_name in mapping.items():
                if jp_name in text:
                    m = _PCT_VAL_RE.search(text)
                    if m:
                        value = parse_japanese_number(m.group(2))
                    else:
                        value = parse_japanese_number(text)
                    result.setdefault(period, {})[en_name] = value
                    break


def parse_bs_page(html: str) -> dict[str, dict[str, float | None]]:
    """Parse IR BANK /bs page HTML into {period: {item_name: value}}.

    Uses BeautifulSoup to find all tables and classify them by header content.
    """
    from bs4 import BeautifulSoup

    soup = BeautifulSoup(html, "html.parser")
    result: dict[str, dict[str, float | None]] = {}

    for table in soup.find_all("table"):
        rows = _table_rows(table)
        if not rows:
            continue
        all_text = " ".join(" ".join(r) for r in rows)

        if "借方" in all_text and "貸方" in all_text:
            header_text = " ".join(rows[0])
            if "固定資産" in header_text and "流動資産" in header_text:
                _parse_overview_pivot(rows, result)
            else:
                _parse_overview_vertical(rows, result)
        elif any(kw in all_text for kw in _DEBIT_MAP):
            _parse_detail_table(rows, _DEBIT_MAP, result)
        elif any(kw in all_text for kw in _CREDIT_MAP):
            _parse_detail_table(rows, _CREDIT_MAP, result)

    return result
