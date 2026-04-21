from __future__ import annotations

import logging
import os
import queue
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import IO, Self, TypedDict
from urllib.parse import unquote, urlsplit

import requests

from stock_db.paths import BROWSER_SERVICE_DIR, magic_numbers

logger: logging.Logger = logging.getLogger("stock_db.browser.client")

_NODE_EXECUTABLE: str = os.environ.get("NODE_PATH", "node")
_BAD_GATEWAY_STATUS = 502
_SHUTDOWN_TIMEOUT_SECONDS = 5


class ProxyFields(TypedDict, total=False):
    proxy: str
    proxyType: str
    proxyUsername: str
    proxyPassword: str


class BrowserConfig(TypedDict):
    pool_size: int
    page_timeout: int
    idle_timeout: int
    startup_timeout: int
    headless: bool
    disable_xvfb: bool
    challenge_poll_interval_ms: int
    challenge_clear_stable_ms: int


def build_proxy_fields(proxy: str | None) -> ProxyFields:
    if proxy is None:
        return {}
    parsed = urlsplit(proxy)
    fields: ProxyFields = {"proxy": f"{parsed.hostname}:{parsed.port}"}
    if parsed.scheme.startswith("socks5"):
        fields["proxyType"] = "socks5"
    if parsed.username is not None:
        fields["proxyUsername"] = unquote(parsed.username)
        fields["proxyPassword"] = unquote(parsed.password or "")
    return fields


@dataclass(frozen=True, slots=True)
class BrowserResponse:
    html: str | None
    status: int
    error: str | None


class BrowserServiceError(RuntimeError):
    pass


