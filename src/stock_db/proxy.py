from __future__ import annotations

import logging
import random
import threading
import time
from pathlib import Path

logger: logging.Logger = logging.getLogger("stock_db.proxy")

_DEFAULT_MAX_FAILURES = 2


class ProxyUnavailableError(RuntimeError):
    pass


def random_delay(min_s: float = 1.0, max_s: float = 5.0) -> None:
    time.sleep(random.uniform(min_s, max_s))


def _proxy_url(addr: str, proto: str) -> str:
    if proto == "socks5":
        return f"socks5h://{addr}"
    return f"http://{addr}"


class ProxyPool:
    def __init__(
        self,
        proxies: list[tuple[str, str]] | list[tuple[str, str, str]],
        *,
        direct: bool = False,
        max_failures: int = _DEFAULT_MAX_FAILURES,
    ) -> None:
        self._lock: threading.Lock = threading.Lock()
        self._proxies: list[tuple[str, str, str]] = [
            (*t, "") if len(t) == 2 else t  # type: ignore[misc]
            for t in proxies
        ]
        self._index: int = 0
        self._failures: dict[str, int] = {}
        self._max_failures: int = max_failures
        self._direct: bool = direct

    @property
    def direct(self) -> bool:
        return self._direct

    @classmethod
    def from_url(cls, url: str) -> ProxyPool:
        for scheme in ("socks5h://", "socks5://"):
            if url.startswith(scheme):
                return cls([(url.removeprefix(scheme), "socks5")])
        addr: str = url.removeprefix("http://").removeprefix("https://")
        return cls([(addr, "http")])

    @classmethod
    def from_file(cls, path: Path) -> ProxyPool:
        entries: list[tuple[str, str, str]] = []
        for line in path.read_text().splitlines():
            line = line.strip()
            if not line:
                continue
            parts = line.split(":")
            if len(parts) == 4:
                host, port, user, pw = parts
                entries.append((f"{host}:{port}", "http", f"{user}:{pw}"))
            elif len(parts) == 2:
                entries.append((line, "http", ""))
        return cls(entries)

    @classmethod
    def make_direct(cls) -> ProxyPool:
        return cls([], direct=True)

    def get(self) -> str | None:
        with self._lock:
            if not self._proxies:
                return None
            addr, proto, auth = self._proxies[self._index % len(self._proxies)]
            scheme: str = "socks5h" if proto == "socks5" else "http"
            if auth:
                return f"{scheme}://{auth}@{addr}"
            return f"{scheme}://{addr}"

    @property
    def size(self) -> int:
        with self._lock:
            return len(self._proxies)

    def _rotate_locked(self) -> None:
        if self._proxies:
            self._index += 1
            addr, proto, _auth = self._proxies[self._index % len(self._proxies)]
            logger.debug("Rotated to proxy: %s", _proxy_url(addr, proto))

    def rotate(self) -> None:
        with self._lock:
            self._rotate_locked()

    def report_failure(self) -> None:
        with self._lock:
            if not self._proxies:
                return
            addr, _proto, _auth = self._proxies[self._index % len(self._proxies)]
            self._failures[addr] = self._failures.get(addr, 0) + 1
            if self._failures[addr] >= self._max_failures:
                logger.info(
                    "Proxy %s failed %d times, removing (pool size: %d -> %d)",
                    addr, self._max_failures, len(self._proxies),
                    len(self._proxies) - 1,
                )
                self._proxies = [p for p in self._proxies if p[0] != addr]
                if self._proxies:
                    self._index = self._index % len(self._proxies)
            else:
                self._rotate_locked()

    @property
    def exhausted(self) -> bool:
        with self._lock:
            return len(self._proxies) == 0

    def split(self, n: int) -> list[ProxyPool]:
        if n <= 0:
            raise ValueError("n must be positive")
        with self._lock:
            buckets: list[list[tuple[str, str, str]]] = [[] for _ in range(n)]
            for i, entry in enumerate(self._proxies):
                buckets[i % n].append(entry)
        return [ProxyPool(b, direct=self._direct) for b in buckets]

    def __repr__(self) -> str:
        with self._lock:
            count: int = len(self._proxies)
            if count:
                addr, proto, auth = self._proxies[self._index % count]
                scheme = "socks5h" if proto == "socks5" else "http"
                current = f"{scheme}://{auth}@{addr}" if auth else f"{scheme}://{addr}"
            else:
                current = None
        return f"ProxyPool(count={count}, current={current})"
