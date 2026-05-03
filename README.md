# stock_db

日本株の財務データ・株価データを自動収集し、SQLite に集約するツールキット。

## データソース

| ソース | 取得内容 | 取得方式 |
|---|---|---|
| **EDINET** | 有価証券報告書 (XBRL) | EDINET API v2 |
| **Stooq** | 日次株価 (JP 全銘柄) | CSV ダウンロード (CAPTCHA 対応) |
| **Yahoo Finance JP** | 非東証銘柄の前日終値 | スクレイピング |

## セットアップ

```bash
# Python 依存関係のインストール
uv sync --frozen

# ブラウザサービスの依存関係（スクレイピングに使用）
npm ci --prefix services/browser
```

## CLI コマンド

### EDINET

```bash
# 有報取得 (step1: 書類一覧取得 → step2: XBRL 取得)
uv run scrape-edinet-reports

# step1 / step2 のみ個別実行
uv run scrape-edinet-reports-step1
uv run scrape-edinet-reports-step2

# XBRL から棚卸資産を抽出
uv run parse-xbrl-bs

# 進捗レポート
uv run report-edinet-progress
```

### 株価

```bash
# Stooq 日次株価の取り込み
uv run scrape-stooq-prices

# Yahoo Finance JP 株価スクレイプ
uv run scrape-yahoo-finance-prices
```

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
- 手動実行: `gh workflow run update-stooq-prices.yml --repo expgolemclone/stock_db --ref main`

## 設定

- `config/cli_defaults.toml` — CLI のデフォルト引数
- `config/magic_numbers.toml` — タイムアウト・リトライ回数・インターバル等の定数

## データベース

SQLite (`var/db/stocks.db`)。WAL モード・外部キー制約有効。

| テーブル | 内容 |
|---|---|
| `stocks` | 銘柄マスタ |
| `financial_items` | 財務データ (EDINET XBRL) |
| `prices` | 日次株価 (Stooq / Yahoo Finance) |
| `sec_reports` | 有価証券報告書メタデータ |
