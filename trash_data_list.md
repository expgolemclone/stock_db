# trash_data_list

監査日時: 2026-04-29 JST

目的:

- `stock_db` 配下にある不要データ候補を、削除実行ではなく棚卸し目的で一覧化する。

## 実施結果 (2026-04-29 18:11 JST)

- DB backup 作成:
  - `var/backup/stocks.db.before_trash_purge_20260429_181039`
- DB cleanup 実施:
  - `financial_items.source='yfinance'` 148 行を purge
  - `ticker='8462'` の `stocks` 1 行と `financial_items` 448 行を purge
  - `market_cap` テーブルを削除
  - `prices.shares_outstanding` 列を削除
  - `stocks.address_source_urls` 列を削除
- tracked file cleanup 実施:
  - `var/reference/proxies/webshare.txt` を repo から削除
- local artifact cleanup 実施:
  - `var/log/*.pid`, `var/log/*.log`, `var/tmp/**`, `var/reports/*.tsv`, `.playwright-mcp/**`
  - `.pytest_cache`, `test-results/**`, `src/tests の __pycache__`, `.cache/structural-clone-index.json`
  - `var/proxies/http_jp.txt`, `var/proxies/socks5.txt.bak`
  - `var/backup/stocks.db.before_raw_restore_20260429_1`
- cleanup 後も残すもの:
  - `.venv`, `services/browser/node_modules`
  - `var/reference/all_stocks.csv` の `8462` / `254A` 差分
  - `var/raw/edinet` と `sec_reports` の同期不整合
  - `financial_items` の `_status` 3 行

以下は cleanup 前に作成した監査メモ。上の実施結果を優先する。

監査時点の前提:

- `uv run python -m stock_db.cli.scrape_edinet_watchdog`
- `/home/exp/projects/stock_db/.venv/bin/python3 -m stock_db.cli.scrape_edinet_reports`
- `node /home/exp/projects/stock_db/services/browser/server.js`

上の 3 プロセスが稼働中だった。実行中プロセスが参照する可能性のある再生成物は `停止後に削除可` として分離した。

判定基準:

- `確定`: 再生成可能、または stale で、本体データ整合性に影響しない。
- `停止後に削除可`: 再生成可能だが、監査時点では稼働中プロセスが使っている可能性がある。
- `要確認`: 機微情報、参照マスタ、または DB 整合性に関わるので、削除前に意図確認が必要。

## 要約

