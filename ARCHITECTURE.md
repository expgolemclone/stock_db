# Architecture

## 目次

- [1. 概要](#1-概要)
- [2. Repo 間依存関係](#2-repo-間依存関係)
- [3. 実行環境とセットアップ](#3-実行環境とセットアップ)
- [4. 主要コンポーネント](#4-主要コンポーネント)
- [5. データフロー](#5-データフロー)
- [6. SQLite スキーマと保存方針](#6-sqlite-スキーマと保存方針)
- [7. 公開インターフェース](#7-公開インターフェース)
- [8. 運用](#8-運用)
- [9. 開発・検証](#9-開発検証)

## 1. 概要

`stock_db` は日本株の銘柄マスタ、株価、有価証券報告書 XBRL、四季報由来の予想値を SQLite に集約するデータ基盤 repo である。

主な責務は次の通り。

- EDINET から有価証券報告書の docID を発見し、EDINET API v2 で XBRL ZIP を取得・展開する。
- Rust core で EDINET XBRL を解析し、BS / PL / CF / forecast / dividend / shares outstanding / 種類株式明細を正規化する。
- Stooq の JP 日次 CSV と Yahoo Finance JP の quote page から日次価格を保存する。
- `japan_company_handbook` の SQLite から四季報予想純利益と配当を同期する。
- 下流 repo が Python API、Rust crate、または `var/db/stocks.db` を通じて同じデータを読めるようにする。

重要な制約として、スクレイピングは直列実行を基本とし、外部サイトへの連続アクセスには 2 秒ディレイを置く。XBRL パーサは IFRS と J-GAAP の両方を正規化対象にする。

## 2. Repo 間依存関係

`stock_db` は外部データ取得と正規化を担当し、下流 repo は `stock_db` の DB / API / artifact を読む。Mermaid 図では実線を通常の依存、点線を CI artifact や検証時の参照として表す。

```mermaid
flowchart LR
    subgraph upstream["Upstream repos"]
        handbook["japan_company_handbook\nstock_performance.db"]
    end

    subgraph stockdb["stock_db"]
        pyapi["Python package\nstock_db.*"]
        rustcore["Rust crate\nstock-db-xbrl"]
        sqlite["SQLite\nvar/db/stocks.db"]
        cli["CLI\nuv run ..."]
        artifact["GitHub Actions artifact\nstocks-db"]
    end

    subgraph downstream["Downstream repos"]
        formula["formula_screening"]
        legends["invest_like_legends"]
        webui["stock_web_ui"]
    end

    handbook -->|sync-shikiho-forecasts / sync-shikiho-dividends| cli
    cli --> sqlite
    pyapi --> sqlite
    rustcore --> sqlite
    sqlite --> artifact

    pyapi -->|storage API / price refresh API| formula
    rustcore -->|screening::load_screening_stocks| formula
    sqlite -->|screening input| formula

    pyapi -->|price refresh API / stock names| legends
    sqlite -->|financial_items / stocks / prices| legends
    artifact -.->|CI bootstrap| legends

    sqlite -->|STOCKS_DB_PATH| webui
    pyapi -.->|downstream UI checks| webui
```

依存の読み方:

- `japan_company_handbook` は `data/stock_performance.db` を生成し、`stock_db` の `sync-shikiho-*` CLI がこれを読む。
- `formula_screening` は `stock_db` を editable Python dependency として使い、Rust 側では `stock-db-xbrl` crate の `screening` module を直接読む。
- `invest_like_legends` は `stock_db` の DB と価格更新 API を使い、CI では `stocks-db` artifact を bootstrap に使う。
- `stock_web_ui` は downstream UI 検証やローカル表示で `stock_db/var/db/stocks.db` を参照する。

関連 repo の bookmark は Rust 移行後の現行実装を `main`、旧 Python 系を `py` として扱う。`push-*` は `jj git push --change` 由来の一時 bookmark であり、長期運用名にはしない。

## 3. 実行環境とセットアップ

必要な実行環境:

- Python 3.11 以上
- Rust stable toolchain
- Node.js。CI では Node.js 24 を使う。
- `uv`

初回セットアップ:

```bash
uv sync --frozen
npm ci --prefix services/browser
```

EDINET API を使う処理では `EDINET_API_KEY` が必要である。環境変数のほか、repo root の `.env` も参照する。

```bash
export EDINET_API_KEY=...
```

既存 DB は GitHub Actions の `stocks-db` artifact から復元できる。定期価格更新 workflow も前回成功 run の artifact を復元してから更新する。復元後は `uv run inspect-stock-db 7203 --limit 1` で読み取り確認する。

## 4. 主要コンポーネント

### 4.1 Python package

`src/stock_db/` は CLI、外部 source client、storage layer、browser service client を持つ。

- `paths.py`: repo root、`var/`、設定ファイル、既定 DB パスを定義する。
- `storage/`: SQLite 接続、schema 初期化、各テーブルの CRUD を提供する。
- `sources/edinet/`: EDINET API、検索フォーム scraping、XBRL parser wrapper を持つ。
- `sources/stooq/`: Stooq 日次 CSV の download / parse / ingest / command wrapper を持つ。
- `sources/yahoo_finance_jp/`: Yahoo Finance JP HTML の parse / scrape / DB 保存を持つ。
- `browser_client/`: Node.js browser service を subprocess で起動し HTTP API を呼ぶ。
- `cli/`: repo の運用入口を `pyproject.toml` の project scripts として公開する。

### 4.2 Rust core

`rust/` は `stock-db-xbrl` crate であり、PyO3 extension `stock_db._edinet_xbrl` と Rust library の両方を提供する。

- XBRL artifact をロードし、通常 `.xbrl` は 1 回の XML 走査で context / unit / namespace / fact を抽出する。
- EDINET XBRL の BS / PL / CF / forecast / dividend / shares outstanding / 種類株式を正規化する。
- ticker 単位の XBRL parse と DB 置換書き込みを Rust 側で実行する。
- Stooq 日次 CSV parse を Rust 側で実行し、Python 側は DB upsert に集中する。
- `screening` module は下流 Rust 実装向けに `stocks.db` から screening 用データを読む。

重い処理は PyO3 で GIL を解放し、ticker や文書単位の並列化は Rayon に寄せる。SQLite 書き込みは競合を避けるため逐次実行する。

### 4.3 Browser service

`services/browser/` は Node.js / Express / Puppeteer 系の browser service である。Python から起動され、Stooq や EDINET 検索など通常の HTTP client だけでは扱いにくいページ取得を担当する。

主な endpoint:

- `/fetch`: URL にアクセスして HTML を返す。
- `/download`: browser 経由でファイルを download する。
- `/evaluate`: ページ上で JavaScript を評価する。
- `/stooq/*`: Stooq 日次 CSV 取得用の専用 endpoint。
- `/shutdown`: service を終了する。

### 4.4 設定ファイル

`config/` は実行時の定数と CLI 既定値を持つ。

- `cli_defaults.toml`: CLI の既定引数。
- `magic_numbers.toml`: timeout、retry、wait interval など。
- `edinet_phase1.toml`: EDINET Phase 1 の alias / excluded ticker。
- `jpx_market_holidays.toml`: JPX 市場休日。未定義年は例外で止め、土日判定だけには fallback しない。

## 5. データフロー

### 5.1 EDINET

```mermaid
flowchart LR
    stocks["stocks"] --> phase1["scrape-edinet-reports-step1\nEDINET search"]
    phase1 -->|docID / PDF URL| stocks
    phase1 --> phase2["scrape-edinet-reports-step2\nEDINET API v2"]
    api["documents/{docID}?type=1"] --> phase2
    phase2 --> raw["var/raw/edinet/xbrl/{ticker}/{doc_id}.zip\n展開 directory"]
    raw --> sec_reports["sec_reports.xbrl_path"]
    sec_reports --> parse["parse-xbrl-financials\nRust parse + DB replace"]
    parse --> financial_items["financial_items\nsource=edinet_xbrl"]
    parse --> share_classes["share_classes\nsource=edinet_xbrl"]
```

Phase 1 は検索フォームから有価証券報告書 docID を見つける。提出者名は `stocks.name`、`config/edinet_phase1.toml` の alias、Yahoo Finance JP の quote title を正規化して候補化する。ETF など自己名義の有報 URL を持たない対象は `excluded_tickers` で除外する。

Phase 2 は EDINET API v2 で ZIP を取得し、ZIP 原本と展開済み artifact を保存する。`sec_reports.xbrl_path` には展開済み artifact root を保存し、parse 時は `sec_reports` から対象 artifact を読む。

`scrape-edinet-historical` は EDINET API v2 の書類一覧 API から過去提出分を探索し、`secCode` の先頭 4 桁で DB 内 ticker と照合する。discovery と processing は checkpoint で再開可能にする。

### 5.2 Prices

```mermaid
flowchart LR
    stooq["Stooq daily JP CSV"] --> browser["browser service"]
    browser --> stooq_cli["scrape-stooq-prices"]
    stooq_cli --> prices["prices"]
    stooq_cli --> refresh["source_refresh_log"]

    yahoo["Yahoo Finance JP quote page"] --> yf_cli["scrape-yahoo-finance-prices"]
    yf_cli --> prices
    yf_cli --> stocks_suffix["stocks.yf_suffix"]
```

Stooq は JP 全銘柄の日次 CSV を取り込み、`prices.close` を upsert する。価格更新が必要かどうかは前営業日の JPX 終値が全 DB 銘柄に揃っているかで判定し、Stooq 側の成功した更新チェック時刻は `source_refresh_log` に保存する。`stock_db` 外の repo から Python の価格読み取り API または Rust screening API が呼ばれた場合は、この鮮度確認を API 側で行い、古ければ `refresh-prices --if-needed` 経由で自動更新する。

Yahoo Finance JP は Stooq だけでは埋まらない stale 銘柄の前日終値を補完する。接尾辞 `.T`、`.N`、`.S`、`.F` などは自動検出し、`stocks.yf_suffix` に保存して次回以降の探索を省く。Yahoo 補完は 1 銘柄ずつ実行し、個別銘柄の未取得・古い quote・接尾辞未解決は `PriceRefreshResult` の `yahoo_errors` / `unresolved_tickers` に集約する。個別銘柄の失敗では価格更新全体を停止せず、最後まで試行してから結果を返す。

### 5.3 Shikiho and derived data

```mermaid
flowchart LR
    handbook["japan_company_handbook/data/stock_performance.db"] --> forecasts["sync-shikiho-forecasts"]
    handbook --> dividends["sync-shikiho-dividends"]
    forecasts --> fi["financial_items\nsource=shikiho"]
    dividends --> fi
    fi --> eps["compute-eps"]
    stocks["stocks.shares_outstanding"] --> eps
    eps --> computed["financial_items\nsource=computed"]
```

四季報予想は upstream に残る全履歴から最新2期を選び、古い方を `forecast.net_income_current`、新しい方を `forecast.net_income_next` として保存する。配当は `dividend.dps` として保存する。同期時は upstream から消えた `shikiho` 行も削除してから再生成する。`compute-eps` は過去 EPS と予想 EPS を `source=computed` で生成する。

## 6. SQLite スキーマと保存方針

主要テーブル:

| テーブル | 役割 |
|---|---|
| `stocks` | ticker、会社名、EDINET / Yahoo 補助メタデータ、発行済株式数 |
| `sec_reports` | 有報 docID、fiscal year、XBRL artifact path |
| `financial_items` | 財務値。EDINET、四季報、計算値を `statement / item / source` 単位で保持 |
| `share_classes` | 種類株式別の発行済株式数 |
| `prices` | 日次価格。Yahoo 由来行では volume を保持し得る |
| `source_refresh_log` | Stooq 更新チェック時刻と価格更新試行時刻 |

保存方針:

- SQLite 接続は WAL と foreign key を有効にする。
- `financial_items` の正本は EDINET XBRL の `source=edinet_xbrl` とする。
- 四季報由来の予想純利益と配当は `source=shikiho` とする。
- EPS など派生値は `source=computed` とする。
- `financial_items` の主キーには `source` を含め、同一 item の source 別共存を許す。
- `parse-xbrl-financials` は ticker 単位で旧 EDINET / IRBank 系 source を置換し、同一 ticker の最新 parse 結果に揃える。
- raw EDINET artifact は `var/raw/edinet/xbrl/{ticker}/{doc_id}.zip` と sibling の展開 directory を正規入力とする。

## 7. 公開インターフェース

### 7.1 CLI

EDINET:

```bash
uv run scrape-edinet-reports
uv run scrape-edinet-reports-step1
uv run scrape-edinet-reports-step2
uv run scrape-edinet-historical
uv run parse-xbrl-financials
uv run parse-xbrl-bs
uv run report-edinet-progress
uv run purge-irbank-financials
```

Prices:

```bash
uv run refresh-prices
uv run scrape-stooq-prices
uv run scrape-yahoo-finance-prices
```

External sync and derived data:

```bash
uv run sync-shikiho-forecasts
uv run sync-shikiho-dividends
uv run compute-eps
```

Inspection:

```bash
uv run inspect-stock-db 7203 --limit 1
uv run generate-validation-site-list
```

### 7.2 Python API

下流 repo が使う代表的な API:

- `stock_db.paths.STOCKS_DB_PATH`
- `stock_db.storage.connection.get_connection`
- `stock_db.storage.schema.init_db`
- `stock_db.storage.financials.*`
- `stock_db.storage.prices.get_previous_jpx_business_day`
- `stock_db.storage.prices.get_stale_price_tickers`
- `stock_db.storage.prices.get_latest_price*`
- `stock_db.storage.stocks.get_validation_targets`
- `stock_db.sources.price_refresh.ensure_prices_fresh_for_api`
- `stock_db.sources.price_refresh.run_price_refresh_command`
- `stock_db.sources.stooq.run_stooq_price_update_command`

`stock_db` 外の repo から価格読み取り API を呼ぶ場合、API は前営業日終値が全 DB 銘柄に揃っているかを確認し、必要なら `run_price_refresh_command(if_needed=True)` で `stock_db` repo root から `uv run refresh-prices --if-needed` を実行する。更新は Stooq を先に試し、まだ stale な銘柄を Yahoo Finance JP で直列・2秒ベースのディレイで補完する。個別銘柄の取得失敗や補完後に残った stale 銘柄は結果 summary に出すが、価格更新コマンド自体は最後まで完走する。完走後は価格更新試行時刻を `source_refresh_log` に記録し、同日内の API 呼び出しで同じ大量スクレイピングを繰り返さない。DB 接続失敗や subprocess 起動失敗などの基盤エラーは例外にする。明示的に Stooq 更新だけを実行する場合は `run_stooq_price_update_command()` を使う。

`get_latest_price_with_shares()` は最新終値と発行済株式数に加えて、その終値の `price_date` も返す。下流 UI は `price_date` と前営業日 metadata を比較し、古い株価・未取得株価を通常銘柄より目立ちにくく表示できる。

### 7.3 Rust API

`stock-db-xbrl` は PyO3 extension と Rust crate として使われる。

PyO3 module `stock_db._edinet_xbrl` の代表 API:

- `parse_xbrl_artifact(path)`
- `parse_financials(path)`
- `parse_share_classes(path)`
- `parse_inventories(path)`
- `parse_xbrl_financials_to_db(db_path, ticker=None, from_ticker=None, skip_existing=True, emit_progress=False)`
- `parse_stooq_daily_file(path)`
- `is_valid_xbrl_text(content)`
- `is_valid_xbrl_path(path)`

Rust crate 側では `screening::load_screening_stocks(db_path, tickers, fcf_periods, pl_periods)` を下流の Rust screening 実装が使う。この入口も `stock_db` 外の cwd から呼ばれた場合は、DB 読み取り前に `refresh-prices --if-needed` で汎用価格更新を確認する。返却する `ScreeningStock` には株価基準日の `price_date` を含める。

## 8. 運用

GitHub Actions の `update-stooq-prices.yml` は毎日 16:00 JST に実行され、Stooq と Yahoo Finance JP 補完で価格を更新する。

運用の流れ:

1. Python / Rust / Node.js 依存を準備する。
2. Stooq CAPTCHA OCR 用 font を runner に入れる。
3. 前回の `stocks-db` artifact を `var/db` に download する。
4. `uv run refresh-prices --headless` で日次価格を更新する。Stooq を先に取り込み、残った stale 銘柄を Yahoo Finance JP で補完する。Stooq 更新だけを確認したい場合は `uv run scrape-stooq-prices --headless` を使う。
5. `uv run purge-irbank-financials` で artifact から IRBank 系 source を除去する。
6. `var/db/stocks.db` を `stocks-db` artifact として upload する。

手動で DB を確認する場合:

```bash
uv run inspect-stock-db 7203 --limit 1
```

EDINET の進捗確認は `uv run report-edinet-progress` を使う。Phase 1 の actionable pending と excluded ticker は別々に見る。

## 9. 開発・検証

通常の検証:

```bash
uv run pytest
cargo test
npm test --prefix services/browser
```

ドキュメント変更後の最低限の確認:

```bash
rg -n "^## [0-9]+\\." ARCHITECTURE.md
rg -n "```mermaid|japan_company_handbook|formula_screening|invest_like_legends|stock_web_ui" ARCHITECTURE.md
uv run inspect-stock-db 7203 --limit 1
```

注意点:

- ブランチは追加しない。
- 作業コピーに既存の未確定変更がある場合、それを戻さず、対象変更だけを重ねる。
- スクレイピング系の実行では直列処理と 2 秒ディレイを守る。
- XBRL 解析の変更では IFRS と J-GAAP の両方を壊していないことを確認する。
- XBRL の canonical financial item 候補タグは `rust/src/financials.rs` の unit test に `main` 由来の静的スナップショットを持つ。実行時に `main` を読まず、現在実装が最低条件として包含していることを検証する。
- `rust/src/screening.rs` の unit test は、source ごとの forecast / dividend 統合と履歴期間の組み立てを直接固定する。
