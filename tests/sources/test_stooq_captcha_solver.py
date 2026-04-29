from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.stooq import captcha_solver as captcha_solver_module
from stock_db.sources.stooq.captcha_solver import solve_stooq_captcha
from stock_db.sources.stooq.exceptions import StooqCaptchaError


def test_solve_stooq_captcha_uses_2captcha(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    captured: dict[str, object] = {}

    class FakeTwoCaptcha:
        def __init__(self, api_key: str) -> None:
            captured["api_key"] = api_key

        def normal(self, file: str, **kwargs: object) -> dict[str, str]:
            captured["file"] = file
            captured["kwargs"] = kwargs
            return {"code": "d1ty"}

    def fake_named_tempfile(*, suffix: str, delete: bool) -> object:
        del suffix, delete
        path = tmp_path / "captcha.png"

        class _Tmp:
            name = str(path)

            def write(self, data: bytes) -> None:
                path.write_bytes(data)

            def __enter__(self) -> "_Tmp":
                return self

            def __exit__(self, exc_type: object, exc: object, tb: object) -> None:
                return None

        return _Tmp()

    monkeypatch.setenv("STOCK_DB_2CAPTCHA_API_KEY", "secret")
    monkeypatch.setattr(captcha_solver_module, "TwoCaptcha", FakeTwoCaptcha)
    monkeypatch.setattr(captcha_solver_module, "NamedTemporaryFile", fake_named_tempfile)

    code = solve_stooq_captcha(b"captcha-bytes")

    assert code == "D1TY"
    assert captured["api_key"] == "secret"
    assert Path(str(captured["file"])).name == "captcha.png"
    assert captured["kwargs"] == {
        "minLen": 4,
        "maxLen": 4,
        "caseSensitive": 0,
        "hintText": "Enter the 4 red letters and numbers.",
    }
    assert not (tmp_path / "captcha.png").exists()


def test_solve_stooq_captcha_requires_api_key(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("STOCK_DB_2CAPTCHA_API_KEY", raising=False)

    with pytest.raises(StooqCaptchaError, match="STOCK_DB_2CAPTCHA_API_KEY is not set"):
        solve_stooq_captcha(b"captcha-bytes")
