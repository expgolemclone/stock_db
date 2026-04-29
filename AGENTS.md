read README.md

proxy関連は完全に廃止。
EDINET scraping修正後のverifyは targeted tests を優先し、`tests/cli/test_scrape_edinet_reports.py` を含めること。
E2Eで `scrape_edinet_reports` を動かすときは、既存watchdogと競合させないため `STOCK_DB_VAR_DIR` を分離して実行すること。
本番で Phase 2 の強制再取得verifyをするときは、`doc_id` が同一tickerに既に紐づいている銘柄だけを選び、`scrape_all_edinet_reports(..., skip_existing=False)` を直接呼ぶこと。
