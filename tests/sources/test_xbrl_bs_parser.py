"""Tests for XBRL balance sheet parser.

Uses real XBRL fixture files from stock_db/var/raw/edinet/xbrl/.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from stock_db.sources.edinet.xbrl_bs_parser import (
    InventoriesTagMismatchError,
    parse_xbrl_bs,
)


def _xbrl_path(ticker: str) -> str:
    return f"/home/exp/projects/stock_db/var/raw/edinet/xbrl/{ticker}"


class TestParseYoshicon:
    """ヨシコン(5280): 不動産会社。Inventoriesタグなし、RealEstateForSale + RealEstateForSaleInTrustCA から inventories を計算。"""

    @pytest.fixture()
    def parsed(self) -> dict[str, dict[str, float | None]]:
        result = parse_xbrl_bs(_xbrl_path("5280"))
        assert result, "XBRL parse returned empty result"
        return result

    def test_has_three_periods(self, parsed: dict[str, dict[str, float | None]]) -> None:
        assert len(parsed) == 3

    def test_latest_period(self, parsed: dict[str, dict[str, float | None]]) -> None:
        periods = sorted(parsed.keys(), reverse=True)
        assert periods[0] == "2025-03"

    def test_prior_period(self, parsed: dict[str, dict[str, float | None]]) -> None:
        periods = sorted(parsed.keys(), reverse=True)
        assert periods[1] == "2024-03"

    def test_prior2_period(self, parsed: dict[str, dict[str, float | None]]) -> None:
        periods = sorted(parsed.keys(), reverse=True)
        assert periods[2] == "2023-03"

    def test_current_assets(self, parsed: dict[str, dict[str, float | None]]) -> None:
        bs = parsed["2025-03"]
        assert bs["current_assets"] == pytest.approx(38_675_872_000)

    def test_current_liabilities(self, parsed: dict[str, dict[str, float | None]]) -> None:
        bs = parsed["2025-03"]
        assert bs["current_liabilities"] == pytest.approx(15_158_894_000)

    def test_non_current_liabilities(self, parsed: dict[str, dict[str, float | None]]) -> None:
        bs = parsed["2025-03"]
        assert bs["non_current_liabilities"] == pytest.approx(1_468_637_000)

    def test_investment_securities(self, parsed: dict[str, dict[str, float | None]]) -> None:
        bs = parsed["2025-03"]
        assert bs["investment_securities"] == pytest.approx(2_985_654_000)

    def test_inventories_summed_from_components(self, parsed: dict[str, dict[str, float | None]]) -> None:
        """Merchandise(空) + RealEstateForSale + RealEstateForSaleInTrustCA."""
        bs = parsed["2025-03"]
        expected = 28_526_855_000 + 4_447_612_000
        assert bs["inventories"] == pytest.approx(expected)

    def test_prior_year_inventories(self, parsed: dict[str, dict[str, float | None]]) -> None:
        """前期: Merchandise(8,284) + RealEstateForSale(28,439,999) + RealEstateForSaleInTrustCA(なし)."""
        bs = parsed["2024-03"]
        expected = 8_284_000 + 28_439_999_000
        assert bs["inventories"] == pytest.approx(expected)

    def test_cash_and_deposits(self, parsed: dict[str, dict[str, float | None]]) -> None:
        bs = parsed["2025-03"]
        assert bs["cash_and_deposits"] == pytest.approx(3_514_675_000)


class TestParseKudan:
    """Kudan(4425): Inventoriesタグが直接存在するパターン。"""

    @pytest.fixture()
    def parsed(self) -> dict[str, dict[str, float | None]]:
        result = parse_xbrl_bs(_xbrl_path("4425"))
        assert result, "XBRL parse returned empty result"
        return result

    def test_inventories_from_direct_tag(self, parsed: dict[str, dict[str, float | None]]) -> None:
        periods = sorted(parsed.keys(), reverse=True)
        bs = parsed[periods[0]]
        assert bs["inventories"] == pytest.approx(39_840_000)


class TestNoInventoryTags:
    """サービス業等で棚卸資産タグが一切ない場合、inventories = 0."""

    def test_inventories_zero_when_no_tags(self) -> None:
        # 7840 フランスベッドHD: 棚卸資産タグなし
        result = parse_xbrl_bs(_xbrl_path("7840"))
        if not result:
            pytest.skip("No XBRL data for 7840")
        periods = sorted(result.keys(), reverse=True)
        bs = result[periods[0]]
        assert bs.get("inventories", 0) == 0


class TestHeaderOnlyFile:
    """ヘッダーのみのXBRLファイルは空結果を返す."""

    def test_returns_empty_for_tiny_file(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "test.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            "<body><p>header only</p></body></html>",
            encoding="utf-8",
        )
        result = parse_xbrl_bs(str(tmp_path))
        assert result == {}


class TestNegativeValues:
    """△(マイナス)値の処理: sign属性付きの負値."""

    def test_negative_value_parsed(self, tmp_path: Path) -> None:
        xbrl = tmp_path / "S9999.xhtml"
        xbrl.write_text(
            '<html xmlns:ix="http://www.xbrl.org/2008/inlineXBRL">'
            '<ix:nonnumeric contextref="FilingDateInstant" '
            'name="jpdei_cor:CurrentFiscalYearEndDateDEI">2025年3月31日</ix:nonnumeric>'
            '<body><ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:CurrentAssets" decimals="-3" scale="3">'
            "1,000</ix:nonfraction>"
            '<ix:nonfraction contextref="CurrentYearInstant" '
            'name="jppfs_cor:CurrentLiabilities" decimals="-3" scale="3" '
            'sign="negative">500</ix:nonfraction>'
            "</body></html>",
            encoding="utf-8",
        )
        # Pad to pass the 100KB threshold for detailed data
        xbrl.write_text(
            xbrl.read_text(encoding="utf-8") + "<!-- " + "x" * 100_000 + " -->",
            encoding="utf-8",
        )
        result = parse_xbrl_bs(str(tmp_path))
        bs = result.get("2025-03", {})
        assert bs.get("current_assets") == pytest.approx(1_000_000)
        assert bs.get("current_liabilities") == pytest.approx(-500_000)
