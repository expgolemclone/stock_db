from __future__ import annotations

import json
from pathlib import Path

from stock_db.sources.irbank.downloader import build_jobs, is_valid_json_file, year_codes, _default_headers


class TestYearCodes:
    def test_returns_requested_count(self) -> None:
        assert len(year_codes(3)) == 3

    def test_zero_padded_four_digits(self) -> None:
        for code in year_codes(1):
            assert len(code) == 4
            assert code.isdigit()

    def test_ascending_order(self) -> None:
        codes = year_codes(5)
        assert codes == sorted(codes)


class TestIsValidJsonFile:
    def test_nonexistent(self, tmp_path: Path) -> None:
        assert is_valid_json_file(tmp_path / "missing.json") is False

    def test_empty(self, tmp_path: Path) -> None:
        f = tmp_path / "empty.json"
        f.write_bytes(b"")
        assert is_valid_json_file(f) is False

    def test_valid_with_item_key(self, tmp_path: Path) -> None:
        f = tmp_path / "ok.json"
        f.write_bytes(json.dumps({"item": []}).encode())
        assert is_valid_json_file(f) is True

    def test_missing_item_key(self, tmp_path: Path) -> None:
        f = tmp_path / "no_item.json"
        f.write_bytes(json.dumps({"data": 1}).encode())
        assert is_valid_json_file(f) is False

    def test_invalid_json(self, tmp_path: Path) -> None:
        f = tmp_path / "bad.json"
        f.write_bytes(b"{broken")
        assert is_valid_json_file(f) is False


class TestBuildJobs:
    def test_one_year_has_8_jobs(self, tmp_path: Path) -> None:
        # 4 FY + 4 QY = 8
        jobs = build_jobs(1, tmp_path)
        assert len(jobs) == 8

    def test_two_years_has_12_jobs(self, tmp_path: Path) -> None:
        # 2 * 4 FY + 4 QY = 12
        jobs = build_jobs(2, tmp_path)
        assert len(jobs) == 12

    def test_includes_fy_and_qy_urls(self, tmp_path: Path) -> None:
        urls = [url for url, _ in build_jobs(1, tmp_path)]
        assert any("fy-profit-and-loss.json" in u for u in urls)
        assert any("qy-net-sales.json" in u for u in urls)

    def test_creates_quarterly_dir(self, tmp_path: Path) -> None:
        build_jobs(1, tmp_path)
        assert (tmp_path / "quarterly").is_dir()


class TestDefaultHeaders:
    def test_contains_user_agent_from_config(self) -> None:
        headers = _default_headers()

        assert "User-Agent" in headers
        assert "Mozilla" in headers["User-Agent"]

    def test_contains_required_headers(self) -> None:
        headers = _default_headers()

        assert headers["Accept"] == "application/json, text/plain, */*"
        assert headers["Referer"] == "https://irbank.net/download"
