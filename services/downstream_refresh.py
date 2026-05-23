"""Run downstream project JSON refresh after stock price update.

Called by stock-db-downstream-refresh.timer at 16:05 JST.
"""

from __future__ import annotations

import subprocess
import sys
from datetime import datetime
from pathlib import Path

UV = Path("/etc/profiles/per-user/exp/bin/uv")

PROJECTS: list[tuple[Path, list[str]]] = [
    (
        Path("/home/exp/projects/formula_screening"),
        [
            str(UV), "run", "python", "-m", "formula_screening", "screen",
            "-s", "strategies/net_cash_fcf.toml", "-t", "all",
            "--json", "docs/assets/screening.json",
        ],
    ),
    (
        Path("/home/exp/projects/land_value_research"),
        [str(UV), "run", "python", "-m", "src.web", "--export-github-pages"],
    ),
    (
        Path("/home/exp/projects/invest_like_legends"),
        [str(UV), "run", "python", "scripts/enrich_investors.py"],
    ),
]


def log(msg: str) -> None:
    ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{ts}] {msg}", flush=True)


def main() -> int:
    rc = 0
    for cwd, cmd in PROJECTS:
        name = cwd.name
        log(f"{name}: starting...")
        result = subprocess.run(cmd, cwd=str(cwd))
        if result.returncode == 0:
            log(f"{name}: done")
        else:
            log(f"{name}: FAILED (exit {result.returncode})")
            rc = 1
    log("All downstream refresh steps completed.")
    return rc


if __name__ == "__main__":
    raise SystemExit(main())
