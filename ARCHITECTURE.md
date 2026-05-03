# Architecture

## Overview

stock_db は日本株の財務データを収集・保存するツールキット。
IR BANK からのスクレイピングと JSON ダウンロード、EDINET API からの有価証券報告書取得の3経路でデータを取得し、SQLite に集約する。

## Directory Structure

```
stock_db/
  .gitignore              # whitelist方式（* で全除外 → ! で個別許可）
  ARCHITECTURE.md
  CLAUDE.md
  README.md
  .github/
    workflows/
      update-stooq-prices.yml # 毎日16:00 JST の Stooq 価格更新 workflow
  config/
    cli_defaults.toml      # CLI のデフォルト引数
    magic_numbers.toml     # タイムアウト・インターバル等の定数
  services/
    browser/               # Node.js ブラウザサービス（Puppeteer）
      server.js            # Express API サーバー（/fetch, /download, /shutdown）
      browser_pool.js      # ブラウザプール管理
  src/stock_db/            # Python パッケージ
    paths.py               # プロジェクトルート・設定ファイルパスの定義
    browser_client/
      client.py            # BrowserServiceClient（ブラウザサービスの Python クライアント）
    cli/
      scrape_irbank_bs.py  # IR BANK B/S スクレイピング CLI
      scrape_edinet_reports.py # EDINET 有報取得 CLI（browser service直列利用, invalid XBRL再取得）
      scrape_edinet_reports_step1.py # EDINET step1（書類一覧取得）のみ
      scrape_edinet_reports_step2.py # EDINET step2（XBRL取得）のみ
      parse_xbrl_bs.py     # EDINET XBRL から inventories のみを保存
      scrape_edinet_watchdog.py # メモリ監視付き watchdog ラッパー
      scrape_stooq_prices.py # Stooq 日次価格取り込み CLI
      scrape_yahoo_finance_prices.py # Yahoo Finance JP 価格スクレイプ CLI
      fetch_irbank_files.py # IR BANK JSON ダウンロード CLI
      purge_irbank_bs.py   # IR BANK B/S データ削除 CLI
      generate_validation_site_list.py
    sources/edinet/
      api_client.py        # EDINET API v2 クライアント（書類一覧・XBRL取得URL生成）
      search_scraper.py    # EDINET 検索フォーム経由で docID 発見（スレッドセーフ）
      xbrl_bs_parser.py    # EDINET iXBRL から inventories を抽出
    sources/stooq/
      downloader.py        # Stooq 日次ファイルダウンロード（CAPTCHA 対応）
      parser.py            # Stooq CSV パーサー（4桁・5桁・英字付きティッカー対応）
      captcha_solver.py    # Stooq CAPTCHA OCR ソルバー
      exceptions.py        # Stooq 固有例外
    sources/yahoo_finance_jp/
      parser.py            # Yahoo Finance JP HTML パーサー（前日終値・出来高抽出）
      scraper.py           # 接尾辞自動検出・価格スクレイプ・DB保存
    sources/irbank/
      bs_parser.py         # B/S ページ HTML パーサー
      bs_scraper.py        # B/S ページスクレイパー（取得 → パース → 保存）
      downloader.py        # IR BANK JSON ファイルダウンローダー
    storage/
      connection.py        # SQLite 接続（WAL, FK有効）
      schema.py            # テーブル定義・マイグレーション
      financials.py        # financial_items テーブル CRUD
      sec_reports.py       # sec_reports テーブル CRUD
      stocks.py            # stocks テーブル CRUD
      prices.py            # prices テーブル CRUD
    tools/
      validation_site_list.py
  var/
    db/stocks.db           # SQLite データベース（GitHub Actions Artifactsで永続化）
    raw/edinet/
      xbrl/{ticker}/*.xhtml        # ダウンロード済み XBRL
    raw/irbank/            # ダウンロード済み JSON ファイル
```

## Data Flow

```
IR BANK /bs ページ ──スクレイピング──→ bs_parser ──→ financial_items
IR BANK JSON API ──ダウンロード──→ downloader ──→ financial_items
EDINET API v2 ──XBRL取得/検証──→ var/raw/edinet/xbrl/
var/raw/edinet/xbrl/ ──parse-xbrl-bs──→ financial_items (source=xbrl_bs, inventoriesのみ)
Stooq 日次CSV ──ダウンロード──→ parser ──→ prices
Yahoo Finance JP ──スクレイピング──→ parser ──→ prices + yf_suffix
                                                  stocks
                                                  prices
```

