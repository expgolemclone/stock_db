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
SCRAPE_TIMEOUT = 1800  # 30分
_PID_FILE = Path("/tmp/scrape_edinet_watchdog.pid")
_SCRAPE_CMD = [sys.executable, "-m", "stock_db.cli.scrape_edinet_reports"]
_SCRAPE_CWD = "/home/exp/projects/stock_db"


def _kill_previous_instance() -> None:
    """Kill a previous watchdog instance if one is running."""
    if not _PID_FILE.exists():
        return
    try:
        old_pid = int(_PID_FILE.read_text().strip())
    except (ValueError, OSError):
        logger.debug("Invalid PID file, ignoring")
        return
    try:
        os.kill(old_pid, signal.SIGTERM)
    except ProcessLookupError:
        logger.debug("Previous instance PID %d already gone", old_pid)
    else:
        logger.info("Killed previous watchdog instance (PID %d)", old_pid)
        time.sleep(2)


def _write_pid() -> None:
    """Write current PID to the PID file."""
    _PID_FILE.write_text(str(os.getpid()))


def _remove_pid() -> None:
    """Remove the PID file on exit."""
    try:
        _PID_FILE.unlink()
    except FileNotFoundError:
        logger.debug("PID file already removed")


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



def _kill_proc_tree(proc: subprocess.Popen[bytes]) -> None:
    """Kill child and all descendants in the same session.

    Grandchildren (browser server, chrome) often create their own PGID,
    so os.killpg alone is insufficient. After killing the direct child,
    scan /proc for orphaned descendants in the same session and force-kill them.
    """
    pid = proc.pid
    try:
        proc.terminate()
    except ProcessLookupError:
        logger.debug("PID %d already gone (SIGTERM)", pid)
        _kill_orphans(pid)
        return
    try:
        proc.wait(timeout=30)
        logger.debug("PID %d exited gracefully", pid)
    except subprocess.TimeoutExpired:
        logger.debug("PID %d did not exit in 30s, SIGKILL", pid)
        try:
            proc.kill()
        except ProcessLookupError:
            logger.debug("PID %d already gone (SIGKILL)", pid)
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            logger.warning("PID %d still alive after SIGKILL", pid)
    _kill_orphans(pid)


def _kill_orphans(session_leader_pid: int) -> None:
    """Force-kill orphaned browser server processes (PPID=1, re-parented by init).

    Grandchildren like browser/server.js call setsid() on parent death,
    creating a new session, so SID-based matching fails. Instead, find
    server.js processes whose parent is init (PID 1) and were started
    around the same time as the scrape process.
    """
    orphans: list[int] = []
    try:
        scrape_start_time = int(Path(f"/proc/{session_leader_pid}/stat").read_text().split(" ")[21])
    except (ValueError, OSError):
        logger.debug("Cannot read starttime for PID %d, using wide window", session_leader_pid)
        scrape_start_time = 0

    for entry in os.listdir("/proc"):
        if not entry.isdigit() or int(entry) == os.getpid():
            continue
        try:
            cmdline = Path(f"/proc/{entry}/cmdline").read_bytes().decode(errors="replace")
            stat = Path(f"/proc/{entry}/stat").read_text().split(" ")
            ppid = int(stat[3])
            if ppid != 1:
                continue
            if "server.js" not in cmdline:
                continue
            # Only kill if started after the scrape (within same timeframe)
            start_time = int(stat[21])
            if scrape_start_time == 0 or start_time >= scrape_start_time - 100:
                orphans.append(int(entry))
        except (ValueError, OSError):
            continue
    if not orphans:
        return
    logger.info("Killing %d orphaned server.js processes: %s", len(orphans), orphans)
    for opid in orphans:
        try:
            os.kill(opid, signal.SIGKILL)
        except ProcessLookupError:
            logger.debug("Orphan PID %d already gone", opid)


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(message)s")
    _kill_previous_instance()
    _write_pid()
    logger.info("Watchdog started (threshold: %d%%, timeout: %ds)", MAX_MEM_PCT, SCRAPE_TIMEOUT)

    scrape_proc: subprocess.Popen[bytes] | None = None
    scrape_start: float = 0.0

    try:
        while True:
            mem = _mem_pct()
            logger.info("Memory: %.0f%%", mem)

            # メモリ超過時はscrapeをkill
            if mem >= MAX_MEM_PCT and scrape_proc and scrape_proc.poll() is None:
                logger.warning("Memory %.0f%% >= %d%%, killing scrape", mem, MAX_MEM_PCT)
                _kill_proc_tree(scrape_proc)
                scrape_proc = None
                logger.info("Cooling down %ds...", COOLDOWN_AFTER_KILL)
                time.sleep(COOLDOWN_AFTER_KILL)

            # scrape終了確認
            if scrape_proc is not None:
                rc = scrape_proc.poll()
                if rc is not None:
                    scrape_proc.wait()
                    if rc == 0:
                        logger.info("Scrape completed successfully.")
                        break
                    logger.warning("Scrape exited with code %d, will retry.", rc)
                    scrape_proc = None
                elif time.monotonic() - scrape_start > SCRAPE_TIMEOUT:
                    logger.warning("Scrape timeout (%ds), killing", SCRAPE_TIMEOUT)
                    _kill_proc_tree(scrape_proc)
                    scrape_proc = None

            # scrape開始
            if scrape_proc is None:
                logger.info("Starting scrape-edinet-reports...")
                scrape_proc = subprocess.Popen(_SCRAPE_CMD, cwd=_SCRAPE_CWD, start_new_session=True)
                scrape_start = time.monotonic()

            time.sleep(CHECK_INTERVAL)
    finally:
        if scrape_proc and scrape_proc.poll() is None:
            _kill_proc_tree(scrape_proc)
        _remove_pid()

    logger.info("Watchdog finished.")


if __name__ == "__main__":
    main()
