from __future__ import annotations

from stock_db.sources.edinet import document_list


def test_redacts_subscription_key_from_error_text() -> None:
    assert (
        document_list._redact_subscription_key(
            "documents.json?date=2024-01-01&type=2&Subscription-Key=secret-key"
        )
        == "documents.json?date=2024-01-01&type=2&Subscription-Key=<redacted>"
    )


def test_discover_historical_reports_skips_completed_dates_and_calls_checkpoint(
    monkeypatch,
) -> None:
    fetch_calls: list[str] = []
    scanned_days: list[tuple[str, int, int]] = []

    def fake_fetch_document_list(target_date: str, api_key: str) -> list[dict]:
        del api_key
        fetch_calls.append(target_date)
        return [
            {
                "ordinanceCode": "010",
                "formCode": "030000",
                "secCode": "72030",
                "docID": "S100NEXT",
                "periodEnd": "2024-03-31",
                "submitDateTime": "2024-06-25T15:00:00",
                "filerName": "Toyota",
            }
        ]

    def on_day_scanned(date_str: str, matches: list[tuple[str, dict]], total_annual: int) -> None:
        scanned_days.append((date_str, len(matches), total_annual))

    monkeypatch.setattr(document_list, "fetch_document_list", fake_fetch_document_list)
    monkeypatch.setattr(document_list.time, "sleep", lambda interval: None)

    reports = document_list.discover_historical_reports(
        from_date="2024-06-24",
        to_date="2024-06-25",
        api_key="dummy",
        target_tickers={"7203"},
        interval=0,
        initial_reports={"7203": [{"doc_id": "S100OLD"}]},
        skip_dates={"2024-06-24"},
        on_day_scanned=on_day_scanned,
    )

    assert fetch_calls == ["2024-06-25"]
    assert scanned_days == [("2024-06-25", 1, 1)]
    assert reports == {
        "7203": [
            {"doc_id": "S100OLD"},
            {
                "doc_id": "S100NEXT",
                "fiscal_year": "FY2024",
                "period_end": "2024-03-31",
                "submit_date": "2024-06-25T15:00:00",
                "filer_name": "Toyota",
            },
        ]
    }
