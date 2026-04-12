from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.proxy import ProxyPool, random_delay


class TestProxyPoolDirect:
    def test_direct_pool_returns_none(self) -> None:
        pool = ProxyPool([], direct=True)

        assert pool.get() is None
        assert pool.direct is True

    def test_make_direct(self) -> None:
        pool = ProxyPool.make_direct()

        assert pool.direct is True
        assert pool.size == 0


class TestProxyPoolFromUrl:
    def test_http_proxy(self) -> None:
        pool = ProxyPool.from_url("http://1.2.3.4:8080")

        assert pool.get() == "http://1.2.3.4:8080"
        assert pool.size == 1

    def test_socks5_proxy(self) -> None:
        pool = ProxyPool.from_url("socks5://1.2.3.4:1080")

        assert pool.get() == "socks5h://1.2.3.4:1080"


class TestProxyPoolFromFile:
    def test_host_port_lines(self, tmp_path: Path) -> None:
        proxy_file = tmp_path / "proxies.txt"
        proxy_file.write_text("1.2.3.4:8080\n5.6.7.8:3128\n")

        pool = ProxyPool.from_file(proxy_file)

        assert pool.size == 2

    def test_host_port_user_pass_lines(self, tmp_path: Path) -> None:
        proxy_file = tmp_path / "proxies.txt"
        proxy_file.write_text("1.2.3.4:8080:user:pass\n")

        pool = ProxyPool.from_file(proxy_file)

        result = pool.get()
        assert result is not None
        assert "user:pass" in result


class TestProxyPoolRotation:
    def test_rotate_changes_proxy(self) -> None:
        pool = ProxyPool([("1.2.3.4:8080", "http"), ("5.6.7.8:3128", "http")])

        first = pool.get()
        pool.rotate()
        second = pool.get()

        assert first != second

    def test_report_failure_removes_after_max(self) -> None:
        pool = ProxyPool([("1.2.3.4:8080", "http")], max_failures=2)

        pool.report_failure()
        assert pool.size == 1
        pool.report_failure()

        assert pool.size == 0

    def test_exhausted_after_all_removed(self) -> None:
        pool = ProxyPool([("1.2.3.4:8080", "http")], max_failures=1)

        pool.report_failure()

        assert pool.exhausted is True


class TestProxyPoolSplit:
    def test_split_distributes_proxies(self) -> None:
        pool = ProxyPool([
            ("1.2.3.4:8080", "http"),
            ("5.6.7.8:3128", "http"),
            ("9.10.11.12:1080", "socks5"),
            ("13.14.15.16:8080", "http"),
        ])

        sub_pools = pool.split(2)

        assert len(sub_pools) == 2
        assert sub_pools[0].size == 2
        assert sub_pools[1].size == 2

    def test_split_invalid_n_raises(self) -> None:
        pool = ProxyPool([])

        with pytest.raises(ValueError, match="positive"):
            pool.split(0)


class TestProxyPoolRepr:
    def test_repr_with_proxies(self) -> None:
        pool = ProxyPool([("1.2.3.4:8080", "http")])

        result = repr(pool)

        assert "count=1" in result
        assert "1.2.3.4:8080" in result

    def test_repr_empty(self) -> None:
        pool = ProxyPool([])

        result = repr(pool)

        assert "count=0" in result


class TestRandomDelay:
    def test_sleeps_within_range(self) -> None:
        import time
        t0 = time.monotonic()

        random_delay(0.01, 0.02)

        elapsed = time.monotonic() - t0
        assert 0.01 <= elapsed < 0.1
