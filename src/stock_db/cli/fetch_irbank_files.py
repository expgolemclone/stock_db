from __future__ import annotations

import argparse
import logging
import sys
from pathlib import Path

from stock_db.proxy_pool import ProxyPool
from stock_db.paths import IRBANK_DIR, cli_defaults
from stock_db.sources.irbank.downloader import download_irbank_files


def _build_pool(proxy_arg: str) -> ProxyPool:
    if proxy_arg == "direct":
        return ProxyPool.make_direct()
    if proxy_arg.startswith("file:"):
        return ProxyPool.from_file(Path(proxy_arg.removeprefix("file:")))
    return ProxyPool.from_url(proxy_arg)


def main() -> None:
    defaults = cli_defaults("fetch_irbank_files")
    parser = argparse.ArgumentParser(description="IR BANK JSON ファイルをダウンロード")
    parser.add_argument("--years", type=int, default=defaults["years"])
    parser.add_argument("--dest", type=str, default=None)
    parser.add_argument("--interval", type=float, default=defaults["interval_seconds"])
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--proxy", type=str, default=defaults["proxy"],
        help="direct | file:<path> | <proxy-url>",
    )
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    pool = _build_pool(args.proxy)
    dest = Path(args.dest) if args.dest else None
    ok, skip, fail = download_irbank_files(
        pool,
        years=args.years,
        dest=dest,
        interval=args.interval,
        force=args.force,
    )
    if fail > 0:
        print("再実行で失敗ファイルをリトライできます", file=sys.stderr)
    if ok + skip > 0:
        print(f"Import: uv run python -m stock_db import-irbank --dir {dest or IRBANK_DIR}")
    sys.exit(1 if fail > 0 else 0)


if __name__ == "__main__":
    main()
