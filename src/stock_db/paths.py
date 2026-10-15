from __future__ import annotations

import os
import tomllib
from functools import lru_cache
from pathlib import Path

PROJECT_ROOT: Path = Path(__file__).resolve().parent.parent.parent
VAR_DIR: Path = Path(os.environ.get("STOCK_DB_VAR_DIR", str(PROJECT_ROOT / "var")))
STOCKS_DB_PATH: Path = VAR_DIR / "db" / "stocks.db"
STOOQ_DIR: Path = VAR_DIR / "raw" / "stooq"
BROWSER_SERVICE_DIR: Path = PROJECT_ROOT / "services" / "browser"


def _load_toml(name: str) -> dict:
    path = PROJECT_ROOT / "config" / name
    with open(path, "rb") as f:
        return tomllib.load(f)


@lru_cache(maxsize=1)
def magic_numbers() -> dict:
    return _load_toml("magic_numbers.toml")


@lru_cache(maxsize=1)
def _cli_defaults_data() -> dict:
    return _load_toml("cli_defaults.toml")


def cli_defaults(section: str) -> dict:
    return dict(_cli_defaults_data()[section])


@lru_cache(maxsize=1)
def edinet_phase1_config() -> dict:
    return _load_toml("edinet_phase1.toml")


@lru_cache(maxsize=1)
def jpx_market_holidays() -> dict:
    return _load_toml("jpx_market_holidays.toml")
