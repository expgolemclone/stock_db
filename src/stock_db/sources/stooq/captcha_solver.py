from __future__ import annotations

import os
import re
from pathlib import Path
from tempfile import NamedTemporaryFile

from twocaptcha import (
    ApiException,
    NetworkException,
    TimeoutException,
    TwoCaptcha,
    ValidationException,
)

from stock_db.sources.stooq.exceptions import StooqCaptchaError

_API_KEY_ENV = "STOCK_DB_2CAPTCHA_API_KEY"


def _normalize_code(raw_code: object) -> str:
    normalized = re.sub(r"[^0-9A-Za-z]", "", str(raw_code)).upper()
    if len(normalized) != 4:
        raise StooqCaptchaError(f"2Captcha returned invalid Stooq code: {raw_code!r}")
    return normalized


def solve_stooq_captcha(
    image_bytes: bytes,
    *,
    api_key: str | None = None,
) -> str:
    effective_api_key = api_key or os.environ.get(_API_KEY_ENV)
    if not effective_api_key:
        raise StooqCaptchaError(f"{_API_KEY_ENV} is not set")

    with NamedTemporaryFile(suffix=".png", delete=False) as tmp:
        tmp.write(image_bytes)
        image_path = Path(tmp.name)

    try:
        solver = TwoCaptcha(effective_api_key)
        result = solver.normal(
            str(image_path),
            minLen=4,
            maxLen=4,
            caseSensitive=0,
            hintText="Enter the 4 red letters and numbers.",
        )
        return _normalize_code(result.get("code"))
    except (
        ValidationException,
        NetworkException,
        ApiException,
        TimeoutException,
    ) as exc:
        raise StooqCaptchaError(f"2Captcha failed for Stooq: {exc}") from exc
    finally:
        image_path.unlink(missing_ok=True)
