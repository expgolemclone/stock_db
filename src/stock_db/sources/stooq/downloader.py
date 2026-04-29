from __future__ import annotations

import base64
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from stock_db.browser_client.client import BrowserServiceClient
from stock_db.sources.stooq.captcha_solver import solve_stooq_captcha
from stock_db.sources.stooq.exceptions import StooqCaptchaError, StooqDownloadError


@dataclass(frozen=True, slots=True)
class DownloadedStooqDailyFile:
    date: str
    label: str
    file_path: Path


def _validate_download(file_path: Path) -> None:
    if not file_path.is_file():
        raise StooqDownloadError(f"Downloaded file not found: {file_path}")

    preview = file_path.read_bytes()[:128].decode("utf-8", errors="ignore").strip().lower()
    if file_path.name == "error.txt" or preview.startswith("unauthorized"):
        raise StooqDownloadError(f"Stooq download was rejected: {file_path}")


def download_latest_daily_file(
    client: BrowserServiceClient,
    output_dir: Path,
    *,
    timeout: int | None = None,
    captcha_solver: Callable[[bytes], str] = solve_stooq_captcha,
) -> DownloadedStooqDailyFile:
    output_dir.mkdir(parents=True, exist_ok=True)

    prepared = client.prepare_stooq_daily_download(timeout=timeout)
    completed = False
    try:
        try:
            captcha_png = base64.b64decode(prepared.captcha_png_base64, validate=True)
        except ValueError as exc:
            raise StooqCaptchaError("Failed to decode Stooq CAPTCHA image") from exc

        captcha_code = captcha_solver(captcha_png)
        if len(captcha_code) != 4 or not captcha_code.isalnum():
            raise StooqCaptchaError(f"Invalid Stooq CAPTCHA OCR result: {captcha_code!r}")

        file_path = Path(
            client.complete_stooq_daily_download(
                prepared.session_id,
                captcha_code,
                str(output_dir),
                timeout=timeout,
            )
        )
        completed = True
    finally:
        if not completed:
            client.close_stooq_session(prepared.session_id)

    _validate_download(file_path)
    return DownloadedStooqDailyFile(
        date=prepared.date,
        label=prepared.label,
        file_path=file_path,
    )
