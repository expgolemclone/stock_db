# Architecture

## Overview

stock_db は日本株の財務データ・株価データを収集し、SQLite に集約するツールキット。

| ソース | 取得内容 | 取得方式 |
|---|---|---|
| EDINET | 有価証券報告書 XBRL | 検索フォームで docID を発見し、EDINET API v2 で ZIP を取得 |
| Stooq | JP 全銘柄の日次株価 | CSV ダウンロード |
| Yahoo Finance JP | 前日終値の補完 | quote page のスクレイピング |
| japan_company_handbook | 会社四季報由来の予想純利益・配当 | 外部 SQLite DB から同期 |

## Requirements

- Python 3.11 以上
- Rust toolchain stable
- Node.js（CI は 24 を使用）
- `uv`

Rust は XBRL パーサコアのビルドに、Node.js は browser service の実行に使う。

## Setup

```bash
# Rust toolchain が未導入の場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Python / Rust 依存関係
uv sync --frozen

# browser service 依存関係
npm ci --prefix services/browser

# EDINET API キー
# 環境変数のほか、repo root の .env も自動参照する
export EDINET_API_KEY=...
```

## Bootstrapping an Existing Database

GitHub Actions の最新成功 run から `stocks-db` artifact を取得すると、
既存の `var/db/stocks.db` を使い始められる。GitHub CLI の認証が必要。

```bash
gh auth login
run_id=$(gh run list --repo expgolemclone/stock_db --workflow update-stooq-prices.yml --branch main --status success --limit 1 --json databaseId --jq '.[0].databaseId')
mkdir -p var/db
gh run download "$run_id" --repo expgolemclone/stock_db --name stocks-db --dir var/db
uv run inspect-stock-db 7203 --limit 1
```

PowerShell:

```powershell
gh auth login
$runId = gh run list --repo expgolemclone/stock_db --workflow update-stooq-prices.yml --branch main --status success --limit 1 --json databaseId --jq '.[0].databaseId'
New-Item -ItemType Directory -Force -Path var/db
gh run download $runId --repo expgolemclone/stock_db --name stocks-db --dir var/db
uv run inspect-stock-db 7203 --limit 1
```

既に `var/db/stocks.db` がある場合は、必要に応じて `stocks.db-wal` /
`stocks.db-shm` と一緒に退避する。artifact が使えない場合は
`uv run scrape-stooq-prices --headless` で DB を新規作成できる。

## CLI Catalog

### EDINET

```bash
uv run scrape-edinet-reports
uv run scrape-edinet-reports-step1
uv run scrape-edinet-reports-step2
uv run scrape-edinet-reports-step2 --force
uv run scrape-edinet-historical
uv run parse-xbrl-financials
uv run parse-xbrl-bs
uv run report-edinet-progress
uv run purge-irbank-financials
```

- `scrape-edinet-reports-step2` / `scrape-edinet-reports` / `scrape-edinet-historical` は `EDINET_API_KEY` が必要
- `parse-xbrl-bs` は旧コマンド名の互換ラッパー
- `parse-xbrl-financials` は既定で既存 `edinet_xbrl` を持つ ticker を skip し、再処理時だけ `--force` を使う
- `purge-irbank-financials` は `financial_items.source LIKE 'irbank%'` を削除する

### Prices

```bash
uv run scrape-stooq-prices
uv run scrape-yahoo-finance-prices
```

- `scrape-stooq-prices` は Stooq の JP 日次 CSV を取り込み、CI では `--headless` を使う
- `scrape-yahoo-finance-prices` は fresh でない銘柄だけを走査し、Yahoo Finance JP の quote page から前日終値を補完する

### External Sync and Derived Data

```bash
uv run sync-shikiho-forecasts
uv run sync-shikiho-dividends
uv run compute-eps
```

- `sync-shikiho-forecasts` は `japan_company_handbook/data/stock_performance.db` の `stock_forecasts` から予想純利益を同期する
- `sync-shikiho-dividends` は同 DB の `stock_dividends` から DPS を同期する
- `compute-eps` は過去 EPS と予想 EPS を `financial_items(source=computed)` に書き込む

### Inspection and Utilities

