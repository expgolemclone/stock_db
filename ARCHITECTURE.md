# Architecture

## Overview

stock_db は日本株の財務データを収集・保存するツールキット。
EDINET API からの有価証券報告書取得、Stooq / Yahoo Finance JP からの株価取得の3経路でデータを取得し、SQLite に集約する。

## Directory Structure

```
stock_db/
  .gitignore              # whitelist方式（* で全除外 → ! で個別許可）
  ARCHITECTURE.md
  README.md
  pyproject.toml
  .github/
    workflows/
      update-stooq-prices.yml # 毎日16:00 JST の Stooq 価格更新 workflow
  config/
    cli_defaults.toml      # CLI のデフォルト引数
    magic_numbers.toml     # タイムアウト・インターバル等の定数
    edinet_phase1.toml     # EDINET Phase 1 の社名 alias / 対象外 ticker
  services/
    browser/               # Node.js ブラウザサービス（Puppeteer）
      server.js            # Express API サーバー
      browser_pool.js      # ブラウザプール管理
  src/stock_db/            # Python パッケージ
    paths.py               # プロジェクトルート・設定ファイルパスの定義
    browser_client/
      client.py            # BrowserServiceClient（ブラウザサービスの Python クライアント）
    cli/
      scrape_edinet_reports.py # EDINET 有報取得 CLI（step1: browser search, step2: EDINET API ZIP取得）
      scrape_edinet_reports_step1.py # EDINET step1（書類一覧取得）のみ
      scrape_edinet_reports_step2.py # EDINET step2（XBRL取得）のみ
      parse_xbrl_bs.py     # 旧コマンド名の互換ラッパー
      parse_xbrl_financials.py # EDINET XBRL から PL / BS / CF / dividend / forecast を保存
      scrape_edinet_watchdog.py # メモリ監視付き watchdog ラッパー
      report_edinet_progress.py # Phase 1/2 の raw / actionable 進捗を集計
      scrape_stooq_prices.py # Stooq 日次価格取り込み CLI
      scrape_yahoo_finance_prices.py # Yahoo Finance JP 価格補完 CLI
      generate_validation_site_list.py
    sources/edinet/
      api_client.py        # EDINET API v2 クライアント（書類ZIP取得・展開）
      search_scraper.py    # EDINET 検索フォーム経由で docID 発見（スレッドセーフ）
      xbrl_bs_parser.py    # EDINET XBRL artifact から inventories 総額を抽出
      xbrl_financials_parser.py # EDINET XBRL artifact から canonical financial_items を抽出
    sources/stooq/
      downloader.py        # Stooq 日次ファイルダウンロード（CAPTCHA 対応）
      parser.py            # Stooq CSV パーサー（4桁・5桁・英字付きティッカー対応）
      captcha_solver.py    # Stooq CAPTCHA OCR ソルバー
      exceptions.py        # Stooq 固有例外
    sources/yahoo_finance_jp/
      parser.py            # Yahoo Finance JP HTML パーサー（前日終値・出来高抽出）
      scraper.py           # 接尾辞自動検出・価格スクレイプ・DB保存
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
      xbrl/{ticker}/{doc_id}.zip   # EDINET API 取得の ZIP 原本
      xbrl/{ticker}/{doc_id}/...   # 展開済み XBRL / taxonomy / linkbase
```

## Data Flow

```
EDINET search (browser) ──docID発見──→ stocks.securities_report_url
config/edinet_phase1.toml ──alias / exclusion──→ Phase 1 search plan
EDINET API v2 documents/{docID}?type=1 ──ZIP保存/展開──→ var/raw/edinet/xbrl/
var/raw/edinet/xbrl/ ──parse-xbrl-financials──→ financial_items (source=edinet_xbrl)
Stooq 日次CSV ──ダウンロード──→ parser ──→ prices
Yahoo Finance JP ──スクレイピング──→ parser ──→ prices
                                                  stocks.yf_suffix
```

## Key Components

### Browser Service (Node.js)

Puppeteer ベースのヘッドレスブラウザプール。Express で HTTP API を提供し、Python 側から `BrowserServiceClient` 経由で利用する。

- `/fetch` — URL にアクセスして HTML を取得
- `/download` — ファイルダウンロードを実行
- `/evaluate` — ページ上で JS を評価
- `/stooq/*` — Stooq 日次 CSV 取得用の専用 endpoint 群
- `/shutdown` — サービス終了

Windows 対応済み（PTY の代わりに PIPE を使用）。

### BrowserServiceClient (Python)

`subprocess.Popen` で Node.js サーバーを起動し、stdout を監視してポート番号を検出。`requests` で API を呼び出す。コンテキストマネージャー対応。

### Storage Layer

SQLite を使用。WAL モード・外部キー制約有効。
4テーブル: `stocks`, `financial_items`, `prices`, `sec_reports`

### EDINET Sources

| コンポーネント | 役割 |
|---|---|
| `api_client` | EDINET API v2 `documents/{docID}?type=1` で提出本文書・監査報告書・taxonomy・linkbase を含む ZIP を取得し、`{doc_id}.zip` と展開ディレクトリを原子的に保存 |
| `search_scraper` | EDINET 検索フォーム経由で有報 docID を発見（スレッドセーフ）。HTML entity デコード・企業名フォールバック付き |
| `xbrl_bs_parser` | EDINET XBRL artifact から棚卸資産総額を抽出。direct total を最優先し、次に calculation linkbase、最後に presentation linkbase を使って taxonomy-aware に集計する。legacy `.xhtml` 形式も移行期間中は互換サポート |
| `xbrl_financials_parser` | EDINET XBRL artifact から `financial_items` の canonical 名へ正規化して抽出する。BS / PL / CF / dividend は direct fact を優先し、`inventories` だけは `xbrl_bs_parser` の taxonomy-aware 集計を再利用する。`forecast` は XBRL に数値 fact がある場合のみ登録し、無ければ補完しない |

