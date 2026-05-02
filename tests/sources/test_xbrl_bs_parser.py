"""Tests for EDINET XBRL inventories parser."""

from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_bs_parser import (
    InventoriesTagMismatchError,
    is_valid_xbrl_path,
    parse_xbrl_bs,
)


def _xbrl_path(ticker: str) -> str:
    return f"/home/exp/projects/stock_db/var/raw/edinet/xbrl/{ticker}"


class TestRealFixtures:
    def test_yoshicon_sums_real_estate_components(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("5280"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(32_974_467_000)
        assert parsed["2024-03"]["inventories"] == pytest.approx(28_448_283_000)

    def test_kudan_prefers_direct_total(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("4425"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(39_840_000)

    def test_toyota_does_not_double_count_ifrs_components(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("7203"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(4_598_232_000_000)

    def test_nyk_ignores_construction_in_progress(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("9001"))

        assert parsed["2025-03"]["inventories"] == 0.0

    def test_mitsubishi_heavy_sums_work_in_process(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("6597"))

        assert parsed["2025-06"]["inventories"] == pytest.approx(775_897_000)

    def test_obayashi_picks_cns_raw_materials(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("1934"))

        assert parsed["2025-03"]["inventories"] == pytest.approx(2_356_000_000)

    def test_valid_consolidated_xbrl_without_inventory_tags_returns_zero(self) -> None:
        parsed = parse_xbrl_bs(_xbrl_path("1802"))

        assert parsed["2025-03"]["inventories"] == 0.0
        assert parsed["2024-03"]["inventories"] == 0.0

    def test_header_only_html_returns_empty(self) -> None:
        assert parse_xbrl_bs(_xbrl_path("143A")) == {}


class TestValidationHelpers:
    def test_saved_fixture_path_validation(self) -> None:
        path = next(Path(_xbrl_path("5280")).glob("*.xhtml"))
        assert is_valid_xbrl_path(path) is True

    def test_invalid_saved_path_validation(self, tmp_path: Path) -> None:
        invalid = tmp_path / "header.xhtml"
        invalid.write_text("<html><body>header only</body></html>", encoding="utf-8")

        assert is_valid_xbrl_path(invalid) is False


class TestSyntheticCases:
    def test_returns_empty_for_tiny_header_only_file(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "test.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            "<body><p>header only</p></body></html>",
            encoding="utf-8",
        )

        assert parse_xbrl_bs(str(tmp_path)) == {}

    def test_parses_negative_inventory_value(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "S9999.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            '<ix:nonnumeric contextref="FilingDateInstant" '
            'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
            '<body><ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:Inventories" decimals="-3" scale="3" sign="negative">'
            "500</ix:nonfraction></body></html>",
            encoding="utf-8",
        )

        parsed = parse_xbrl_bs(str(tmp_path))
        assert parsed["2025-03"]["inventories"] == pytest.approx(-500_000)

    def test_raises_for_unknown_inventory_like_tag(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "S9998.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            '<ix:nonnumeric contextref="FilingDateInstant" '
            'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
            '<body><ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:MysteryInventoriesCA" decimals="-3" scale="3">'
            "500</ix:nonfraction></body></html>",
            encoding="utf-8",
        )

        with pytest.raises(InventoriesTagMismatchError, match="MysteryInventoriesCA"):
            parse_xbrl_bs(str(tmp_path))
