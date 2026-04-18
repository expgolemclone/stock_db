from __future__ import annotations

import subprocess
import sys

import pytest

import stock_db.browser_client.client as client_module
from stock_db.browser_client.client import (
    BrowserConfig,
    BrowserResponse,
    BrowserServiceClient,
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


class TestBrowserServiceClientInit:
    def test_not_running_initially(self) -> None:
        svc = BrowserServiceClient(config=_TEST_CONFIG, browser_service_dir="/nonexistent")

        assert svc.running is False
        assert svc.port is None

    def test_fetch_raises_when_not_running(self) -> None:
        svc = BrowserServiceClient(config=_TEST_CONFIG, browser_service_dir="/nonexistent")

        with pytest.raises(BrowserServiceError):
            svc.fetch("http://example.com")

    def test_start_uses_pipes_and_new_session(self, monkeypatch: pytest.MonkeyPatch) -> None:
        captured: dict[str, object] = {}

        class FakeStdout:
            def __init__(self) -> None:
                self._lines = iter(["BROWSER_SERVICE_PORT=43210\n", ""])

            def __enter__(self) -> "FakeStdout":
                return self

            def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
                return None

            def readline(self) -> str:
                return next(self._lines)

        class FakeProcess:
            def __init__(self) -> None:
                self.stdout = FakeStdout()
                self._returncode: int | None = None

            def poll(self) -> int | None:
                return self._returncode

            def terminate(self) -> None:
                self._returncode = 0

            def wait(self, timeout: float | None = None) -> int:
                return 0

            def kill(self) -> None:
                self._returncode = -9

        def fake_popen(args: list[str], **kwargs: object) -> FakeProcess:
            captured["args"] = args
            captured["kwargs"] = kwargs
            return FakeProcess()

        monkeypatch.setattr(client_module.subprocess, "Popen", fake_popen)

        svc = BrowserServiceClient(config=_TEST_CONFIG, browser_service_dir="/tmp/browser-service")
        svc.start()

        assert svc.port == 43210
        assert captured["args"] == ["node", "/tmp/browser-service/server.js"]
        kwargs = captured["kwargs"]
        assert kwargs["stdin"] is subprocess.DEVNULL
        assert kwargs["stdout"] is subprocess.PIPE
        assert kwargs["stderr"] is subprocess.STDOUT
        assert kwargs["text"] is True
        assert kwargs["encoding"] == "utf-8"
        assert kwargs["errors"] == "replace"
        assert kwargs["bufsize"] == 1
        if sys.platform == "win32":
            assert "start_new_session" not in kwargs
        else:
            assert kwargs["start_new_session"] is True
