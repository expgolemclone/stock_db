from __future__ import annotations

import os
from pathlib import Path

DATA_DIR = Path(os.environ.get("STOCK_DB_DATA", "data"))
STOCKS_DB_PATH = DATA_DIR / "stocks.db"
IRBANK_DIR = DATA_DIR / "irbank"
