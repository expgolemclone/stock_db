# stocks.db 監査レポート (2026-04-29)

## 対象

- 監査対象DB: `stock_db/var/db/stocks.db`
- 比較対象: `../formula_screening`
- 備考: `../formula_screening/data/stocks.db` は 0 byte で未使用。`../formula_screening/src/formula_screening/db/schema.py` は `stock_db.paths.STOCKS_DB_PATH` を参照している。

## 実施結果 (2026-04-29 18:11 JST)

- backup 作成:
  - `var/backup/stocks.db.before_trash_purge_20260429_181039`
- 実施済み cleanup:
  - `financial_items.source='yfinance'` 148 行を削除
  - `ticker='8462'` の `stocks` 1 行と `financial_items` 448 行を削除
  - `market_cap` テーブルを削除
  - `prices.shares_outstanding` 列を削除
  - `stocks.address_source_urls` 列を削除
  - `var/reference/all_stocks.csv` から `8462` を削除し、`254A` を追加
  - `var/backup/stocks.db.before_trash_purge_20260429_181039` を削除
- cleanup 後の主な状態:
  - `stocks`: 4478 行
  - `financial_items`: 2,181,045 行
  - `sec_reports`: 3178 行
  - `financial_items.statement='_status'`: 3 行のまま維持
- 残件:
  - `var/raw/edinet` と `sec_reports` の不整合は未対応

以下は cleanup 前の監査内容。

## 要約

- もっとも危険なのは `financial_items` に混ざっている `source='yfinance'` の 148 行。6 銘柄の最新PL/CF/配当データに直接混ざっており、`formula_screening` の計算結果へ影響する。
- `prices.shares_outstanding` は現行スキーマ外の死に列で、全件 `NULL`。
- `stocks.address_source_urls` は保存されているが読取経路がない。

## テーブル別サマリ

| table             |      rows | 用途判定                           |
| ----------------- | --------: | ---------------------------------- |
| `financial_items` | 2,181,641 | 使用中。ただし `yfinance` 混在あり |
| `prices`          |      3608 | 使用中。ただし死に列あり           |
| `market_cap`      |      3183 | 未使用候補                         |

補助件数:

- `stocks.shares_outstanding IS NOT NULL`: 3838
- `stocks.securities_report_url IS NOT NULL`: 3495
- `stocks.address_source_urls IS NOT NULL`: 628
- `stocks.edinet_code IS NOT NULL`: 0
- `stocks.sector <> ''`: 0
- `stocks.market <> ''`: 0
- `prices.shares_outstanding IS NOT NULL`: 0

## 削除候補

### P0: `financial_items` の `source='yfinance'`

- 件数: 148 行
- 銘柄数: 6 (`4063, 6501, 6861, 6920, 7203, 8035`)
- 内訳:
  - `dividend.dps`: 100 行
  - `cf.free_cf`: 24 行
  - `pl.cost_of_revenue`: 24 行
- 問題:
  - `financial_items` の主キーは `(ticker, period, statement, item_name)` で `source` を含まない。
  - そのため、`source='irbank'` と同じ EAV 空間に `yfinance` の値が混ざる。
  - `../formula_screening/src/formula_screening/db/repository.py` の `get_financial_dict()` は `source` を絞らずに最新期間を読む。
  - `../formula_screening/src/formula_screening/metrics.py` は `pl.cost_of_revenue`、`cf.free_cf`、`dividend.dps` をそのまま利用する。
- 現に最新期間へ混入している行:

```text
4063 2025-03 cf.free_cf / pl.cost_of_revenue
6501 2025-03 cf.free_cf / pl.cost_of_revenue
6861 2025-03 cf.free_cf / pl.cost_of_revenue
6920 2025-06 cf.free_cf / pl.cost_of_revenue
7203 2025-03 cf.free_cf / pl.cost_of_revenue
8035 2025-03 cf.free_cf / pl.cost_of_revenue
```

- 影響:
  - `gross_margin`
  - `free_cf`
  - `free_cf_ratio`
  - `dividend_yield`
  - `fcf_yield_avg` を使う戦略
- 判定: 現行repo用途に対して混ざってはいけないデータ。最優先で隔離または削除対象。

### P1: `prices.shares_outstanding` 列

- 件数: 全 3608 行で `NULL`
- 問題:
  - 現行コードは `shares_outstanding` を `stocks` テーブルに保持しており、`src/stock_db/storage/prices.py` も `../formula_screening/src/formula_screening/db/repository.py` も `prices` からは読まない。
  - `src/stock_db/storage/schema.py` の現行 `CREATE TABLE prices` 定義にも列がない。
- 判定: 過去スキーマの残骸。死に列。

### P2: `market_cap` テーブル

- 件数: 3183 行
- source: `kabutan` のみ
- `fetched_at` 範囲: `2026-04-02` から `2026-04-13`
- 問題:
  - `formula_screening` の実運用は `price * shares_outstanding` で時価総額を計算しており、`market_cap` テーブルを参照しない。
  - `stock_db` 側でも実運用の読取は `src/stock_db/storage/market_caps.py` と inspect 用CLI/テストに留まる。
- 判定: 現状未使用。削除

### P3: `stocks.address_source_urls` 列

- 件数: 628 行
- 値の特徴: 628 件すべて一意、内容は `["https://irbank.net/<ticker>/ir"]` のJSON配列
- 問題:
  - 保存は `src/stock_db/storage/stocks.py` にあるが、読取経路が現行コードにない。
- 判定: 現状未接続のメタデータ。即時危険ではないが未使用候補。

### P3: `stocks.sector` / `stocks.market` 列

- 非空件数: どちらも 0
- 問題:
  - スキーマ上は存在するが、DB上では全件空。
  - `formula_screening` でも現行スクリーニング処理は参照していない。
- 判定: データとしては未使用。列自体を残すかは別として、少なくとも現DBには有効情報が入っていない。

## 根拠SQL

### テーブル件数

```sql
SELECT 'stocks' AS table_name, COUNT(*) FROM stocks
UNION ALL
SELECT 'financial_items', COUNT(*) FROM financial_items
UNION ALL
SELECT 'prices', COUNT(*) FROM prices
UNION ALL
SELECT 'market_cap', COUNT(*) FROM market_cap
UNION ALL
SELECT 'sec_reports', COUNT(*) FROM sec_reports;
```

### `yfinance` 混在件数

```sql
SELECT item_name, COUNT(*)
FROM financial_items
WHERE source = 'yfinance'
GROUP BY item_name
ORDER BY COUNT(*) DESC;
```

### 最新期間への `yfinance` 混在確認

```sql
WITH latest_pl AS (
    SELECT ticker, MAX(period) AS period
    FROM financial_items
    WHERE statement = 'pl'
    GROUP BY ticker
)
SELECT fi.ticker, fi.period, fi.statement, fi.item_name, fi.source, fi.value
FROM financial_items fi
JOIN latest_pl lp
  ON lp.ticker = fi.ticker
 AND lp.period = fi.period
WHERE fi.source = 'yfinance'
ORDER BY fi.ticker, fi.statement, fi.item_name;
```

### `prices.shares_outstanding` の死に列確認

```sql
PRAGMA table_info(prices);

SELECT COUNT(*)
FROM prices
WHERE shares_outstanding IS NOT NULL;
```

## 結論

- まず対処すべきは `financial_items` の `yfinance` 148 行。
- 次に `prices.shares_outstanding` はスキーマ整理対象。
- `market_cap` と `address_source_urls` は、使う意思がないなら削除候補、残すなら用途を明文化すべき。
- `sec_reports`、`securities_report_url`、`_status` は不要情報ではない。
