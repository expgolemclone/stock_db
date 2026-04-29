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
  config/
    cli_defaults.toml      # CLI のデフォルト引数
    magic_numbers.toml     # タイムアウト・インターバル等の定数
  services/
    browser/               # Node.js ブラウザサービス（Puppeteer）
      server.js            # Express API サーバー（/fetch, /download, /shutdown）
      browser_pool.js      # ブラウザプール管理
  src/stock_db/            # Python パッケージ
    paths.py               # プロジェクトルート・設定ファイルパスの定義
    proxy_pool.py          # プロキシプール（ローテーション・障害検知）
    browser_client/
      client.py            # BrowserServiceClient（ブラウザサービスの Python クライアント）
    cli/
      scrape_irbank_bs.py  # IR BANK B/S スクレイピング CLI
      scrape_edinet_reports.py # EDINET 有報取得 CLI（3並列・ThreadPoolExecutor）
      scrape_edinet_watchdog.py # メモリ監視付き watchdog ラッパー
      fetch_irbank_files.py # IR BANK JSON ダウンロード CLI
      purge_irbank_bs.py   # IR BANK B/S データ削除 CLI
      generate_validation_site_list.py
    sources/edinet/
      api_client.py        # EDINET API v2 クライアント（書類一覧・PDF取得）
      search_scraper.py    # EDINET 検索フォーム経由で docID 発見（スレッドセーフ）
      pdf_extractor.py     # PDF → Markdown テキスト抽出
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
    db/stocks.db           # SQLite データベース（Git LFS）
    raw/edinet/
      markdown/{ticker}/latest.md  # 抽出済み Markdown
      pdf/{ticker}/*.pdf           # ダウンロード済み PDF
      xbrl/{ticker}/*.xhtml        # ダウンロード済み XBRL
    raw/irbank/            # ダウンロード済み JSON ファイル
```

## Data Flow

```
IR BANK /bs ページ ──スクレイピング──→ bs_parser ──→ financial_items
IR BANK JSON API ──ダウンロード──→ downloader ──→ financial_items
EDINET API v2 ──PDF取得──→ pdf_extractor ──→ sec_reports + var/raw/edinet/markdown/
EDINET API v2 ──XBRL取得──→ var/raw/edinet/xbrl/
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
| `api_client` | EDINET API v2 で書類一覧取得・PDFダウンロード |
| `pdf_extractor` | pypdf で PDF からテキスト抽出し Markdown に変換 |
| `search_scraper` | EDINET 検索フォーム経由で有報 docID を発見（スレッドセーフ） |

### ProxyPool

プロキシのローテーション・障害検知・除外管理。HTTP/SOCKS5 対応。

## Configuration

`config/magic_numbers.toml` にタイムアウト値・リトライ回数・インターバル等を集約。
`config/cli_defaults.toml` に CLI のデフォルト引数を定義。
