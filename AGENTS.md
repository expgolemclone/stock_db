read README.md

## stooq

- stooqの日次価格取得はbrowser service経由で行い, CAPTCHAはローカルOCRで解くこと.
- stooqの日次価格取り込みでは`prices.close`のみを使い, `prices.volume`は保存しないこと.

proxy関連は完全に廃止。
EDINET scraping修正後のverifyは targeted tests を優先し、`tests/cli/test_scrape_edinet_reports.py` を含めること。
E2Eで `scrape_edinet_reports` を動かすときは、既存watchdogと競合させないため `STOCK_DB_VAR_DIR` を分離して実行すること。
本番で Phase 2 の強制再取得verifyをするときは、`doc_id` が同一tickerに既に紐づいている銘柄だけを選び、`scrape_all_edinet_reports(..., skip_existing=False)` を直接呼ぶこと。
READMEには追加した主要CLIの実行コマンドを `uv run ...` 形式で明記すること。
