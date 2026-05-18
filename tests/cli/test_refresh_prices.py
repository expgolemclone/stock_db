from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.cli import refresh_prices as cli_module
from stock_db.sources.price_refresh import PriceRefreshError


def test_main_prints_noop_message_when_prices_are_fresh(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"
    captured: dict[str, object] = {}

    def fake_refresh_prices(**kwargs: object) -> None:
        captured.update(kwargs)
        return None

    monkeypatch.setattr(cli_module, "refresh_prices", fake_refresh_prices)

    rc = cli_module.main(["--db", str(db_path), "--if-needed"])
    output = capsys.readouterr()

    assert rc == 0
    assert captured["db_path"] == db_path
    assert captured["if_needed"] is True
    assert output.err.strip() == "Stock prices are fresh or were refreshed recently; no update needed"


def test_main_returns_1_on_refresh_failure(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    db_path = tmp_path / "stocks.db"

    def fake_refresh_prices(**kwargs: object) -> None:
        del kwargs
        raise PriceRefreshError("Yahoo failed")

    monkeypatch.setattr(cli_module, "refresh_prices", fake_refresh_prices)

    rc = cli_module.main(["--db", str(db_path)])
    output = capsys.readouterr()

    assert rc == 1
    assert output.err.strip() == "Yahoo failed"