`search_scraper` は書類種別ラジオを明示的に「指定する」に切り替えたうえで `有価証券報告書` チェックを付与し、提出者名検索で大量保有報告書などに埋もれて annual report を取り逃がさないようにしている。

`scrape_edinet_reports` の Phase 1 は `config/edinet_phase1.toml` の alias、`stocks.name`、Yahoo Finance JP の quote title から提出者名称候補を作る。候補は HTML entity デコード、Unicode NFKC、空白正規化、`(株)` / `（株）` → `株式会社` 変換をかけ、`株式会社` の前置 / 後置バリアントも生成したうえで順に試す。Yahoo で補完した名称・suffix は `stocks` に保存して次回以降の探索に再利用する。

英字付き ticker (`275A` など) は EDINET の提出者証券コード欄が 4 桁数値専用のため、証券コード検索を行わず提出者名称候補だけで探索する。

`config/edinet_phase1.toml` の `excluded_tickers` は、ETF など自己名義の `securities_report_url` を保持しない銘柄を Phase 1 の対象外として扱う。`report_edinet_progress` は raw の `phase1_pending` とは別に `phase1_excluded` と `phase1_pending_actionable` を出力し、actionable 未解決と除外済みを別 TSV に書き出す。

`scrape_edinet_reports` の Phase 2 は `EDINET_API_KEY` を必須とし、`sec_reports.xbrl_path` には展開済みアーティファクトのルートディレクトリを保存する。skip 判定では `xbrl_path` だけでなく、ZIP+展開済み artifact が有効かを再検証し、legacy `.xhtml` の header-only / invalid 保存物は再取得対象に戻す。

raw 同期 (`sync_edinet_raw_to_db`) は `xbrl/{ticker}/{doc_id}/` と sibling の `{doc_id}.zip` を正規形として回収する。既存 `*.xhtml` は移行期間の互換入力としてのみ扱い、同じ `doc_id` に新旧両形式がある場合は ZIP+展開形式を優先する。

`parse_xbrl_financials` は `sec_reports` 上の同一 ticker の全 artifact を fiscal year 昇順で畳み込み、後続 filing の同一 `(period, statement, item_name)` を優先する。その後 `financial_items` の同一 ticker に対する `xbrl_bs` / 旧 `edinet_xbrl` を削除し、再構築した rows を `source=edinet_xbrl` で一括登録する。`financial_items` の主キーは `(ticker, period, statement, item_name)` で `source` を含まないため、source 単位ではなく ticker 単位で置換する。`--skip-existing` はデフォルトで有効であり、既存 `edinet_xbrl` を持つ ticker を skip する。再パースが必要な場合だけ `--force` を使う。

### Stooq Sources

| コンポーネント | 役割 |
|---|---|
| `downloader` | Stooq 日次JP全銘柄CSVのダウンロード（CAPTCHA 解決・リトライ付き） |
| `parser` | CSV から `.JP` 銘柄を抽出し prices テーブルに upsert。4桁・5桁・英字付き（3桁+A等）ティッカーに対応。保存値は `prices.close` のみで `prices.volume` は保持しない |
| `captcha_solver` | Stooq CAPTCHA 画像の OCR 解決 |

`scrape_stooq_prices` CLI は browser service を起動して Stooq 日次CSVを取得する。通常実行に加えて `--headless` / `--no-headless` で browser 起動モードを上書きでき、CI では `uv run scrape-stooq-prices --headless` を使う。

### Yahoo Finance JP Sources

価格未取得または stale な銘柄の前日終値を Yahoo Finance Japan から補完取得する。
接尾辞（.T/.N/.S/.F）を自動検出し、`stocks.yf_suffix` に記録して2回目以降は一発取得。

| コンポーネント | 役割 |
|---|---|
| `parser` | HTML から前日終値・日付・出来高を抽出。存在しないページと、ページはあるが quote data が空のケースを判別 |
| `scraper` | 接尾辞自動検出・価格取得・DB 保存のオーケストレーション。quote data が空でも有効な銘柄ページなら `stocks.yf_suffix` を保存し、次回以降の再探索を省略 |

## Configuration

`config/magic_numbers.toml` にタイムアウト値・リトライ回数・インターバル等を集約。
`config/cli_defaults.toml` に CLI のデフォルト引数を定義。

EDINET API を使う `scrape-edinet-reports` / `scrape-edinet-reports-step2` では、環境変数 `EDINET_API_KEY` が必要。

## Operations

- GitHub Actions `update-stooq-prices.yml` が毎日 **16:00 JST**（cron `0 7 * * *`）に `uv run scrape-stooq-prices --headless` を実行する
- workflow は `Update Stooq prices` の前に `fonts-dejavu-extra` / `fonts-freefont-ttf` / `fonts-liberation` を install し、Stooq CAPTCHA OCR に必要な DejaVu / FreeSans / Liberation Sans 系フォントを runner に揃える
- `stocks.db` は GitHub Actions Artifacts（名前 `stocks-db`）で永続化する。workflow 開始時に前回の artifact をダウンロードし、スクレイプ後にアップロードする
- 手動検証時は `gh workflow run update-stooq-prices.yml --repo expgolemclone/stock_db --ref main` で `workflow_dispatch` を発火し、各ステップが success になることを確認する
- EDINET Phase 1 の手動検証は `uv run scrape-edinet-reports-step1` 実行後に `uv run report-edinet-progress` を実行し、`phase1_pending_actionable` が 0 か、残件が alias / exclusion 追加対象として妥当かを確認する
