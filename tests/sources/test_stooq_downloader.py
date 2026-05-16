from __future__ import annotations

import base64
from pathlib import Path

import pytest

from stock_db.browser_client.client import BrowserServiceError, StooqDailyDownloadSession
from stock_db.sources.stooq import (
    StooqCaptchaError,
    StooqDownloadError,
    download_latest_daily_file,
)


class FakeBrowserClient:
    def __init__(
        self,
        *,
        content: bytes,
        file_name: str = "0429_d.csv",
        complete_side_effects: list[Exception] | None = None,
    ) -> None:
        self._content = content
        self._file_name = file_name
        self._complete_side_effects = complete_side_effects or []
        self.prepare_calls = 0
        self.completed_calls: list[tuple[str, str, str, int | None]] = []
        self.closed_sessions: list[str] = []

    def prepare_stooq_daily_download(self, *, timeout: int | None = None) -> StooqDailyDownloadSession:
        self.prepare_calls += 1
        return StooqDailyDownloadSession(
            session_id=f"session-{self.prepare_calls}",
            date="20260429",
            label="0429_d",
            download_url="https://stooq.com/db/d/?d=20260429&t=d",
            captcha_png_base64=base64.b64encode(b"captcha-bytes").decode("ascii"),
        )

    def complete_stooq_daily_download(
        self,
        session_id: str,
        captcha_code: str,
        download_dir: str,
        *,
        timeout: int | None = None,
    ) -> str:
        self.completed_calls.append((session_id, captcha_code, download_dir, timeout))
        if self._complete_side_effects:
            raise self._complete_side_effects.pop(0)
        path = Path(download_dir) / self._file_name
        path.write_bytes(self._content)
        return str(path)

    def close_stooq_session(self, session_id: str) -> None:
        self.closed_sessions.append(session_id)


def test_download_latest_daily_file_returns_downloaded_path(tmp_path: Path) -> None:
    client = FakeBrowserClient(
        content=(
            b"Symbol,Date,Time,Open,High,Low,Close,Volume\n"
            b"7203.JP,2026-04-29,08:00:00,3065,3101,3057,3067,19390900\n"
        ),
    )

    downloaded = download_latest_daily_file(
        client,
        tmp_path,
        captcha_solver=lambda _image: "D1TY",
    )

    assert downloaded.date == "20260429"
    assert downloaded.label == "0429_d"
    assert downloaded.file_path.is_file()
    assert client.completed_calls == [
        ("session-1", "D1TY", str(tmp_path), None),
    ]
    assert client.closed_sessions == []


def test_download_latest_daily_file_reuses_existing_latest_file(tmp_path: Path) -> None:
    existing_path = tmp_path / "0429_d.csv"
    existing_path.write_text("already downloaded\n", encoding="utf-8")
    client = FakeBrowserClient(content=b"unused")

    downloaded = download_latest_daily_file(
        client,
        tmp_path,
        captcha_solver=lambda _image: "D1TY",
    )

    assert downloaded.file_path == existing_path
    assert client.completed_calls == []
    assert client.closed_sessions == ["session-1"]


def test_download_latest_daily_file_reuses_existing_date_named_file(tmp_path: Path) -> None:
    existing_path = tmp_path / "20260429_d.txt"
    existing_path.write_text("already downloaded\n", encoding="utf-8")
    client = FakeBrowserClient(content=b"unused")

    downloaded = download_latest_daily_file(
        client,
        tmp_path,
        captcha_solver=lambda _image: "D1TY",
    )

    assert downloaded.file_path == existing_path
    assert client.completed_calls == []
    assert client.closed_sessions == ["session-1"]


def test_download_latest_daily_file_retries_after_captcha_rejection(tmp_path: Path) -> None:
    client = FakeBrowserClient(
        content=(
            b"<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>\n"
            b"7203.JP,D,20260429,000000,3065,3101,3057,3067,19390900,0\n"
        ),
        complete_side_effects=[
            BrowserServiceError("Stooq download failed: Stooq CAPTCHA rejected"),
        ],
    )

    downloaded = download_latest_daily_file(
        client,
        tmp_path,
        captcha_solver=lambda _image: "D1TY",
    )

    assert downloaded.file_path.is_file()
    assert client.prepare_calls == 2
    assert client.completed_calls == [
        ("session-1", "D1TY", str(tmp_path), None),
        ("session-2", "D1TY", str(tmp_path), None),
    ]
    assert client.closed_sessions == ["session-1"]


def test_download_latest_daily_file_stops_after_retry_limit(tmp_path: Path) -> None:
    client = FakeBrowserClient(
        content=b"unused",
        complete_side_effects=[
            BrowserServiceError("Stooq download failed: Stooq CAPTCHA rejected"),
            BrowserServiceError("Stooq download failed: Stooq CAPTCHA rejected"),
            BrowserServiceError("Stooq download failed: Stooq CAPTCHA rejected"),
        ],
    )

    with pytest.raises(BrowserServiceError, match="Stooq CAPTCHA rejected"):
        download_latest_daily_file(
            client,
            tmp_path,
            captcha_solver=lambda _image: "D1TY",
        )

    assert client.prepare_calls == 3
    assert client.closed_sessions == ["session-1", "session-2", "session-3"]


def test_download_latest_daily_file_retries_when_ocr_fails(tmp_path: Path) -> None:
    client = FakeBrowserClient(content=b"unused")

    with pytest.raises(StooqCaptchaError, match="Invalid Stooq CAPTCHA OCR result"):
        download_latest_daily_file(
            client,
            tmp_path,
            captcha_solver=lambda _image: "BAD",
        )

    assert client.prepare_calls == 3
    assert client.completed_calls == []
    assert client.closed_sessions == ["session-1", "session-2", "session-3"]


def test_download_latest_daily_file_rejects_unauthorized_payload(tmp_path: Path) -> None:
    client = FakeBrowserClient(content=b"Unauthorized\n", file_name="error.txt")

    with pytest.raises(StooqDownloadError, match="Stooq download was rejected"):
        download_latest_daily_file(
            client,
            tmp_path,
            captcha_solver=lambda _image: "D1TY",
        )
