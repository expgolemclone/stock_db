# stock_db

日本株の財務データ・株価データを自動収集し、SQLite に集約するツールキット。

## データソース

| ソース | 取得内容 | 取得方式 |
|---|---|---|
| **[EDINET](https://disclosure2.edinet-fsa.go.jp/)** | 有価証券報告書 (XBRL) | EDINET API v2 |
| **[Stooq](https://stooq.com/db/)** | 日次株価 (JP 全銘柄) | CSV ダウンロード (CAPTCHA 対応) |
| **[Yahoo Finance JP](https://finance.yahoo.co.jp/)** | 前日終値の補完取得 | スクレイピング |

## 前提

- Python 3.11+
- Rust toolchain (stable) — XBRL パーサコアが Rust (PyO3 + maturin) で実装されている
- Node.js 24+ (ブラウザサービス用)

## セットアップ

```bash
# Rust toolchain のインストール（未導入の場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Python・Rust 依存関係のビルド・インストール
uv sync --frozen

# ブラウザサービスの依存関係（EDINET step1 / Yahoo / Stooq に使用）
npm ci --prefix services/browser

# EDINET API キー（step2 / combined run / historical の raw 取得に使用）
# .env の EDINET_API_KEY も自動的に参照される
export EDINET_API_KEY=...
```

## 既存 stocks.db の取得

GitHub Actions の最新成功 run から `stocks-db` artifact を取得すると、既存の
`var/db/stocks.db` を使い始められる。Artifact の取得には GitHub CLI の認証が必要。

```bash
gh auth login
run_id=$(gh run list --repo expgolemclone/stock_db --workflow update-stooq-prices.yml --branch main --status success --limit 1 --json databaseId --jq '.[0].databaseId')
mkdir -p var/db
gh run download "$run_id" --repo expgolemclone/stock_db --name stocks-db --dir var/db
uv run inspect-stock-db 7203 --limit 1
```

PowerShell では以下のように実行する。

```powershell
gh auth login
$runId = gh run list --repo expgolemclone/stock_db --workflow update-stooq-prices.yml --branch main --status success --limit 1 --json databaseId --jq '.[0].databaseId'
New-Item -ItemType Directory -Force -Path var/db
gh run download $runId --repo expgolemclone/stock_db --name stocks-db --dir var/db
uv run inspect-stock-db 7203 --limit 1
```

既に `var/db/stocks.db` がある場合は、必要に応じて `stocks.db-wal` / `stocks.db-shm`
と一緒に事前退避する。Artifact が期限切れまたは存在しない場合は、
`uv run scrape-stooq-prices --headless` で新規作成する。

## CLI コマンド

### EDINET

```bash
# 有報取得 (step1: ブラウザ検索 → step2: EDINET API ZIP 取得)
uv run scrape-edinet-reports

# step1 / step2 のみ個別実行
uv run scrape-edinet-reports-step1
uv run scrape-edinet-reports-step2

# 既存書類の API 再取得
uv run scrape-edinet-reports-step2 --force

# 過去10年分の有報XBRL取得（既存 raw ZIP+展開済み artifact は再利用）
uv run scrape-edinet-historical

# XBRL から財務データを抽出
uv run parse-xbrl-financials

# irbank 由来の財務データを完全削除
uv run purge-irbank-financials

# 旧コマンド名（互換ラッパー）
uv run parse-xbrl-bs

# 進捗レポート
uv run report-edinet-progress
```

- `scrape-edinet-reports-step2` / `scrape-edinet-reports` / `scrape-edinet-historical` は `EDINET_API_KEY` が必要（環境変数または `.env`）
- raw EDINET 書類は `var/raw/edinet/xbrl/{ticker}/{doc_id}.zip` と `var/raw/edinet/xbrl/{ticker}/{doc_id}/` に保存される
- `scrape-edinet-historical` は discovery の途中結果を `var/raw/edinet/discovery/*.json` に保存し、同じ日付範囲の再実行では完了済み日付を再走査しない
- `purge-irbank-financials` は `financial_items` から `source LIKE 'irbank%'` を完全削除する
- `parse-xbrl-financials` は既定で既存 `edinet_xbrl` を持つ ticker を skip し、`--force` 指定時だけ再パースする
- `parse-xbrl-financials` は `financial_items` を `source=edinet_xbrl` で再構築し、同一 ticker の `irbank` / `irbank_bs` / `irbank_forecast` / `xbrl_bs` を置き換える

### 株価

```bash
# Stooq 日次株価の取り込み
uv run scrape-stooq-prices

# Yahoo Finance JP 価格補完
uv run scrape-yahoo-finance-prices
```

- `scrape-yahoo-finance-prices` は fresh でない銘柄を走査し、解決できた Yahoo Finance JP の quote page から前日終値を補完する

### その他

```bash
# DB 内容の確認
uv run inspect-stock-db

# バリデーション用サイトリスト生成
uv run generate-validation-site-list
```

## 自動実行

GitHub Actions が毎日 16:00 JST に Stooq 日次株価の更新を実行する。

- `stocks.db` は Artifacts (`stocks-db`) で永続化
- CAPTCHA OCR 用に DejaVu / FreeSans / Liberation Sans フォントをインストール
- artifact へ再保存する前に `uv run purge-irbank-financials` を実行し、`irbank` 系 source を残さない
- 手動実行: `gh workflow run update-stooq-prices.yml --repo expgolemclone/stock_db --ref main`

## 設定

- `config/cli_defaults.toml` — CLI のデフォルト引数
- `config/magic_numbers.toml` — タイムアウト・リトライ回数・インターバル等の定数
- `config/edinet_phase1.toml` — EDINET Phase 1 の社名 alias と対象外 ticker の定義

## EDINET Phase 1 運用

- `scrape-edinet-reports-step1` は `securities_report_url` 未設定銘柄だけを対象にする
- 英字付き ticker (`275A` など) は EDINET の証券コード欄が 4 桁数値専用のため、証券コード検索を行わず提出者名称候補だけで探索する
- 提出者名称候補は `config/edinet_phase1.toml` の alias、`stocks.name`、Yahoo Finance JP の quote title から構成し、HTML entity / 全角英数 / `(株)` を正規化して順に試す
- `config/edinet_phase1.toml` の `excluded_tickers` は、ETF など自己名義の有報 URL を保持しない銘柄を Phase 1 の対象外として扱う
- `uv run report-edinet-progress` は raw の `phase1_pending` に加えて `phase1_excluded` と `phase1_pending_actionable` を出力し、actionable な未解決一覧と除外一覧を別 TSV に書き出す

## データベース

SQLite (`var/db/stocks.db`)。WAL モード・外部キー制約有効。

| テーブル | 内容 |
|---|---|
| `stocks` | 銘柄マスタ |
| `financial_items` | 財務データ (`source=edinet_xbrl` を正とする) |
| `prices` | 日次株価 (Stooq / Yahoo Finance) |
| `sec_reports` | 有価証券報告書メタデータ (`xbrl_path` は展開済み XBRL アーティファクトのルート) |
