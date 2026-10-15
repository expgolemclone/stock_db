"""EDINET identifier and company-name helpers."""

from __future__ import annotations

import csv
import html
import logging
import re
import sqlite3
import unicodedata
from pathlib import Path

logger = logging.getLogger("stock_db.sources.edinet.resolver")

# ---------------------------------------------------------------------------
# secCode handling
# ---------------------------------------------------------------------------

_SEC_CODE_RE = re.compile(r"^\d{5}$")


def sec_code_to_ticker(sec_code: str | None) -> str | None:
    """Convert 5-digit EDINET secCode to 4-digit ticker.

    EDINET API v2 returns secCode as 5 digits where the trailing digit is a
    check digit (e.g. ``72030`` → ``7203``).

    We do **not** re-validate the check digit here.  Rationale: the EDINET
    API is the authoritative source — if it returns a secCode, the value is
    already valid.  Re-computing the check digit would add complexity without
    catching any real-world error (the API never returns malformed codes).
    We simply strip the 5th digit after confirming the field is exactly
    5 ASCII digits.  Values that fail this format check are rejected to
    avoid silent mismatches.
    """
    if not sec_code:
        return None
    sec_code = str(sec_code).strip()
    if not _SEC_CODE_RE.match(sec_code):
        return None
    return sec_code[:4]


# ---------------------------------------------------------------------------
# Company name normalization
# ---------------------------------------------------------------------------

_KABUSHIKI_GAISHA = "株式会社"


def normalize_company_name(name: str | None) -> str | None:
    """Normalize a company name for matching purposes.

    Pipeline: HTML unescape → strip → NFKC normalize → whitespace collapse
    → （株）/(株) → 株式会社 unification.
    """
    if name is None:
        return None
    normalized = html.unescape(name).strip()
    if not normalized:
        return None
    normalized = unicodedata.normalize("NFKC", normalized)
    normalized = re.sub(r"\s+", " ", normalized)
    normalized = normalized.replace("（株）", _KABUSHIKI_GAISHA).replace(
        "(株)", _KABUSHIKI_GAISHA
    )
    return normalized.strip() or None


def _dedupe_preserve_order(names: list[str]) -> list[str]:
    seen: set[str] = set()
    ordered: list[str] = []
    for name in names:
        if name not in seen:
            seen.add(name)
            ordered.append(name)
    return ordered


def build_company_name_candidates(*names: str | None) -> list[str]:
    """Generate search-variant company names from one or more raw names.

    For each input, produces:
    - The normalized name itself
    - With 株式会社 prefix removed / re-added as suffix
    - With 株式会社 suffix removed / re-added as prefix

    Returns deduplicated results preserving first-seen order.
    """
    candidates: list[str] = []
    for raw_name in names:
        name = normalize_company_name(raw_name)
        if not name:
            continue
        variants = [name]
        if name.startswith(_KABUSHIKI_GAISHA):
            stripped = name.removeprefix(_KABUSHIKI_GAISHA).strip()
            if stripped:
                variants.append(stripped)
                variants.append(f"{stripped}{_KABUSHIKI_GAISHA}")
        elif name.endswith(_KABUSHIKI_GAISHA):
            stripped = name.removesuffix(_KABUSHIKI_GAISHA).strip()
            if stripped:
                variants.append(stripped)
                variants.append(f"{_KABUSHIKI_GAISHA}{stripped}")
        else:
            variants.append(f"{_KABUSHIKI_GAISHA}{name}")
            variants.append(f"{name}{_KABUSHIKI_GAISHA}")
        candidates.extend(variants)
    return _dedupe_preserve_order([c for c in candidates if c])


# ---------------------------------------------------------------------------
# Attribution verification
# ---------------------------------------------------------------------------

def verify_attribution(
    ticker: str,
    *,
    expected_edinet_code: str | None,
    found_edinet_code: str | None,
) -> bool:
    """Verify that a found EDINET code belongs to the target ticker.

    Returns True if attribution is valid (or cannot be checked).
    Returns False and logs a warning if a known mismatch is detected.
    """
    if not expected_edinet_code or not found_edinet_code:
        return True
    if found_edinet_code == expected_edinet_code:
        return True
    logger.warning(
        "Attribution mismatch for ticker %s: expected edinet_code=%s, found=%s — rejecting",
        ticker,
        expected_edinet_code,
        found_edinet_code,
    )
    return False


# ---------------------------------------------------------------------------
# EDINET Code List CSV ingestion
# ---------------------------------------------------------------------------