```bash
uv run inspect-stock-db 7203 --limit 1
uv run generate-validation-site-list
```

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
    jpx_market_holidays.toml # JPX 市場休日の年別定義
  rust/                    # Rust crate — XBRL パーサコア (PyO3 + maturin)
    Cargo.toml
    src/
      lib.rs               # PyO3 モジュール定義・公開関数
      db.rs                # XBRL 解析結果の ticker 単位マージ・SQLite 置換書き込み
      types.rs             # ConceptKey, ContextInfo, LoadedXbrlArtifact 等の構造体
      xml_util.rs          # namespace 解析・XBRL 値パース・タグ分類ユーティリティ
      artifact.rs          # XBRL artifact ローダー (Rayon 並列パース → 逐次 bucket 格納)
      inventory.rs         # inventories 集計 (direct → calculation → presentation → component sum)
      financials.rs        # BS/PL/CF/dividend/forecast 正規化・候補マッチング
      share_classes.rs     # 種類株式別発行済株式数の正規化
  services/
    browser/               # Node.js ブラウザサービス（Puppeteer）
      server.js            # Express API サーバー
      browser_pool.js      # ブラウザプール管理
  src/stock_db/            # Python パッケージ
    _edinet_xbrl.cpython-*.so  # maturin ビルド時に生成される Rust 拡張
    paths.py               # プロジェクトルート・設定ファイルパスの定義
    market_calendar.py     # JPX 営業日判定
    browser_client/
      client.py            # BrowserServiceClient（ブラウザサービスの Python クライアント）
    cli/
      scrape_edinet_reports.py # EDINET 有報取得 CLI（step1: browser search, step2: EDINET API ZIP取得）
      scrape_edinet_reports_step1.py # EDINET step1（ブラウザ検索）のみ
      scrape_edinet_reports_step2.py # EDINET step2（XBRL取得）のみ
      scrape_edinet_historical.py # 過去分の有報XBRL一括取得（EDINET API v2 書類一覧→ZIP取得）
      parse_xbrl_bs.py     # 旧コマンド名の互換ラッパー
      parse_xbrl_financials.py # Rust DB 更新 API を呼ぶ薄い CLI
      inspect_stock_db.py  # ticker 単位の read-only DB 確認
      purge_irbank_financials.py # irbank 系 source の一括削除
      scrape_edinet_watchdog.py # メモリ監視付き watchdog ラッパー
      report_edinet_progress.py # Phase 1/2 の raw / actionable 進捗を集計
      scrape_stooq_prices.py # Stooq 日次価格取り込み CLI
      sync_shikiho_forecasts.py # 会社四季報予想の純利益同期 CLI
      sync_shikiho_dividends.py # 会社四季報の配当同期 CLI
      compute_eps.py          # EPS 計算 CLI（過去: net_income / shares_outstanding、将来: 予想純利益 / 現在発行済株式数）
      scrape_yahoo_finance_prices.py # Yahoo Finance JP 価格補完 CLI
      generate_validation_site_list.py
    sources/edinet/
      api_client.py        # EDINET API v2 クライアント（書類ZIP取得・展開）
      search_scraper.py    # EDINET 検索フォーム経由で docID 発見（スレッドセーフ）
      document_list.py     # EDINET API v2 書類一覧取得（日付範囲で有報docIDを収集）
      xbrl_bs_parser.py    # 薄い Python ラッパー — Rust parse_inventories / validation を呼び出し + 例外変換
      xbrl_financials_parser.py # 薄い Python ラッパー — Rust parse_xbrl_artifact から financials を返す
      xbrl_share_classes_parser.py # 薄い Python ラッパー — Rust parse_xbrl_artifact から share_classes を返す
    sources/stooq/
      downloader.py        # Stooq 日次ファイルダウンロード（CAPTCHA 対応）
      parser.py            # Stooq CSV パーサー（4桁・5桁・英字付きティッカー対応）
      updater.py           # Stooq ダウンロード・ingest・commit をまとめた public API
      captcha_solver.py    # Stooq CAPTCHA OCR ソルバー
      exceptions.py        # Stooq 固有例外
    sources/yahoo_finance_jp/
      parser.py            # Yahoo Finance JP HTML パーサー（前日終値・出来高抽出）
      scraper.py           # 接尾辞自動検出・価格スクレイプ・DB保存
    storage/
      connection.py        # SQLite 接続（WAL, FK有効）
      schema.py            # テーブル定義・マイグレーション
      financials.py        # financial_items テーブル CRUD
      share_classes.py      # share_classes テーブル CRUD
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
EDINET API v2 documents.json?date=YYYY-MM-DD ──書類一覧→ secCode ticker照合──→ scrape-edinet-historical ──ZIP保存/展開──→ var/raw/edinet/xbrl/
var/raw/edinet/xbrl/ ──parse-xbrl-financials──→ financial_items (source=edinet_xbrl)
                                                └→ share_classes (source=edinet_xbrl)
