from __future__ import annotations

from stock_db.paths import cli_defaults, magic_numbers


class TestMagicNumbers:
    def test_loads_proxy_section(self) -> None:
        cfg = magic_numbers()

        assert cfg["proxy"]["max_failures"] == 2

    def test_loads_irbank_section(self) -> None:
        cfg = magic_numbers()

        assert cfg["irbank"]["max_tries"] == 5
        assert cfg["irbank"]["rate_limit_wait"] == 30.0

    def test_loads_browser_section(self) -> None:
        cfg = magic_numbers()

        assert cfg["browser"]["startup_poll_interval"] == 0.25


class TestCliDefaults:
    def test_fetch_irbank_files_defaults(self) -> None:
        defaults = cli_defaults("fetch_irbank_files")

        assert defaults["years"] == 5
        assert defaults["interval_seconds"] == 1.0
        assert defaults["proxy"] == "direct"
        assert defaults["output_dir"] == "var/raw/irbank"

    def test_generate_validation_site_list_defaults(self) -> None:
        defaults = cli_defaults("generate_validation_site_list")

        assert defaults["count"] == 5000
        assert defaults["output"] == "var/generated/validation_sites.txt"
