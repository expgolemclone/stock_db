from __future__ import annotations

from base64 import b64decode
from collections.abc import Iterator

import pytest

import stock_db.sources.stooq.captcha_solver as captcha_solver_module
from stock_db.sources.stooq.captcha_solver import solve_stooq_captcha
from stock_db.sources.stooq.exceptions import StooqCaptchaError

_CAPTCHA_D1TY_BASE64 = (
    "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"
    "SM63CZQAAAAASUVORK5CYII="
)


@pytest.fixture(autouse=True)
def clear_font_caches() -> Iterator[None]:
    captcha_solver_module._font_paths.cache_clear()
    captcha_solver_module._templates_for_char.cache_clear()
    yield
    captcha_solver_module._font_paths.cache_clear()
    captcha_solver_module._templates_for_char.cache_clear()


def test_font_paths_include_linux_paths_and_deduplicate(monkeypatch: pytest.MonkeyPatch) -> None:
    matches = {
        "/nix/store/*/share/fonts/truetype/DejaVuSansCondensed-Bold.ttf": [],
        "/nix/store/*/share/fonts/truetype/DejaVuSans-BoldOblique.ttf": [],
        "/nix/store/*/share/fonts/truetype/FreeSansBoldOblique.ttf": [],
        "/nix/store/*/share/fonts/truetype/LiberationSans-BoldItalic.ttf": [],
        "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed-Bold.ttf": [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed-Bold.ttf"
        ],
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf": [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf"
        ],
        "/usr/share/fonts/truetype/freefont/FreeSansBoldOblique.ttf": [
            "/usr/share/fonts/truetype/freefont/FreeSansBoldOblique.ttf"
        ],
        "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf": [
            "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf"
        ],
        "/usr/share/fonts/truetype/liberation2/LiberationSans-BoldItalic.ttf": [
            "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf"
        ],
    }

    monkeypatch.setattr(captcha_solver_module, "glob", lambda pattern: matches.get(pattern, []))

    assert captcha_solver_module._font_paths() == (
        "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-BoldOblique.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSansBoldOblique.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-BoldItalic.ttf",
    )


def test_font_paths_raises_when_no_fonts_are_found(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(captcha_solver_module, "glob", lambda pattern: [])

    with pytest.raises(StooqCaptchaError, match="No OCR fonts found"):
        captcha_solver_module._font_paths()


def test_solve_stooq_captcha() -> None:
    image_bytes = b64decode(_CAPTCHA_D1TY_BASE64)

    assert solve_stooq_captcha(image_bytes) == "D1TY"
