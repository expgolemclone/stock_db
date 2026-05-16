from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Sequence

from stock_db.paths import STOCKS_DB_PATH, cli_defaults
from stock_db.sources.edinet.xbrl_bs_parser import InventoriesTagMismatchError
from stock_db.sources.edinet.xbrl_financials_parser import parse_xbrl_financials
from stock_db.sources.edinet.xbrl_share_classes_parser import (
    ShareClassRow,
    parse_xbrl_share_classes,
)
from stock_db.storage.connection import get_connection
from stock_db.storage.financials import replace_financial_items_for_ticker_sources
from stock_db.storage.share_classes import replace_share_classes_for_ticker_source

_SOURCE = "edinet_xbrl"
_REPLACED_SOURCES = (
    "edinet_xbrl",
    "irbank",
    "irbank_bs",
    "irbank_forecast",
    "xbrl_bs",
)


def _parse_ticker(
    ticker: str,
    rows: list[sqlite3.Row],
) -> tuple[str, dict[str, dict[str, dict[str, float | None]]] | None, list[ShareClassRow]]:
    """Parse all XBRL artifacts for one ticker."""
    merged: dict[str, dict[str, dict[str, float | None]]] = {}
    share_classes_by_key: dict[tuple[str, str], ShareClassRow] = {}
    for row in rows:
        parsed = parse_xbrl_financials(str(row["xbrl_path"]))
        for period, statements in parsed.items():
            period_bucket = merged.setdefault(period, {})
            for statement, items in statements.items():
                statement_bucket = period_bucket.setdefault(statement, {})
                statement_bucket.update(items)
        for share_class in parse_xbrl_share_classes(str(row["xbrl_path"])):
            key = (share_class["period"], share_class["class_name"])
            existing = share_classes_by_key.get(key)
            if existing is None or _should_replace_share_class(existing, share_class):
                share_classes_by_key[key] = share_class
    return ticker, merged if merged else None, list(share_classes_by_key.values())


def _should_replace_share_class(existing: ShareClassRow, candidate: ShareClassRow) -> bool:
    existing_priority = _share_class_source_priority(existing["source_kind"])
    candidate_priority = _share_class_source_priority(candidate["source_kind"])
    return candidate_priority <= existing_priority


def _share_class_source_priority(source_kind: str) -> int:
    if source_kind == "classes_of_shares_axis":
        return 0
    return 1