Stooq 日次CSV ──ダウンロード──→ parser ──→ prices
                                         └→ source_refresh_log
Yahoo Finance JP ──スクレイピング──→ parser ──→ prices
                                                  stocks.yf_suffix
japan_company_handbook stock_performance.db ──sync-shikiho-forecasts──→ financial_items (source=shikiho)
japan_company_handbook stock_performance.db ──sync-shikiho-dividends──→ financial_items (source=shikiho)
financial_items (net_income + shares_outstanding) + stocks.shares_outstanding ──compute-eps──→ financial_items (source=computed)
```

## Key Components

### Rust Parser Core (PyO3)

EDINET XBRL パーサと XBRL 由来 DB 更新を Rust (PyO3 + maturin) で実装。`rust/` クレートを `stock_db._edinet_xbrl` として Python に公開。

- `parse_inventories(path)` — 棚卸資産総額の抽出。direct total → calculation linkbase → presentation linkbase → component sum の優先度チェーンで集計
- `parse_xbrl_artifact(path)` — 1 artifact を 1 回だけロードし、BS / PL / CF / dividend / forecast / shares outstanding / preferred-share flag と種類株明細を同時に返す。CF 候補は JPPFS 系と IFRS 系の alias を正規化する。主表の無次元 fact を優先し、無い場合のみ `NonConsolidatedMember` 単独の非連結総額 context を fallback として使う
- `parse_xbrl_financials_to_db(db_path, ...)` — `sec_reports` から対象 artifact を取得し、ticker 単位の Rayon 並列解析、後続 filing 優先のマージ、`financial_items` / `share_classes` の置換書き込みまで Rust 側で実行する
- `is_valid_xbrl_text` / `is_valid_xbrl_path` — 保存済み EDINET artifact の検証補助。Python 側の scrape skip 判定から呼ぶ
- `parse_financials(path)` / `parse_share_classes(path)` — 既存テスト・外部呼び出し用の互換 API。内部は `parse_xbrl_artifact` と同じ 1 回ロード経路を使う
- `InventoriesTagMismatchError` — PyO3 例外として定義し、Python 側 `xbrl_bs_parser` から再エクスポート
- quick-xml によるイベント駆動 XML パース。iXBRL inline fact と XBRL instance fact の両方をサポート
- Rayon (`par_iter`) で ticker 単位と文書単位の XML 抽出を並列化。context/unit/nsmap/fact の抽出は文書ごとに独立しており、並列実行後に結果を逐次マージする。`store_fact_buckets` の HashMap 書き込みと SQLite 書き込みは競合を避けるため逐次のまま
- `XBRL/PublicDoc/*.xbrl` が存在する場合はそれを canonical fact document として優先使用し、iXBRL HTML の重複パースを回避する。PublicDoc .xbrl が無い場合は全 fact document を収集して `is_valid_xbrl_text` でフィルタする従来の fallback を使う
- PyO3 の重い関数は `py.allow_threads()` で GIL を解放する。並列化は Python thread ではなく Rayon に統一する

Python 側は `xbrl_bs_parser.py` / `xbrl_financials_parser.py` / `xbrl_share_classes_parser.py` と CLI 引数処理だけを薄く保持し、XBRL 解析・正規化・DB 書き込みは Rust 側に集約する。

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
6テーブル: `stocks`, `financial_items`, `share_classes`, `prices`, `source_refresh_log`, `sec_reports`

| テーブル | 役割 |
|---|---|
| `stocks` | 銘柄マスタ、EDINET / Yahoo の補助メタデータ |
| `financial_items` | 財務値。EDINET 正本、四季報同期値、計算値を保持 |
| `share_classes` | 種類株式別の発行済株式数 |
| `prices` | 日次終値。Yahoo 由来行では出来高も保持し得る |
| `source_refresh_log` | Stooq 更新チェック時刻 |
| `sec_reports` | 有報メタデータと展開済み XBRL artifact のルート |

### EDINET Sources

| コンポーネント | 役割 |
|---|---|
| `api_client` | EDINET API v2 `documents/{docID}?type=1` で提出本文書・監査報告書・taxonomy・linkbase を含む ZIP を取得し、`{doc_id}.zip` と展開ディレクトリを原子的に保存 |
| `search_scraper` | EDINET 検索フォーム経由で有報 docID を発見（スレッドセーフ）。HTML entity デコード・企業名フォールバック付き |
| `document_list` | EDINET API v2 `documents.json?date=YYYY-MM-DD` で日付範囲の書類一覧を取得し、有価証券報告書 (`ordinanceCode=010, formCode=030000`) をフィルタ。`secCode` 先頭4桁で ticker 照合 |
| `xbrl_bs_parser` | Rust 拡張 (`_edinet_xbrl.parse_inventories` / validation API) の薄い Python ラッパー |
| `xbrl_financials_parser` | Rust 拡張 (`_edinet_xbrl.parse_xbrl_artifact`) の financials 互換ラッパー |
| `xbrl_share_classes_parser` | Rust 拡張 (`_edinet_xbrl.parse_xbrl_artifact`) の share classes 互換ラッパー |

`search_scraper` は書類種別ラジオを明示的に「指定する」に切り替えたうえで `有価証券報告書` チェックを付与し、提出者名検索で大量保有報告書などに埋もれて annual report を取り逃がさないようにしている。

`scrape_edinet_reports` の Phase 1 は `config/edinet_phase1.toml` の alias、`stocks.name`、Yahoo Finance JP の quote title から提出者名称候補を作る。候補は HTML entity デコード、Unicode NFKC、空白正規化、`(株)` / `（株）` → `株式会社` 変換をかけ、`株式会社` の前置 / 後置バリアントも生成したうえで順に試す。Yahoo で補完した名称・suffix は `stocks` に保存して次回以降の探索に再利用する。

英字付き ticker (`275A` など) は EDINET の提出者証券コード欄が 4 桁数値専用のため、証券コード検索を行わず提出者名称候補だけで探索する。

`config/edinet_phase1.toml` の `excluded_tickers` は、ETF など自己名義の `securities_report_url` を保持しない銘柄を Phase 1 の対象外として扱う。`report_edinet_progress` は raw の `phase1_pending` とは別に `phase1_excluded` と `phase1_pending_actionable` を出力し、actionable 未解決と除外済みを別 TSV に書き出す。

`scrape_edinet_reports` の Phase 2 は `EDINET_API_KEY` を必須とし、`sec_reports.xbrl_path` には展開済みアーティファクトのルートディレクトリを保存する。skip 判定では `xbrl_path` だけでなく、ZIP+展開済み artifact が有効かを再検証し、旧 `.xhtml` file や invalid 保存物は再取得対象に戻す。

raw 同期 (`sync_edinet_raw_to_db`) は `xbrl/{ticker}/{doc_id}/` と sibling の `{doc_id}.zip` だけを正規入力として回収する。top-level の legacy `*.xhtml` は同期対象に含めない。

`scrape_edinet_historical` は EDINET API v2 の書類一覧API (`documents.json`) を既定で `--to-date` から10年前までイテレートし、各日の有価証券報告書を抽出する。discovery は日付単位で `var/raw/edinet/discovery/*.json` に checkpoint 保存し、同じ日付範囲の再実行では `completed_dates` をスキップして再開する。API 失敗日は `failed_dates` に残し、成功した日付は `failed_dates` から消す。`secCode`（5桁）の先頭4桁で DB 内の数値 ticker と照合し、一致する docID を収集後、既存の `download_xbrl_package` で ZIP を取得する。download/sync フェーズは同じ checkpoint の `processing.statuses` に docID 別の `skipped` / `synced_existing` / `downloaded` / `error` を保存し、DB 書き込みと checkpoint 保存を `--commit-interval` 件ごとに flush する。`sec_reports.fiscal_year` には API レスポンスの `periodEnd` から抽出した `FYXXXX` を格納する。英字付き ticker は `secCode` がないため対象外。`--skip-existing` がデフォルトで有効であり、`sec_reports` に既存の docID はスキップする。raw に有効な `xbrl/{ticker}/{doc_id}.zip` と `xbrl/{ticker}/{doc_id}/` が既にある場合は再ダウンロードせず、`sec_reports` に同期する。

`purge_irbank_financials` は `financial_items` の `source LIKE 'irbank%'` を一括削除し、WAL checkpoint と `VACUUM` を実行して artifact に `irbank` 系 source を残さない。

`parse_xbrl_financials` は Python CLI から Rust の `parse_xbrl_financials_to_db` を呼ぶ。Rust 側は `sec_reports` 上の同一 ticker の全 artifact を fiscal year 昇順で畳み込み、後続 filing の同一 `(period, statement, item_name)` と `(period, class_name)` を優先する。その後 `financial_items` の同一 ticker に対する `irbank` / `irbank_bs` / `irbank_forecast` / `xbrl_bs` / 旧 `edinet_xbrl` を削除し、再構築した rows を `source=edinet_xbrl` で一括登録する。種類株明細は `share_classes` の同一 ticker / source を置換する。`financial_items` の主キーは `(ticker, period, statement, item_name)` で `source` を含まないため、source 単位ではなく ticker 単位で置換する。`--skip-existing` はデフォルトで有効であり、既存 `edinet_xbrl` を持つ ticker を skip する。再パースが必要な場合だけ `--force` を使う。ticker 単位の解析並列化は Rayon が担当し、SQLite 書き込みと commit は Rust 側で ticker ごとに逐次実行する。`InventoriesTagMismatchError` は ticker 単位の error として集計し、予期しない例外は伝播させる。

棚卸資産 parser は未知の inventory-like tag をエラーにするが、金融業の `StocksAssetsInvestmentSecuritiesBNK` / `StocksAssetsINS` は有価証券・保険資産の株式であり棚卸資産ではないため明示除外する。

### Stooq Sources

| コンポーネント | 役割 |
|---|---|
| `downloader` | Stooq 日次JP全銘柄CSVのダウンロード（CAPTCHA 解決・リトライ付き） |
| `parser` | CSV から `.JP` 銘柄を抽出し prices テーブルに upsert。4桁・5桁・英字付き（3桁+A等）ティッカーに対応。保存値は `prices.close` のみで `prices.volume` は保持しない |
| `updater` | `download_latest_daily_file` と `ingest_daily_prices` を組み合わせた直接更新 API と、`uv run scrape-stooq-prices` を `stock_db` cwd で実行する command API |
| `captcha_solver` | Stooq CAPTCHA 画像の OCR 解決 |

`scrape_stooq_prices` CLI は browser service を起動して Stooq 日次CSVを取得する。通常実行に加えて `--headless` / `--no-headless` で browser 起動モードを上書きでき、CI では `uv run scrape-stooq-prices --headless` を使う。

他プロジェクトからは `run_stooq_price_update_command()` を使う。これは `stock_db` プロジェクトルートを cwd にして `uv run scrape-stooq-prices` を実行し、CLI と同じ browser service 起動経路を使う。対象DBや出力先を変える場合は `db_path` / `output_dir` を渡す。

`storage.prices.is_stooq_price_update_required` は、`prices` の最新価格日付と JPX 営業日カレンダーを比較し、最新価格日の翌日から当日までに JPX 営業日が1日以上ある場合だけ更新を要求する。Stooq 更新チェックに成功した時刻は `source_refresh_log` に保存し、Stooq 側のデータがまだ進んでいない場合でも24時間以内の再実行は抑止する。JPX の市場休日は `config/jpx_market_holidays.toml` に年別で明示し、判定対象年が未定義の場合は例外で止める。土日だけの判定には fallback しない。

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
`config/jpx_market_holidays.toml` に JPX 公式 Market Holidays の年別休日定義を保持する。

`magic_numbers.toml` 上の待機間隔は、通常 EDINET が 1 秒、Yahoo Finance JP が 1 秒、historical EDINET が 2 秒。
EDINET API を使う `scrape-edinet-reports` / `scrape-edinet-reports-step2` / `scrape-edinet-historical` では、環境変数または `.env` の `EDINET_API_KEY` が必要。

## Operations

- GitHub Actions `update-stooq-prices.yml` が毎日 **16:00 JST**（cron `0 7 * * *`）に `uv run scrape-stooq-prices --headless` を実行する
- workflow は `Update Stooq prices` の前に `fonts-dejavu-extra` / `fonts-freefont-ttf` / `fonts-liberation` を install し、Stooq CAPTCHA OCR に必要な DejaVu / FreeSans / Liberation Sans 系フォントを runner に揃える
- workflow は upload 前に `uv run purge-irbank-financials` を実行し、artifact の `financial_items` から `irbank` 系 source を除去する
- `stocks.db` は GitHub Actions Artifacts（名前 `stocks-db`）で永続化する。workflow 開始時に前回の artifact をダウンロードし、スクレイプ後にアップロードする
- 手動検証時は `gh workflow run update-stooq-prices.yml --repo expgolemclone/stock_db --ref main` で `workflow_dispatch` を発火し、各ステップが success になることを確認する
- EDINET Phase 1 の手動検証は `uv run scrape-edinet-reports-step1` 実行後に `uv run report-edinet-progress` を実行し、`phase1_pending_actionable` が 0 か、残件が alias / exclusion 追加対象として妥当かを確認する
