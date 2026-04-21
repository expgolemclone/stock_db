"""Memory-watchdog wrapper for scrape-edinet-reports.

Monitors system memory and restarts the scrape from where it left off
if usage exceeds the threshold (default 70%).
"""

from __future__ import annotations

import logging
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

logger = logging.getLogger("stock_db.cli.scrape_edinet_watchdog")

MAX_MEM_PCT = 70
CHECK_INTERVAL = 30
COOLDOWN_AFTER_KILL = 60


def _mem_pct() -> float:
    """Return current memory usage as a percentage (0-100)."""
    info = {}
    with open("/proc/meminfo") as f:
        for line in f:
            key, val = line.split(":")
            info[key.strip()] = int(val.split()[0])
    total = info["MemTotal"]
    available = info.get("MemAvailable", info["MemFree"] + info.get("Buffers", 0) + info.get("Cached", 0))
    used = total - available
    return (used / total) * 100


def _find_scrape_pids() -> list[int]:
    """Find PIDs of running scrape-edinet-reports processes."""
    pids: list[int] = []
    for entry in os.listdir("/proc"):
        if not entry.isdigit():
            continue
        try:
            cmdline = Path(f"/proc/{entry}/cmdline").read_bytes().decode(errors="replace")
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            continue
        if "scrape-edinet-reports" in cmdline:
            pids.append(int(entry))
    return pids


def _kill_scrape() -> None:
    """Gracefully then forcefully kill scrape processes."""
    pids = _find_scrape_pids()
    if not pids:
        return
    logger.info("Sending SIGTERM to %s", pids)
    for pid in pids:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            logger.debug("PID %d already gone (SIGTERM)", pid)
    # Wait up to 30s for graceful exit
    for _ in range(15):
        if not _find_scrape_pids():
            return
        time.sleep(2)
    # Force kill
    pids = _find_scrape_pids()
    if pids:
        logger.warning("Force killing %s", pids)
        for pid in pids:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                logger.debug("PID %d already gone (SIGKILL)", pid)
        time.sleep(3)


def _start_scrape() -> int:
    """Start scrape-edinet-reports (default skip_existing=true). Returns exit code."""
    proc = subprocess.run(
        [sys.executable, "-m", "stock_db.cli.scrape_edinet_reports"],
        cwd="/home/exp/projects/stock_db",
    )
    return proc.returncode


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
    logger.info("Watchdog started (threshold: %d%%)", MAX_MEM_PCT)

    while True:
        mem = _mem_pct()
        logger.info("Memory: %.0f%%", mem)

        if mem >= MAX_MEM_PCT:
            logger.warning("Memory %.0f%% >= %d%%", mem, MAX_MEM_PCT)
            if _find_scrape_pids():
                _kill_scrape()
                logger.info("Cooling down %ds...", COOLDOWN_AFTER_KILL)
                time.sleep(COOLDOWN_AFTER_KILL)

        if not _find_scrape_pids():
            logger.info("Starting scrape-edinet-reports (skip_existing)...")
            rc = _start_scrape()
            if rc == 0:
                logger.info("Scrape completed successfully.")
                break
            logger.warning("Scrape exited with code %d, will retry.", rc)

        time.sleep(CHECK_INTERVAL)

    logger.info("Watchdog finished.")


if __name__ == "__main__":
    main()