_EDINET_CSV_ENCODING = "cp932"

# Column header patterns (the exact header text varies by CSV version).
# We normalize with NFKC before matching to handle fullwidth/halfwidth differences.
_HEADER_PATTERNS_EDINET = {"EDINETコード"}
_HEADER_PATTERNS_SEC = {"証券コード"}
_HEADER_PATTERNS_NAME = {"提出者名"}


def _normalize_csv_header(header: str) -> str:
    return unicodedata.normalize("NFKC", header.strip())


def parse_edinet_code_csv(csv_path: Path) -> list[dict[str, str]]:
    """Parse the EDINET Code List CSV (EdinetcodeDlInfo.csv).

    The CSV is Shift-JIS encoded.  The first line may be metadata or a blank
    line; we scan for the header row containing 'EDINETコード' and '証券コード'.

    Returns a list of dicts with keys: edinet_code, sec_code, ticker, name.
    Only rows with a valid 4-digit ticker (derived from 5-digit secCode) are included.
    """
    rows: list[dict[str, str]] = []
    with csv_path.open(encoding=_EDINET_CSV_ENCODING, newline="") as f:
        reader = csv.reader(f)

        # Scan for the header row (skip preamble lines)
        headers: list[str] = []
        col_edinet: int | None = None
        col_sec: int | None = None
        col_name: int | None = None
        for raw_headers in reader:
            normalized = [_normalize_csv_header(h) for h in raw_headers]
            for i, h in enumerate(normalized):
                if any(p in h for p in _HEADER_PATTERNS_EDINET):
                    col_edinet = i
                if any(p in h for p in _HEADER_PATTERNS_SEC):
                    col_sec = i
                if any(p in h for p in _HEADER_PATTERNS_NAME):
                    col_name = i
            if col_edinet is not None and col_sec is not None:
                headers = normalized
                break

        if col_edinet is None or col_sec is None:
            logger.warning(
                "EDINET CSV missing required columns (edinet=%s, sec=%s): %s",
                col_edinet,
                col_sec,
                csv_path,
            )
            return rows

        for row in reader:
            if len(row) <= max(col_edinet, col_sec):
                continue
            edinet_code = row[col_edinet].strip()
            sec_code_raw = row[col_sec].strip()
            name = row[col_name].strip() if col_name is not None and col_name < len(row) else ""

            ticker = sec_code_to_ticker(sec_code_raw)
            if not ticker or not edinet_code:
                continue

            rows.append(
                {
                    "edinet_code": edinet_code,
                    "sec_code": sec_code_raw,
                    "ticker": ticker,
                    "name": name,
                }
            )
    return rows


# ---------------------------------------------------------------------------
# EdinetResolver
# ---------------------------------------------------------------------------