| 対象                                                                                 | 区分                     | 確度           | 規模                            | 根拠                                       | 推奨アクション               |
| ------------------------------------------------------------------------------------ | ------------------------ | -------------- | ------------------------------- | ------------------------------------------ | ---------------------------- |
| `var/log/*.pid`                                                                      | stale 実行痕跡           | 確定           | 7 files                         | 記録された PID はすべて dead               | 削除                         |
| `var/tmp/**`                                                                         | debug/probe 生成物       | 確定           | 32 files, 7.3M                  | Playwright/XBRL 調査の一時出力             | 削除                         |
| `var/reports/**`                                                                     | レポートスナップショット | 確定           | 33 files, 3.0M                  | 進捗確認用 TSV の世代蓄積                  | 削除または外部保管           |
| `.playwright-mcp/**`                                                                 | MCP 実行ログ             | 確定           | 31 files, 644K                  | ローカル観測ログで本体ではない             | 削除                         |
| `.pytest_cache`, `test-results`, `__pycache__`, `.cache/structural-clone-index.json` | キャッシュ               | 確定           | 小                              | すべて再生成可能                           | 削除                         |
| `var/log/*.log`                                                                      | 過去実行ログ             | 確定           | 10 files, 16M                   | 2026-04-24 から 2026-04-28 の過去 run ログ | 不要なら削除                 |
| `var/backup/stocks.db.before_raw_restore_20260429_1`                                 | ワンオフ backup          | 確定           | 372M                            | 2026-04-29 raw restore 前の退避 DB         | rollback 不要なら削除        |
| `.venv`                                                                              | ローカル実行環境         | 停止後に削除可 | 46M                             | 現在 `scrape_edinet_reports` が利用中      | 停止後に削除し再作成可       |
| `services/browser/node_modules`                                                      | npm 依存                 | 停止後に削除可 | 53M, 4351 files                 | 現在 `server.js` が利用中                  | 停止後に削除し `npm install` |
| `var/db/stocks.db-wal`, `var/db/stocks.db-shm`                                       | SQLite 一時ファイル      | 停止後に削除可 | 2 files                         | 現在 scrape 実行中で WAL 運用中            | 停止後に削除可               |
| `var/reference/proxies/webshare.txt`                                                 | 平文認証情報             | 要確認         | 1 file, tracked                 | Git 追跡済みの認証付き proxy 一覧          | repo から除去し secrets 化   |
| `var/proxies/http_jp.txt`, `var/proxies/socks5.txt.bak`                              | 手動 proxy リスト        | 要確認         | 2 files                         | コードから固定参照なし、手動用途の可能性   | 利用有無確認後に削除         |
| `financial_items.source='yfinance'`                                                  | 仕様不明 DB 行           | 要確認         | 148 rows, 6 tickers             | 読み出し側が `source` 非限定で混入する     | 用途確認後に purge           |
| `stocks.ticker='8462'` と関連 `financial_items`                                      | stale DB 行              | 要確認         | 1 stock row, 448 financial rows | 2024-09-27 上場廃止銘柄が残存              | 参照マスタ方針確認後に purge |
| `var/reference/all_stocks.csv` の `8462` / `254A` 差分                               | stale 参照マスタ         | 要確認         | 1 stale, 1 missing              | CSV に `8462` はあるが `254A` はない       | CSV 更新                     |
| `var/raw/edinet` と `sec_reports` の不整合                                           | 整合性問題               | 要確認         | 4 tickers + 1 missing field     | raw-only / DB-only / field 欠落あり        | sync 方針決定後に整理        |

## 確定で不要

### 1. stale PID ファイル

- 対象: `var/log/*.pid`
- 件数: 7 files
- 根拠: 記録された PID `330011, 334057, 654195, 657898, 45365, 376060, 377589` はすべて dead だった。
- 推奨: 全削除でよい。

### 2. debug / probe / snapshot 生成物

- `var/tmp/**`
  - 32 files, 7.3M
  - `playwright_edinet_debug`, `pw_xbrl_probe`, `xbrl_retry_probe`, `edinet_playwright_debug.spec.js`
  - 調査用出力であり、本体データ更新経路では使われていない。
- `var/reports/**`
  - 33 files, 3.0M
  - `edinet_phase1_unresolved_*`, `edinet_phase2_no_report_*`, `edinet_phase2_no_xbrl_*`
  - 進捗確認用の世代 TSV で、DB や raw 本体ではない。
- `.playwright-mcp/**`
  - 31 files, 644K
  - `console-*.log`, `page-*.yml`
  - ローカル MCP の観測ログ。
- 推奨: まとめて削除してよい。必要なら別保管に切り出す。

### 3. キャッシュ類

- `.pytest_cache`
  - 44K
- `test-results/.last-run.json`
  - 1 file
- `src/**/__pycache__`, `tests/**/__pycache__`
  - 14 dirs, 60 files
- `.cache/structural-clone-index.json`
  - 2.0M の `.cache` 内生成物
- 推奨: すべて削除してよい。再実行時に再生成される。

### 4. 過去 run ログ

- 対象: `var/log/*.log`
- 件数: 10 files, 16M
- 時間帯: 2026-04-24 から 2026-04-28 の過去実行ログ
- 根拠: 現在稼働中の run とは別世代の記録で、本体データではない。
- 推奨: 事後調査が不要なら削除。

### 5. ワンオフ backup DB

- 対象: `var/backup/stocks.db.before_raw_restore_20260429_1`
- 規模: 372M
- 根拠: 2026-04-29 の raw restore 前退避用 DB。通常運用経路では参照されない。
- 推奨: rollback 意図がなければ削除。

