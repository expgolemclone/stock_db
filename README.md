## rules

1. scrapingはbrowser serviceを使うこと. 並列数は1, 各リクエスト間に適切なディレイを入れること.
2. fall backは問題が発覚しづらくなるから禁止.
   どうしても実装すべきだと思う場合はuserの許可を取ること.

## commands

EDINET報告書scrapeを単一銘柄で実行する:

```bash
uv run scrape-edinet-reports --ticker 7203
```

全銘柄を対象にする場合は `--ticker` を外す。

EDINET報告書 scrape の step1 のみを単一銘柄で実行する:

```bash
uv run scrape-edinet-reports-step1 --ticker 7203
```

EDINET報告書 scrape の step2 のみを単一銘柄で実行する:

```bash
uv run scrape-edinet-reports-step2 --ticker 7203
```

step2 は `securities_report_url` 未設定の ticker をスキップする。全銘柄を対象にする場合は `--ticker` を外す。

stooqの日次価格を取り込む:

```bash
uv run scrape-stooq-prices
```

Yahoo Finance JPから非東証銘柄の価格をスクレイプする:

```bash
uv run scrape-yahoo-finance-prices --ticker 3442
```

全銘柄を対象にする場合は `--ticker` を外す。接尾辞（.T/.N/.S/.F）は自動検出されDBに記録される。