class EdinetResolver:
    """Resolver for EDINET Code / Securities Code / company name to ticker."""

    def __init__(
        self,
        *,
        edinet_to_ticker: dict[str, str],
        ticker_to_edinet: dict[str, str],
        ticker_to_name: dict[str, str],
    ) -> None:
        self._edinet_to_ticker = dict(edinet_to_ticker)
        self._ticker_to_edinet = dict(ticker_to_edinet)
        self._ticker_to_name = dict(ticker_to_name)

    # -- Factory methods ----------------------------------------------------

    @classmethod
    def from_db(cls, conn: sqlite3.Connection) -> EdinetResolver:
        """Build a resolver from the current DB state."""
        rows = conn.execute(
            "SELECT ticker, edinet_code, name FROM stocks"
        ).fetchall()

        edinet_to_ticker: dict[str, str] = {}
        ticker_to_edinet: dict[str, str] = {}
        ticker_to_name: dict[str, str] = {}

        for row in rows:
            ticker = str(row["ticker"])
            name = row["name"] or ""
            edinet_code = row["edinet_code"]

            ticker_to_name[ticker] = name
            if edinet_code:
                ticker_to_edinet[ticker] = edinet_code
                edinet_to_ticker[edinet_code] = ticker

        return cls(
            edinet_to_ticker=edinet_to_ticker,
            ticker_to_edinet=ticker_to_edinet,
            ticker_to_name=ticker_to_name,
        )

    @classmethod
    def from_edinet_code_csv(
        cls,
        conn: sqlite3.Connection,
        csv_path: Path,
    ) -> EdinetResolver:
        """Build a resolver enriched with the EDINET Code List CSV.

        Parses the CSV and upserts edinet_code + name into stock_entities,
        then returns a resolver reflecting the updated state.
        """
        csv_rows = parse_edinet_code_csv(csv_path)
        if not csv_rows:
            logger.warning("No valid rows in EDINET Code CSV: %s", csv_path)
            return cls.from_db(conn)

        from stock_db.storage._util import utc_now_iso

        now = utc_now_iso()
        for entry in csv_rows:
            ticker = entry["ticker"]
            edinet_code = entry["edinet_code"]
            name = entry["name"]

            conn.execute(
                """
                INSERT INTO stock_entities
                    (ticker, edinet_code, name, sector, market, updated_at)
                VALUES (?, ?, ?, '', '', ?)
                ON CONFLICT(ticker) DO UPDATE SET
                    edinet_code = COALESCE(excluded.edinet_code, stock_entities.edinet_code),
                    name = CASE
                             WHEN excluded.name = ''
                             THEN stock_entities.name
                             ELSE excluded.name
                           END,
                    updated_at = excluded.updated_at
                """,
                (ticker, edinet_code, name, now),
            )
        conn.commit()
        logger.info(
            "Ingested %d rows from EDINET Code List CSV", len(csv_rows)
        )
        return cls.from_db(conn)

    # -- Identifier conversion -----------------------------------------------

    def resolve_ticker(
        self,
        result: dict,
        target_tickers: set[str] | None = None,
    ) -> tuple[str | None, str | None]:
        """Resolve a ticker from an EDINET API result row.

        Primary: secCode (first 4 digits).
        Fallback: edinetCode reverse-lookup.

        Returns (ticker, edinet_code_or_None).
        When *target_tickers* is provided, only returns tickers in that set.
        """
        ticker = sec_code_to_ticker(result.get("secCode"))
        edinet_code = result.get("edinetCode", "")
        found_edinet = edinet_code or None
        mapped = self._edinet_to_ticker.get(edinet_code) if edinet_code else None

        if ticker is not None:
            if mapped is not None and mapped != ticker:
                logger.warning(
                    "EDINET identifier mismatch for doc_id=%s: secCode maps to "
                    "ticker=%s but edinetCode=%s maps to ticker=%s; rejecting",
                    result.get("docID"),
                    ticker,
                    edinet_code,
                    mapped,
                )
                return None, found_edinet
            if target_tickers is None or ticker in target_tickers:
                return ticker, found_edinet
            return None, found_edinet

        if mapped and (target_tickers is None or mapped in target_tickers):
            return mapped, edinet_code

        return None, found_edinet

    def edinet_code_for_ticker(self, ticker: str) -> str | None:
        """Return the edinetCode for a ticker, or None if unknown."""
        return self._ticker_to_edinet.get(ticker)

    def ticker_for_edinet_code(self, edinet_code: str) -> str | None:
        """Return the ticker for an edinetCode, or None if unknown."""
        return self._edinet_to_ticker.get(edinet_code)

    def name_for_ticker(self, ticker: str) -> str | None:
        """Return the company name for a ticker, or None."""
        return self._ticker_to_name.get(ticker)

    # -- Reverse maps -------------------------------------------------------

    @property
    def edinet_code_reverse_map(self) -> dict[str, str]:
        """edinetCode → ticker mapping for fallback resolution."""
        return dict(self._edinet_to_ticker)

    @property
    def ticker_edinet_map(self) -> dict[str, str]:
        """ticker → edinetCode mapping."""
        return dict(self._ticker_to_edinet)

    # -- filerName cross-check ----------------------------------------------

    def check_filer_name(
        self,
        ticker: str,
        filer_name: str | None,
    ) -> None:
        """Warn if filerName from an API result doesn't match the DB name.

        This is a diagnostic aid — it logs warnings but does not reject matches.
        Useful for catching ticker↔edinetCode misassignments early.
        """
        if not filer_name:
            return
        expected = self._ticker_to_name.get(ticker)
        if not expected:
            return
        normalized_filer = normalize_company_name(filer_name)
        normalized_expected = normalize_company_name(expected)
        if not normalized_filer or not normalized_expected:
            return
        if normalized_filer == normalized_expected:
            return
        # Check if one contains the other (partial match for subsidiary names)
        if normalized_expected in normalized_filer or normalized_filer in normalized_expected:
            return
        logger.warning(
            "filerName mismatch for ticker %s: DB=%r, filing=%r",
            ticker,
            expected,
            filer_name,
        )
