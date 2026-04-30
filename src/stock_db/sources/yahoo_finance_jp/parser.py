"""Parse Yahoo Finance Japan quote pages to extract price data."""

from __future__ import annotations

import re
from dataclasses import dataclass
from datetime import date
from typing import NamedTuple

from bs4 import BeautifulSoup


class _FieldResult(NamedTuple):
    value: str
    date: str | None


_NOT_FOUND_MARKER = "指定されたページは表示できません"
_PRICE_RE = re.compile(r"[\d,]+")
_DATE_RE = re.compile(r"\((\d{2})/(\d{2})\)")


@dataclass(frozen=True, slots=True)
class QuoteData:
    close: float
    date: str | None
    volume: int | None


def _extract_field(soup: BeautifulSoup, label: str) -> _FieldResult | None:
    for dl in soup.find_all("dl"):
        dt = dl.find("dt")
        if dt is None:
            continue
        name_span = dt.find("span")
        if name_span is None or name_span.get_text(strip=True) != label:
            continue

        dd = dl.find("dd")
        if dd is None:
            continue

        value_span = dd.find("span", class_=re.compile(r"value"))
        if value_span is None:
            continue
        value_text = value_span.get_text(strip=True)

        date_span = dd.find("span", class_=re.compile(r"date"))
        date_text = date_span.get_text(strip=True) if date_span else None

        return _FieldResult(value=value_text, date=date_text)

    return None


def _parse_price(raw: str) -> float | None:
    m = _PRICE_RE.search(raw)
    if m is None:
        return None
    return float(m.group().replace(",", ""))


def _parse_volume(raw: str) -> int | None:
    cleaned = raw.replace("株", "").replace(",", "").strip()
    try:
        return int(cleaned)
    except ValueError:
        return None


def _parse_date(raw: str | None) -> str | None:
    if raw is None:
        return None
    m = _DATE_RE.search(raw)
    if m is None:
        return None
    month, day = int(m.group(1)), int(m.group(2))
    today = date.today()
    year = today.year
    candidate = date(year, month, day)
    if candidate > today:
        candidate = date(year - 1, month, day)
    return candidate.isoformat()


def parse_quote_page(html: str) -> QuoteData | None:
    if not html or _NOT_FOUND_MARKER in html:
        return None

    soup = BeautifulSoup(html, "html.parser")

    close_field = _extract_field(soup, "前日終値")
    if close_field is None:
        return None

    close = _parse_price(close_field.value)
    if close is None:
        return None

    price_date = _parse_date(close_field.date)

    volume_field = _extract_field(soup, "出来高")
    volume = _parse_volume(volume_field.value) if volume_field else None

    return QuoteData(close=close, date=price_date, volume=volume)
