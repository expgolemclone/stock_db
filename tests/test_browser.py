from __future__ import annotations

import pytest

from stock_db.browser import (
    BrowserConfig,
    BrowserResponse,
    BrowserService,
    BrowserServiceError,
    build_proxy_fields,
)


class TestBuildProxyFields:
    def test_none_returns_empty(self) -> None:
        assert build_proxy_fields(None) == {}

    def test_http_proxy(self) -> None:
        result = build_proxy_fields("http://1.2.3.4:8080")

        assert result["proxy"] == "1.2.3.4:8080"
        assert "proxyType" not in result

    def test_socks5_proxy(self) -> None:
        result = build_proxy_fields("socks5h://1.2.3.4:1080")

        assert result["proxy"] == "1.2.3.4:1080"
        assert result["proxyType"] == "socks5"

    def test_proxy_with_auth(self) -> None:
        result = build_proxy_fields("http://user:p%40ss@1.2.3.4:8080")

        assert result["proxy"] == "1.2.3.4:8080"
        assert result["proxyUsername"] == "user"
        assert result["proxyPassword"] == "p@ss"


class TestBrowserResponse:
    def test_fields(self) -> None:
        resp = BrowserResponse(html="<html>", status=200, error=None)

        assert resp.html == "<html>"
        assert resp.status == 200
        assert resp.error is None

    def test_frozen(self) -> None:
        resp = BrowserResponse(html="<html>", status=200, error=None)

        with pytest.raises(AttributeError):
            resp.html = "changed"  # type: ignore[misc]


_TEST_CONFIG = BrowserConfig(
    pool_size=1, page_timeout=5000, idle_timeout=60,
    startup_timeout=5, headless=True, disable_xvfb=True,
    challenge_poll_interval_ms=250, challenge_clear_stable_ms=1000,
)


class TestBrowserServiceInit:
    def test_not_running_initially(self) -> None:
        svc = BrowserService(config=_TEST_CONFIG, browser_service_dir="/nonexistent")

        assert svc.running is False
        assert svc.port is None

    def test_fetch_raises_when_not_running(self) -> None:
        svc = BrowserService(config=_TEST_CONFIG, browser_service_dir="/nonexistent")

        with pytest.raises(BrowserServiceError):
            svc.fetch("http://example.com")
