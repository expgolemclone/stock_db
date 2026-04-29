read README.md

proxy関連は完全に廃止。
EDINET scraping修正後のverifyは targeted tests を優先し、`tests/cli/test_scrape_edinet_reports.py` を含めること。
E2Eで `scrape_edinet_reports` を動かすときは、既存watchdogと競合させないため `STOCK_DB_VAR_DIR` を分離して実行すること。
