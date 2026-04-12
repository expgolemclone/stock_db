from __future__ import annotations

from stock_db.scraping.check_sites import should_exclude


class TestShouldExclude:
    def test_excludes_google(self) -> None:
        assert should_exclude("google.com") is True

    def test_excludes_youtube(self) -> None:
        assert should_exclude("youtube.com") is True

    def test_excludes_cdn_keyword(self) -> None:
        assert should_exclude("cdn.example.com") is True

    def test_excludes_tracking_keyword(self) -> None:
        assert should_exclude("tracking.example.com") is True

    def test_excludes_irbank(self) -> None:
        assert should_exclude("irbank.net") is True

    def test_excludes_cloudflare(self) -> None:
        assert should_exclude("cloudflare.com") is True

    def test_allows_normal_domain(self) -> None:
        assert should_exclude("example.com") is False

    def test_allows_wikipedia(self) -> None:
        assert should_exclude("wikipedia.org") is False
