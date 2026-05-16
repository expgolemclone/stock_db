from __future__ import annotations

from stock_db.paths import cli_defaults, magic_numbers


class TestMagicNumbers:
    def test_loads_proxy_section(self) -> None:
        cfg = magic_numbers()

        assert cfg["proxy"]["max_failures"] == 2

    def test_loads_browser_section(self) -> None:
        cfg = magic_numbers()

        assert cfg["browser"]["startup_poll_interval"] == 0.25
        assert cfg["browser"]["page_timeout"] == 120000


class TestCliDefaults:
    def test_generate_validation_site_list_defaults(self) -> None:
        defaults = cli_defaults("generate_validation_site_list")

        assert defaults["count"] == 5000
        assert defaults["output"] == "var/generated/validation_sites.txt"

    def test_scrape_stooq_prices_defaults(self) -> None:
        defaults = cli_defaults("scrape_stooq_prices")

        assert defaults["pool_size"] == 1
        assert defaults["headless"] is False
        assert defaults["disable_xvfb"] is True
        assert defaults["output_dir"] == "var/raw/stooq"

    def test_parse_xbrl_financials_defaults(self) -> None:
        defaults = cli_defaults("parse_xbrl_financials")

        assert defaults["skip_existing"] is True
