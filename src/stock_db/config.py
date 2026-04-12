from __future__ import annotations

import os
from pathlib import Path

_PACKAGE_ROOT = Path(__file__).resolve().parent.parent.parent

DATA_DIR = Path(os.environ.get("STOCK_DB_DATA", _PACKAGE_ROOT / "data"))
STOCKS_DB_PATH = DATA_DIR / "stocks.db"
