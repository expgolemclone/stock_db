from __future__ import annotations

from collections.abc import Sequence

from stock_db.cli.scrape_edinet_reports import main_step2


def main(argv: Sequence[str] | None = None) -> int:
    return main_step2(argv)


if __name__ == "__main__":
    raise SystemExit(main())
