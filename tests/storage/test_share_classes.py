from __future__ import annotations

import sqlite3

from stock_db.storage.share_classes import replace_share_classes_for_ticker_source


def test_replace_share_classes_for_ticker_source(db_conn: sqlite3.Connection) -> None:
    replace_share_classes_for_ticker_source(
        db_conn,
        ticker="1301",
        source="edinet_xbrl",
        rows=[
            {
                "ticker": "1301",
                "period": "2025-03",
                "source": "edinet_xbrl",
                "class_key": "http://example.test#OrdinaryShareMember",
                "class_name": "普通株式",
                "shares": 1_000_000.0,
                "is_preferred": 0,
                "source_kind": "classes_of_shares_axis",
            }
        ],
    )
    replace_share_classes_for_ticker_source(
        db_conn,
        ticker="1301",
        source="edinet_xbrl",
        rows=[
            {
                "ticker": "1301",
                "period": "2025-03",
                "source": "edinet_xbrl",
                "class_key": "http://example.test#ClassAPreferredSharesMember",
                "class_name": "Ａ種優先株式",
                "shares": 3800.0,
                "is_preferred": 1,
                "source_kind": "classes_of_shares_axis",
            }
        ],
    )

    rows = db_conn.execute(
        """
        SELECT class_name, shares, is_preferred
        FROM share_classes
        WHERE ticker = '1301'
        ORDER BY class_key
        """
    ).fetchall()

    assert len(rows) == 1
    assert rows[0]["class_name"] == "Ａ種優先株式"
    assert rows[0]["shares"] == 3800.0
    assert rows[0]["is_preferred"] == 1
