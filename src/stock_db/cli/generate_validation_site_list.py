from __future__ import annotations

import argparse
import logging
from pathlib import Path

from stock_db.paths import cli_defaults
from stock_db.tools.validation_site_list import generate_validation_sites


def main() -> None:
    defaults = cli_defaults("generate_validation_site_list")
    parser = argparse.ArgumentParser(
        description="proxy 検証用サイトリストを生成",
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=Path(defaults["output"]),
    )
    parser.add_argument("-n", "--count", type=int, default=defaults["count"])
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(message)s")
    generate_validation_sites(args.output, count=args.count)


if __name__ == "__main__":
    main()