class BrowserServiceClient:
    def __init__(
        self,
        *,
        config: BrowserConfig,
        browser_service_dir: str | Path = BROWSER_SERVICE_DIR,
    ) -> None:
        self._config: BrowserConfig = config
        self._browser_service_dir: Path = Path(browser_service_dir)
        self._process: subprocess.Popen[str] | None = None
        self._port: int | None = None
        self._base_url: str = ""

    @property
    def port(self) -> int | None:
        return self._port

    @property
    def running(self) -> bool:
        return self._process is not None and self._process.poll() is None

    def start(self) -> None:
        if self.running:
            return

        startup_poll_interval: float = magic_numbers()["browser"]["startup_poll_interval"]
        cfg = self._config
        env: dict[str, str] = {
            **os.environ,
            "BROWSER_POOL_SIZE": str(cfg["pool_size"]),
            "BROWSER_PAGE_TIMEOUT": str(cfg["page_timeout"]),
            "BROWSER_IDLE_TIMEOUT": str(cfg["idle_timeout"]),
            "BROWSER_HEADLESS": str(cfg["headless"]).lower(),
            "BROWSER_DISABLE_XVFB": str(cfg["disable_xvfb"]).lower(),
            "BROWSER_CHALLENGE_POLL_INTERVAL_MS": str(cfg["challenge_poll_interval_ms"]),
            "BROWSER_CHALLENGE_CLEAR_STABLE_MS": str(cfg["challenge_clear_stable_ms"]),
        }

        popen_kwargs: dict[str, object] = {
            "stdin": subprocess.DEVNULL,
            "stdout": subprocess.PIPE,
            "stderr": subprocess.STDOUT,
            "text": True,
            "encoding": "utf-8",
            "errors": "replace",
            "bufsize": 1,
            "env": env,
        }
        if sys.platform != "win32":
            # ブラウザサービスを端末セッションから切り離し、Chrome 停止時の
            # シグナル伝播が親のターミナルまで波及しないようにする。
            popen_kwargs["start_new_session"] = True

        self._process = subprocess.Popen(
            [_NODE_EXECUTABLE, str(self._browser_service_dir / "server.js")],
            **popen_kwargs,
        )
        assert self._process.stdout is not None
        stdout_stream: IO[str] = self._process.stdout

        startup_timeout: int = cfg["startup_timeout"]
        line_queue: queue.Queue[str] = queue.Queue()
        output_lines: list[str] = []

        def _reader() -> None:
            with stdout_stream as stream:
                while True:
                    try:
                        raw_line = stream.readline()
                    except OSError:
                        logger.debug("stdout stream closed")
                        return
                    if raw_line == "":
                        return
                    line = raw_line.strip()
                    output_lines.append(line)
                    line_queue.put(line)

        reader_thread: threading.Thread = threading.Thread(target=_reader, daemon=True)
        reader_thread.start()

        deadline: float = time.monotonic() + startup_timeout
        while time.monotonic() < deadline:
            if self._process.poll() is not None:
                stderr_output: str = "\n".join(output_lines)
                raise BrowserServiceError(
                    f"Browser service exited with code {self._process.returncode}: {stderr_output}"
                )
            try:
                line: str = line_queue.get(timeout=startup_poll_interval)
            except queue.Empty:
                continue
            if line.startswith("BROWSER_SERVICE_PORT="):
                self._port = int(line.split("=", 1)[1])
                self._base_url = f"http://127.0.0.1:{self._port}"
                logger.info("Browser service started on port %d", self._port)
                return

        self._kill()
        raise BrowserServiceError(
            f"Browser service did not start within {startup_timeout}s"
        )

    def fetch(
        self,
        url: str,
        *,
        proxy: str | None = None,
        timeout: int | None = None,
    ) -> BrowserResponse:
        if not self.running:
            raise BrowserServiceError("Browser service is not running")

        effective_timeout: int = timeout if timeout is not None else self._config["page_timeout"]
        fetch_body: dict[str, str | int | None] = {
            "url": url, "timeout": effective_timeout,
            **build_proxy_fields(proxy),
        }

        try:
            resp: requests.Response = requests.post(
                f"{self._base_url}/fetch",
                json=fetch_body,
                timeout=effective_timeout / 1000 + 10,
            )
            data: dict[str, str | int | None] = resp.json()
            return BrowserResponse(
                html=str(data.get("html")) if data.get("html") is not None else None,
                status=int(data.get("status", resp.status_code)),
                error=str(data["error"]) if data.get("error") is not None else None,
            )
        except requests.RequestException as exc:
            return BrowserResponse(html=None, status=_BAD_GATEWAY_STATUS, error=str(exc))

    def evaluate(
        self,
        url: str,
        script: str,
        *,
        proxy: str | None = None,
        timeout: int | None = None,
    ) -> object:
        if not self.running:
            raise BrowserServiceError("Browser service is not running")

        effective_timeout: int = timeout if timeout is not None else self._config["page_timeout"]
        body: dict[str, str | int | None] = {
            "url": url,
            "script": script,
            "timeout": effective_timeout,
            **build_proxy_fields(proxy),
        }
        try:
            resp: requests.Response = requests.post(
                f"{self._base_url}/evaluate",
                json=body,
                timeout=effective_timeout / 1000 + 10,
            )
            data: dict[str, object] = resp.json()
            if resp.status_code != 200 or data.get("error"):
                raise BrowserServiceError(
                    str(data.get("error", resp.status_code))
                )
            return data.get("result")
        except requests.RequestException as exc:
            raise BrowserServiceError(f"Evaluate request failed: {exc}") from exc

    def download(
        self,
        url: str,
        download_dir: str,
        *,
        selector: str | None = None,
        proxy: str | None = None,
        timeout: int | None = None,
    ) -> str:
        if not self.running:
            raise BrowserServiceError("Browser service is not running")

        effective_timeout: int = timeout if timeout is not None else self._config["page_timeout"]
        body: dict[str, str | int | None] = {
            "url": url,
            "downloadDir": download_dir,
            "timeout": effective_timeout,
            **build_proxy_fields(proxy),
        }
        if selector is not None:
            body["selector"] = selector

        try:
            resp: requests.Response = requests.post(
                f"{self._base_url}/download",
                json=body,
                timeout=effective_timeout / 1000 + 10,
            )
            data: dict[str, str | int | None] = resp.json()
            if resp.status_code != 200 or data.get("error"):
                raise BrowserServiceError(
                    f"Download failed: {data.get('error', resp.status_code)}"
                )
            return str(data["filePath"])
        except requests.RequestException as exc:
            raise BrowserServiceError(f"Download request failed: {exc}") from exc

    def shutdown(self) -> None:
        if not self.running:
            return
        try:
            requests.post(f"{self._base_url}/shutdown", timeout=_SHUTDOWN_TIMEOUT_SECONDS)
        except requests.RequestException:
            logger.debug("Shutdown request failed", exc_info=True)
        self._kill()
        logger.info("Browser service stopped")

    def _kill(self) -> None:
        if self._process is not None:
            try:
                self._process.terminate()
                self._process.wait(timeout=_SHUTDOWN_TIMEOUT_SECONDS)
            except (subprocess.TimeoutExpired, OSError):
                self._process.kill()
            self._process = None
            self._port = None
            self._base_url = ""

    def __enter__(self) -> Self:
        self.start()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: object,
    ) -> None:
        self.shutdown()

    def __repr__(self) -> str:
        status = f"port={self._port}" if self.running else "stopped"
        return f"BrowserServiceClient({status})"