## 停止後に削除可

### 1. Python / Node 実行環境

- `.venv`
  - 46M
  - 監査時点で `scrape_edinet_reports` が利用中。
- `services/browser/node_modules`
  - 53M, 4351 files
  - 監査時点で `services/browser/server.js` が利用中。
- 推奨: 実行停止後なら削除可。必要時に再作成できる。

## 要確認だが不要候補

### 1. 平文 proxy 情報

- `var/reference/proxies/webshare.txt`
  - tracked
  - 認証付き proxy 一覧
  - repo 全文検索で参照箇所なし
- `var/proxies/http_jp.txt`
  - 8 行の HTTP proxy 一覧
- `var/proxies/socks5.txt.bak`
  - SOCKS5 proxy の退避リスト
- 評価:
  - 固定参照はないが、CLI の `--proxy file:<path>` で手動利用している可能性はある。
  - `webshare.txt` は不要データというより機微情報混入。
- 推奨:
  - 利用有無を確認する。
  - `webshare.txt` は repo 追跡から外し、ローカル ignore + secrets 管理へ移す。

### 2. `yfinance` 残留行

- 対象: `financial_items.source='yfinance'`
- 件数: 148 rows, 6 tickers
- 対象 ticker: `4063, 6501, 6861, 6920, 7203, 8035`
- 内容: `cost_of_revenue`, `free_cf`, `dps`
- 根拠:
  - 本番コード側で `yfinance` の投入経路が見えない。
  - 読み出し側は `source` を絞らないため、返却値に混ざる。
- 推奨:
  - 残す仕様がなければ `source='yfinance'` を purge 対象にする。

### 3. 上場廃止済み `8462` の残留

- `stocks` に `8462|フューチャーベンチャーキャピタル` が 1 行残っている。
- `financial_items` に 448 rows 残っている。
- `sec_reports`, `market_cap`, `prices` は 0 rows。
- 外部確認:
  - 2026-04-29 時点の JPX 情報で、`8462` は 2024-09-27 上場廃止。
  - 同日確認で `254A` は現行上場。
- 評価:
  - `stocks` を現行スクレイプ対象マスタとみなすなら stale。
- 推奨:
  - 現行銘柄のみ保持の方針なら `8462` と関連行を purge。

### 4. `all_stocks.csv` の stale 差分

- `var/reference/all_stocks.csv` には `8462` がある。
- 同 CSV に `254A` はない。
- DB と CSV の比較結果:
  - DB only ticker: `254A`
  - ref only ticker: `0`
- 評価:
  - `254A` 上場後のマスタ更新が反映されていない。
- 推奨:
  - 参照 CSV を現行上場銘柄に更新。

### 5. EDINET raw / DB 同期不整合

- raw 集計:
  - `latest.md`: 3180
  - `pdf/*.pdf`: 3181
  - `xbrl/*.xhtml`: 3180
- DB 集計:
  - `sec_reports`: 3178
  - `sec_reports.file_path` あり: 3177
  - `sec_reports.xbrl_path` あり: 3177
- 確認済み差分:
  - raw only ticker: `4217, 4364, 4886`
  - DB only ticker: `6945`
  - `264A` は `sec_reports` にあるが `xbrl_path` が空
- 評価:
  - これは丸ごと削除候補ではなく、同期漏れと不整合の候補。
- 推奨:
  - `sync_edinet_raw_to_db` の前提と運用方針を見直してから個別整理。

## 優先順位

1. 先に消してよい
   `var/log/*.pid`, `var/tmp/**`, `var/reports/**`, `.playwright-mcp/**`, cache 類
2. 次に判断する
   `var/log/*.log`
3. 方針確認が必要
   `webshare.txt`, `yfinance` 残留行, `8462`, `all_stocks.csv` 差分, EDINET 同期不整合
4. 稼働停止後に整理
   `.venv`, `services/browser/node_modules`