def main(argv: Sequence[str] | None = None) -> int:
    defaults = cli_defaults("parse_xbrl_financials")
    parser = argparse.ArgumentParser(description="Parse EDINET XBRL files and store financial data")
    parser.add_argument("--ticker", type=str, help="Single ticker to parse")
    parser.add_argument(
        "--from-ticker",
        type=str,
        help="Resume from this ticker in sorted ticker order (inclusive)",
    )
    parser.add_argument(
        "--skip-existing",
        action="store_true",
        default=defaults.get("skip_existing", True),
        help="Skip tickers with existing edinet_xbrl data (default)",
    )
    parser.add_argument("--force", action="store_true", help="Disable --skip-existing")
    parser.add_argument(
        "--jobs",
        type=int,
        default=int(defaults.get("jobs", 1)),
        help="Number of parallel parse workers (default: from config, else 1)",
    )
    args = parser.parse_args(argv)

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    logger = logging.getLogger("parse_xbrl_financials")
    skip_existing = args.skip_existing and not args.force
    jobs = max(1, args.jobs)

    conn: sqlite3.Connection = get_connection(STOCKS_DB_PATH)
    try:
        rows = conn.execute(
            """
            SELECT ticker, fiscal_year, doc_id, xbrl_path, updated_at
            FROM sec_reports
            WHERE xbrl_path IS NOT NULL
            ORDER BY ticker, fiscal_year ASC, updated_at ASC, doc_id ASC
            """
        ).fetchall()

        grouped: dict[str, list[sqlite3.Row]] = defaultdict(list)
        for row in rows:
            grouped[str(row["ticker"])].append(row)

        tickers = sorted(grouped)
        if args.ticker:
            tickers = [ticker for ticker in tickers if ticker == args.ticker]
        if args.from_ticker:
            tickers = [ticker for ticker in tickers if ticker >= args.from_ticker]

        if not tickers:
            print("No XBRL files to parse", file=sys.stderr)
            return 1

        if skip_existing:
            existing = {
                row[0]
                for row in conn.execute(
                    "SELECT DISTINCT ticker FROM financial_items WHERE source = ?",
                    (_SOURCE,),
                ).fetchall()
            }
            before = len(tickers)
            tickers = [ticker for ticker in tickers if ticker not in existing]
            skipped = before - len(tickers)
            if skipped:
                logger.info("Skipping %d tickers with existing %s data", skipped, _SOURCE)

        ok = 0
        errors = 0

        if jobs == 1:
            # Serial path (unchanged behavior)
            for i, ticker in enumerate(tickers, 1):
                logger.info("[%d/%d] Parsing %s", i, len(tickers), ticker)
                try:
                    ticker_, merged, share_classes = _parse_ticker(ticker, grouped[ticker])
                except InventoriesTagMismatchError as exc:
                    logger.error("  %s: %s", ticker, exc)
                    errors += 1
                    continue

                if merged is None and not share_classes:
                    logger.info("  %s: no parseable financial or share-class facts", ticker)
                    continue

                db_rows = _build_db_rows(ticker, merged or {})
                share_class_rows = _build_share_class_db_rows(ticker, share_classes)
                _write_to_db(conn, ticker, db_rows, share_class_rows)
                logger.info(
                    "  %s: %d items across %d periods, %d share classes",
                    ticker,
                    len(db_rows),
                    len(merged or {}),
                    len(share_class_rows),
                )
                ok += 1
        else:
            # Parallel path: workers parse, main thread writes
            logger.info("Using %d parallel workers", jobs)
            with ThreadPoolExecutor(max_workers=jobs) as executor:
                futures = {
                    executor.submit(_parse_ticker, ticker, grouped[ticker]): ticker
                    for ticker in tickers
                }
                for i, future in enumerate(as_completed(futures), 1):
                    ticker = futures[future]
                    try:
                        _, merged, share_classes = future.result()
                    except InventoriesTagMismatchError as exc:
                        logger.error("[%d/%d] %s: %s", i, len(tickers), ticker, exc)
                        errors += 1
                        continue

                    if merged is None and not share_classes:
                        logger.info(
                            "[%d/%d] %s: no parseable financial or share-class facts",
                            i,
                            len(tickers),
                            ticker,
                        )
                        continue

                    db_rows = _build_db_rows(ticker, merged or {})
                    share_class_rows = _build_share_class_db_rows(ticker, share_classes)
                    _write_to_db(conn, ticker, db_rows, share_class_rows)
                    logger.info(
                        "[%d/%d] %s: %d items across %d periods, %d share classes",
                        i, len(tickers), ticker, len(db_rows), len(merged or {}),
                        len(share_class_rows),
                    )
                    ok += 1

        print(f"Done: {ok} ok, {errors} errors", file=sys.stderr)
        return 1 if errors > 0 else 0
    finally:
        conn.close()


def _build_db_rows(
    ticker: str,
    merged: dict[str, dict[str, dict[str, float | None]]],
) -> list[dict[str, str | float | None]]:
    db_rows: list[dict[str, str | float | None]] = []
    for period, statements in sorted(merged.items(), reverse=True):
        for statement, items in sorted(statements.items()):
            for item_name, value in sorted(items.items()):
                db_rows.append(
                    {
                        "ticker": ticker,
                        "period": period,
                        "statement": statement,
                        "item_name": item_name,
                        "value": value,
                        "source": _SOURCE,
                    }
                )
    return db_rows


def _build_share_class_db_rows(
    ticker: str,
    share_classes: list[ShareClassRow],
) -> list[dict[str, str | float | int]]:
    db_rows: list[dict[str, str | float | int]] = []
    for row in sorted(
        share_classes,
        key=lambda item: (item["period"], item["class_name"]),
        reverse=True,
    ):
        db_rows.append(
            {
                "ticker": ticker,
                "period": row["period"],
                "source": _SOURCE,
                "class_key": row["class_key"],
                "class_name": row["class_name"],
                "shares": row["shares"],
                "is_preferred": 1 if row["is_preferred"] else 0,
                "source_kind": row["source_kind"],
            }
        )
    return db_rows


def _write_to_db(
    conn: sqlite3.Connection,
    ticker: str,
    db_rows: list[dict[str, str | float | None]],
    share_class_rows: list[dict[str, str | float | int]],
) -> None:
    replace_financial_items_for_ticker_sources(
        conn,
        ticker=ticker,
        sources=_REPLACED_SOURCES,
        rows=db_rows,
    )
    replace_share_classes_for_ticker_source(
        conn,
        ticker=ticker,
        source=_SOURCE,
        rows=share_class_rows,
    )
    conn.commit()


if __name__ == "__main__":
    raise SystemExit(main())
