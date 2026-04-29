read README.md

## stooq

- stooqの日次価格取得はbrowser service経由で行い, CAPTCHAはローカルOCRで解くこと.
- CAPTCHA関連の失敗時は新しいCAPTCHAを取り直して最大3回までretryすること.
- stooqの日次価格取り込みでは`prices.close`のみを使い, `prices.volume`は保存しないこと.

proxy関連は完全に廃止。
EDINET scraping修正後のverifyは targeted tests を優先し、`tests/cli/test_scrape_edinet_reports.py` を含めること。
E2Eで `scrape_edinet_reports` を動かすときは、既存watchdogと競合させないため `STOCK_DB_VAR_DIR` を分離して実行すること。
本番で Phase 2 の強制再取得verifyをするときは、`doc_id` が同一tickerに既に紐づいている銘柄だけを選び、`scrape_all_edinet_reports(..., skip_existing=False)` を直接呼ぶこと。
READMEには追加した主要CLIの実行コマンドを `uv run ...` 形式で明記すること。
EDINETの`sec_reports`は `ticker + doc_id` 単位で保持すること。同じ `doc_id` を旧/新tickerの両方に持つケースを許容する。
全銘柄run完了時に `with_url_no_report=0` かつ `with_url_report_no_xbrl=0` なら Phase 2 完了とみなす。`securities_report_url IS NULL` が残る銘柄は、EDINET検索で年報URL未発見の別課題。