## Key Components

### Browser Service (Node.js)

Puppeteer ベースのヘッドレスブラウザプール。Express で HTTP API を提供し、Python 側から `BrowserServiceClient` 経由で利用する。

- `/fetch` — URL にアクセスして HTML を取得
- `/download` — ファイルダウンロードを実行
- `/shutdown` — サービス終了

Windows 対応済み（PTY の代わりに PIPE を使用）。

### BrowserServiceClient (Python)

`subprocess.Popen` で Node.js サーバーを起動し、stdout を監視してポート番号を検出。`requests` で API を呼び出す。コンテキストマネージャー対応。

### IR BANK Sources

| コンポーネント | 役割 |
|---|---|
| `bs_scraper` | B/S ページの取得・リトライ・DB保存のオーケストレーション |
| `bs_parser` | HTML からの財務データ抽出（概要表・詳細表の2形式対応） |
| `downloader` | IR BANK の JSON ファイル一括ダウンロード |

### Storage Layer

SQLite を使用。WAL モード・外部キー制約有効。
4テーブル: `stocks`, `financial_items`, `prices`, `sec_reports`

### EDINET Sources

| コンポーネント | 役割 |
|---|---|
| `api_client` | EDINET API v2 で書類一覧取得・XBRL取得URL生成。XBRL は文書ビューアの GeneXus AJAX PostBack で ix facts を含む本文セクションだけを反復取得し、header-only HTML は reject |
| `search_scraper` | EDINET 検索フォーム経由で有報 docID を発見（スレッドセーフ）。HTML entity デコード・企業名フォールバック付き |
| `xbrl_bs_parser` | EDINET iXBRL の exact consolidated instant facts から棚卸資産を抽出。direct total を優先し、必要時のみ component を合算 |

`scrape_edinet_reports` の Phase 2 は `sec_reports.xbrl_path` があるだけでは完了扱いせず、保存済み `.xhtml` が parseable かを再判定する。header-only / invalid XBRL は再取得対象に戻す。

### Stooq Sources

| コンポーネント | 役割 |
|---|---|
| `downloader` | Stooq 日次JP全銘柄CSVのダウンロード（CAPTCHA 解決・リトライ付き） |
| `parser` | CSV から `.JP` 銘柄を抽出し prices テーブルに upsert。4桁・5桁・英字付き（3桁+A等）ティッカーに対応。保存値は `prices.close` のみで `prices.volume` は保持しない |
| `captcha_solver` | Stooq CAPTCHA 画像の OCR 解決 |

`scrape_stooq_prices` CLI は browser service を起動して Stooq 日次CSVを取得する。通常実行に加えて `--headless` / `--no-headless` で browser 起動モードを上書きでき、CI では `uv run scrape-stooq-prices --headless` を使う。

### Yahoo Finance JP Sources

Stooq 未カバーの非東証銘柄（名証・札証・福証）の前日終値を Yahoo Finance Japan からスクレイプ。
接尾辞（.T/.N/.S/.F）を自動検出し、`stocks.yf_suffix` に記録して2回目以降は一発取得。

| コンポーネント | 役割 |
|---|---|
| `parser` | HTML から前日終値・日付・出来高を抽出。存在しないページと、ページはあるが quote data が空のケースを判別 |
| `scraper` | 接尾辞自動検出・価格取得・DB 保存のオーケストレーション。quote data が空でも有効な銘柄ページなら `stocks.yf_suffix` を保存し、次回以降の再探索を省略 |

## Configuration

`config/magic_numbers.toml` にタイムアウト値・リトライ回数・インターバル等を集約。
`config/cli_defaults.toml` に CLI のデフォルト引数を定義。

## Operations

- GitHub Actions `update-stooq-prices.yml` が毎日 **16:00 JST**（cron `0 7 * * *`）に `uv run scrape-stooq-prices --headless` を実行する
- workflow は `Update Stooq prices` の前に `fonts-dejavu-extra` / `fonts-freefont-ttf` / `fonts-liberation` を install し、Stooq CAPTCHA OCR に必要な DejaVu / FreeSans / Liberation Sans 系フォントを runner に揃える
- `stocks.db` は GitHub Actions Artifacts（名前 `stocks-db`）で永続化する。workflow 開始時に前回の artifact をダウンロードし、スクレイプ後にアップロードする
- 手動検証時は `gh workflow run update-stooq-prices.yml --repo expgolemclone/stock_db --ref main` で `workflow_dispatch` を発火し、各ステップが success になることを確認する
