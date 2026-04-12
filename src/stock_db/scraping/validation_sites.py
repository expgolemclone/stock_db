from __future__ import annotations

import argparse
import csv
import io
import logging
import re
import zipfile
from pathlib import Path

import requests

logger: logging.Logger = logging.getLogger("stock_db.scraping.validation_sites")

_TRANCO_URL = "https://tranco-list.eu/top-1m.csv.zip"
_DEFAULT_COUNT = 5000

_EXCLUDE_KEYWORDS: frozenset[str] = frozenset([
    "cdn", "dns", "sdk", "pixel", "beacon", "telemetry", "adserv",
    "tracker", "tracking", "analytics",
])

_EXCLUDE_PATTERNS: list[re.Pattern[str]] = [
    re.compile(p)
    for p in [
        r"google",
        r"gmail\.",
        r"gstatic\.",
        r"googleapis\.",
        r"youtube\.",
        r"youtu\.be$",
        r"ggpht\.",
        r"blogspot\.",
        r"gvt\d",
        r"github",
        r"githubusercontent\.",
        r"yahoo",
        r"yimg\.",
        r"irbank\.",
        r"edinet",
        r"cloudfront\.net$",
        r"akamai",
        r"cloudflare",
        r"fastly\.",
        r"jsdelivr\.",
        r"unpkg\.",
        r"polyfill\.",
        r"gtld-servers\.",
        r"root-servers\.",
        r"workers\.dev$",
        r"doubleclick\.",
        r"adsense\.",
        r"adnxs\.",
        r"outbrain\.",
        r"taboola\.",
        r"criteo\.",
        r"rubiconproject\.",
        r"pubmatic\.",
        r"casalemedia\.",
        r"scorecardresearch\.",
        r"hotjar\.",
        r"optimizely\.",
        r"newrelic\.",
        r"sentry\.",
        r"datadoghq\.",
        r"nr-data\.",
        r"smartadserver\.",
        r"appflyer",
        r"appsflyer",
        r"app-analytics",
        r"adjust\.",
        r"branch\.io$",
        r"fbcdn\.",
        r"fbsbx\.",
        r"tfbnw\.",
        r"cdninstagram\.",
        r"twimg\.",
        r"tiktok(cdn|v)",
        r"musical\.ly$",
        r"sc-cdn\.",
        r"snapchat\.",
        r"bytepluscdn\.",
        r"bytefcdn",
        r"douyincdn\.",
        r"whatsapp\.",
        r"telegram\.",
        r"icloud\.",
        r"mzstatic\.",
        r"apple-dns\.",
        r"microsoftonline\.",
        r"msedge\.net$",
        r"msftconnecttest\.",
        r"windowsupdate\.",
        r"azure",
        r"office365\.",
        r"office\.com$",
        r"office\.net$",
        r"sharepoint\.",
        r"onedrive\.",
        r"live\.com$",
        r"live\.net$",
        r"skype\.",
        r"trafficmanager\.net$",
        r"amazonaws\.",
        r"awsstatic\.",
        r"awsdns",
        r"amazonvideo\.",
        r"media-amazon\.",
        r"domaincontrol\.",
        r"parking\.",
        r"keenetic\.",
        r"in-addr\.arpa$",
        r"nominetdns\.",
    ]
]


def should_exclude(domain: str) -> bool:
    if any(kw in domain for kw in _EXCLUDE_KEYWORDS):
        return True
    return any(pat.search(domain) for pat in _EXCLUDE_PATTERNS)


def generate_check_sites(output: Path, *, count: int = _DEFAULT_COUNT) -> int:
    """Tranco top sites から proxy 検証用サイトリストを生成。書き出したドメイン数を返す。"""
    logger.info("Downloading Tranco top 1M list ...")
    resp = requests.get(_TRANCO_URL, timeout=60)
    resp.raise_for_status()

    with zipfile.ZipFile(io.BytesIO(resp.content)) as zf:
        csv_name = zf.namelist()[0]
        raw = zf.read(csv_name).decode()

    domains: list[str] = []
    reader = csv.reader(io.StringIO(raw))
    for _rank, domain in reader:
        if should_exclude(domain):
            continue
        domains.append(domain)
        if len(domains) >= count:
            break

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(domains) + "\n")
    logger.info("Wrote %d domains to %s", len(domains), output)
    return len(domains)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="proxy 検証用サイトリストを生成",
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=Path("config/validation_sites.txt"),
    )
    parser.add_argument("-n", "--count", type=int, default=_DEFAULT_COUNT)
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    generate_check_sites(args.output, count=args.count)
