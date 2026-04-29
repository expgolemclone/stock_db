from __future__ import annotations

import base64
from pathlib import Path

import pytest

from stock_db.browser_client.client import StooqDailyDownloadSession
from stock_db.sources.stooq import (
    StooqCaptchaError,
    StooqDownloadError,
    download_latest_daily_file,
)


class FakeBrowserClient:
    def __init__(self, *, content: bytes, file_name: str = "0429_d.csv") -> None:
        self._content = content
        self._file_name = file_name
        self.completed_calls: list[tuple[str, str, str, int | None]] = []
        self.closed_sessions: list[str] = []

    def prepare_stooq_daily_download(self, *, timeout: int | None = None) -> StooqDailyDownloadSession:
        return StooqDailyDownloadSession(
            session_id="session-1",
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


def test_download_latest_daily_file_closes_session_when_ocr_fails(tmp_path: Path) -> None:
    client = FakeBrowserClient(content=b"unused")

    with pytest.raises(StooqCaptchaError, match="Invalid Stooq CAPTCHA OCR result"):
        download_latest_daily_file(
            client,
            tmp_path,
            captcha_solver=lambda _image: "BAD",
        )

    assert client.completed_calls == []
    assert client.closed_sessions == ["session-1"]


def test_download_latest_daily_file_rejects_unauthorized_payload(tmp_path: Path) -> None:
    client = FakeBrowserClient(content=b"Unauthorized\n", file_name="error.txt")

    with pytest.raises(StooqDownloadError, match="Stooq download was rejected"):
        download_latest_daily_file(
            client,
            tmp_path,
            captcha_solver=lambda _image: "D1TY",
        )
