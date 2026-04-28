from __future__ import annotations

import logging

from stock_db.cli import scrape_edinet_watchdog


class TestScrapeEdinetWatchdog:
    def test_timeout_is_disabled_by_default(self, monkeypatch: object) -> None:
        monkeypatch.delenv("STOCK_DB_EDINET_WATCHDOG_TIMEOUT_SECONDS", raising=False)

        assert scrape_edinet_watchdog._scrape_timeout_seconds() is None

    def test_timeout_uses_positive_env_value(self, monkeypatch: object) -> None:
        monkeypatch.setenv("STOCK_DB_EDINET_WATCHDOG_TIMEOUT_SECONDS", "7200")

        assert scrape_edinet_watchdog._scrape_timeout_seconds() == 7200

    def test_timeout_disables_non_positive_env_value(self, monkeypatch: object) -> None:
        monkeypatch.setenv("STOCK_DB_EDINET_WATCHDOG_TIMEOUT_SECONDS", "0")

        assert scrape_edinet_watchdog._scrape_timeout_seconds() is None

    def test_timeout_disables_invalid_env_value(
        self, monkeypatch: object, caplog: object,
    ) -> None:
        monkeypatch.setenv("STOCK_DB_EDINET_WATCHDOG_TIMEOUT_SECONDS", "abc")

        with caplog.at_level(logging.WARNING):
            result = scrape_edinet_watchdog._scrape_timeout_seconds()

        assert result is None
        assert "Invalid STOCK_DB_EDINET_WATCHDOG_TIMEOUT_SECONDS='abc'" in caplog.text

    def test_timeout_expired_only_when_configured(self) -> None:
        assert scrape_edinet_watchdog._timeout_expired(
            10.0, now=4000.0, timeout_seconds=None,
        ) is False
        assert scrape_edinet_watchdog._timeout_expired(
            10.0, now=20.0, timeout_seconds=15,
        ) is False
        assert scrape_edinet_watchdog._timeout_expired(
            10.0, now=26.0, timeout_seconds=15,
        ) is True
