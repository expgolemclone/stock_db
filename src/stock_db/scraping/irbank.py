from __future__ import annotations

import argparse
import json
import logging
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import requests

from stock_db.config import IRBANK_DIR
from stock_db.proxy import ProxyPool, random_delay

logger: logging.Logger = logging.getLogger("stock_db.scraping.irbank")

_BASE_URL = "https://f.irbank.net/files"
_FY_FILES = [
    "fy-profit-and-loss.json",
    "fy-balance-sheet.json",
    "fy-cash-flow-statement.json",
    "fy-stock-dividend.json",
]
_QY_FILES = [
    "qy-net-sales.json",
    "qy-operating-income.json",
    "qy-ordinary-income.json",
    "qy-profit-loss.json",
]
_HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36"
    ),
    "Accept": "application/json, text/plain, */*",
    "Accept-Encoding": "gzip, deflate, br",
    "Referer": "https://irbank.net/download",
}
_DEFAULT_MAX_TRIES = 5
_DEFAULT_RATE_LIMIT_WAIT = 30.0


def year_codes(years: int) -> list[str]:
    latest = datetime.now(timezone.utc).year
    return [f"{y % 100:04d}" for y in range(latest - years + 1, latest + 1)]


def _is_rate_limited(resp: requests.Response) -> bool:
    content_type = resp.headers.get("Content-Type", "")
    return "html" in content_type or resp.content.lstrip()[:1] == b"<"


def _try_download(
    url: str,
    proxy_url: str | None,
    *,
    timeout: float = 15,
) -> bytes | None:
    kwargs: dict = {"headers": _HEADERS, "timeout": timeout}
    if proxy_url:
        kwargs["proxies"] = {"http": proxy_url, "https": proxy_url}
    resp = requests.get(url, **kwargs)
    if resp.status_code != 200 or _is_rate_limited(resp):
        return None
    # レスポンスが有効な JSON であることを確認する
    json.loads(resp.content)
    return resp.content


def _download_file(
    url: str,
    dest: Path,
    pool: ProxyPool,
    *,
    max_tries: int = _DEFAULT_MAX_TRIES,
    rate_limit_wait: float = _DEFAULT_RATE_LIMIT_WAIT,
    timeout: float = 15,
) -> bool:
    for _ in range(max_tries):
        proxy_url = pool.get()
        label = proxy_url or "direct"
        try:
            content = _try_download(url, proxy_url, timeout=timeout)
        except (requests.RequestException, json.JSONDecodeError, UnicodeDecodeError) as exc:
            logger.debug("Download error via %s: %s", label, exc)
            pool.report_failure()
            continue
        if content is not None:
            dest.write_bytes(content)
            logger.info("OK via %s", label)
            return True
        logger.info("Rate-limited (%s), rotating + waiting %.0fs", label, rate_limit_wait)
        pool.report_failure()
        time.sleep(rate_limit_wait)

    logger.warning("FAILED: %s", url)
    return False


def is_valid_json_file(path: Path) -> bool:
    if not path.exists() or path.stat().st_size == 0:
        return False
    try:
        data = json.loads(path.read_bytes())
        return isinstance(data, dict) and "item" in data
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        logger.debug("Invalid JSON file %s: %s", path, exc)
        return False


def build_jobs(years: int, dest: Path) -> list[tuple[str, Path]]:
    codes = year_codes(years)
    jobs: list[tuple[str, Path]] = []
    for code in codes:
        out_dir = dest / code
        out_dir.mkdir(parents=True, exist_ok=True)
        for filename in _FY_FILES:
            jobs.append((f"{_BASE_URL}/{code}/{filename}", out_dir / filename))
    qy_dir = dest / "quarterly"
    qy_dir.mkdir(parents=True, exist_ok=True)
    for filename in _QY_FILES:
        jobs.append((f"{_BASE_URL}/0000/{filename}", qy_dir / filename))
    return jobs


def download_irbank_files(
    pool: ProxyPool,
    *,
    years: int = 5,
    dest: Path | None = None,
    interval: float = 1.0,
    force: bool = False,
    max_tries: int = _DEFAULT_MAX_TRIES,
    rate_limit_wait: float = _DEFAULT_RATE_LIMIT_WAIT,
) -> tuple[int, int, int]:
    """IR BANK JSON ファイルをダウンロード。(ok, skip, fail) を返す。"""
    effective_dest = dest or IRBANK_DIR
    jobs = build_jobs(years, effective_dest)

    if force:
        download_jobs = jobs
        skip = 0
    else:
        download_jobs = [(url, t) for url, t in jobs if not is_valid_json_file(t)]
        skip = len(jobs) - len(download_jobs)

    total = len(download_jobs)
    logger.info(
        "Downloading %d files (%d skipped) to %s",
        total, skip, effective_dest,
    )

    ok = 0
    fail = 0
    for count, (url, target) in enumerate(download_jobs, 1):
        logger.info("[%d/%d] %s", count, total, url)
        if _download_file(
            url, target, pool,
            max_tries=max_tries,
            rate_limit_wait=rate_limit_wait,
        ):
            ok += 1
        else:
            fail += 1
        if count < total:
            random_delay(interval * 0.5, interval * 1.5)

    logger.info("Done: %d downloaded, %d skipped, %d failed", ok, skip, fail)
    return ok, skip, fail


def _build_pool(proxy_arg: str) -> ProxyPool:
    if proxy_arg == "direct":
        return ProxyPool.make_direct()
    if proxy_arg.startswith("file:"):
        return ProxyPool.from_file(Path(proxy_arg.removeprefix("file:")))
    return ProxyPool.from_url(proxy_arg)


def main() -> None:
    parser = argparse.ArgumentParser(description="IR BANK JSON ファイルをダウンロード")
    parser.add_argument("--years", type=int, default=5)
    parser.add_argument("--dest", type=str, default=None)
    parser.add_argument("--interval", type=float, default=1.0)
    parser.add_argument("--force", action="store_true")
    parser.add_argument(
        "--proxy", type=str, default="direct",
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
