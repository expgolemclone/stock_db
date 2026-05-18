# Code Review Report

レビュー日: 2026-05-18

## Findings

### [P1] Stooq の定期 workflow は前回 artifact を復元できず、毎回空 DB から作り直す

- 該当箇所: `.github/workflows/update-stooq-prices.yml:53-70`
- `actions/download-artifact@v4` は `github-token` / `run-id` を渡さない場合、同一 workflow run 内の artifact だけを対象にする。現在の workflow は upload より前に download を実行しており、同じ run にはまだ `stocks-db` が存在しない。
- さらに download step は `continue-on-error: true` なので、復元失敗を無視して `scrape-stooq-prices` に進む。その結果、定期実行は既存の `stocks.db` を継承せず、新規 DB に当日の価格だけを入れて再 upload する構成になっている。
- 影響: artifact を bootstrap に使う下流 repo から見ると、過去価格、銘柄 master、EDINET 財務、四季報同期結果が失われた DB を受け取る可能性がある。

### [P1] 新しい XBRL が追加されても、既存 ticker は既定では再解析されない

- 該当箇所: `src/stock_db/cli/parse_xbrl_financials.py:39-45`, `rust/src/db.rs:131-145`
- CLI の既定値は `skip_existing=True` で、Rust 側は `financial_items` に既存 `edinet_xbrl` が 1 行でもある ticker を丸ごと除外している。
- 再現: `1234` に既存 `edinet_xbrl` 行と複数 `sec_reports` を入れた DB で `parse_xbrl_financials_to_db(..., skip_existing=True)` を呼ぶと、`{'ok': 0, 'errors': 0, 'skipped': 1, ...}` になり、追加 report は処理されない。
- 影響: 2 回目以降の有報取得後、運用者が毎回 `--force` を付けない限り、既存銘柄の最新年度が DB に反映されない。`ARCHITECTURE.md` の「同一 ticker の最新 parse 結果に揃える」という説明ともずれている。

### [P1] `financial_items` は source ごとの共存を表現できず、後勝ちで他 source を上書きする

- 該当箇所: `src/stock_db/storage/schema.py:22-31`, `src/stock_db/storage/financials.py:17-47`
- 主キーが `(ticker, period, statement, item_name)` で、`source` が含まれていない。upsert も同じ key で衝突解決するため、同一 item を別 source で保存すると前の行を静かに上書きする。
- 再現: 同じ `2025-03 / forecast / net_income` に `edinet_xbrl=100`、続けて `shikiho=200` を書くと、残るのは `shikiho=200` の 1 行だけ。
- 影響: source 別 purge や `source='shikiho'` を前提にした読み取りがある一方で、実データは source ごとに独立して残らない。EDINET と派生/外部 source の値が同じ item 名を使った瞬間に、出所付きの履歴が破壊される。

### [P2] 四季報 forecast 同期は最新 2 期ではなく最古 2 期を選ぶ

- 該当箇所: `src/stock_db/cli/sync_shikiho_forecasts.py:32-84`
- 上流の `japan_company_handbook` は `stock_forecasts` を `(stock_code, forecast_type, period)` 主キーで UPSERT しており、過去 period を削除しない設計である。
- それに対して同期側は `ORDER BY stock_code, period ASC` の先頭 2 行を `net_income_current` / `net_income_next` として採用している。
- 再現: upstream に `25.3`, `26.3`, `27.3` を入れると、同期結果は `25.3=current`, `26.3=next` になる。
- 影響: 年度が進んで upstream に過去 period が残り始めると、forecast が自動的に古い期へ逆戻りする。

### [P2] 四季報同期は upstream から消えた ticker の古い行を削除しない

- 該当箇所: `src/stock_db/cli/sync_shikiho_forecasts.py:43-86`, `src/stock_db/cli/sync_shikiho_dividends.py:44-74`
- どちらの同期も、今回 upstream から取得できた ticker ごとにだけ delete/replace している。upstream から ticker が消えた場合や forecast が `NULL` になった場合、その ticker の既存 `shikiho` 行は一切触られない。
- 再現: `financial_items` に `9999` の `shikiho` forecast/dividend を入れ、空の upstream DB を同期すると、両方の行がそのまま残る。
- 影響: 取り下げられた予想や削除された銘柄の値が、最新データとして残り続ける。

### [P3] スクレイピングの 2 秒ディレイ制約を実装が守っていない

- 該当箇所: `RULES.md:1`, `ARCHITECTURE.md:27`, `config/magic_numbers.toml:7-14`, `src/stock_db/cli/scrape_edinet_reports.py:341-347`
- repo ルールと設計書は「直列かつ 2 秒ディレイ」を要求しているが、通常 EDINET は `1.0` 秒、Yahoo Finance JP も `1.0` 秒で設定されている。
- さらに EDINET phase1 の Yahoo fallback は `discover_company_name(..., interval=0.0)` を使っており、suffix 探索を無待機で連続実行する。
- 影響: サイト側 rate limit / block の確率を自ら上げるうえ、文書化された運用制約をコードが満たしていない。

### [P3] `ARCHITECTURE.md` に書かれた browser service の検証コマンドは失敗する

- 該当箇所: `ARCHITECTURE.md:321-327`, `services/browser/package.json:8-10`
- 設計書は `npm test --prefix services/browser` を通常検証として案内しているが、`package.json` には `test` script がない。
- 再現: 実行すると `npm error Missing script: "test"` で終了する。
- 影響: ドキュメント通りに検証した利用者は、browser service のテストを実行できない。

## Verification

- `uv run pytest` -> 254 passed
- `cargo test` -> 14 passed
- `node --test services/browser/browser_pool.test.js` -> 2 passed
- `npm test --prefix services/browser` -> failed (`Missing script: "test"`)

## Notes

- 今回はレビューのみを行い、修正は加えていない。
