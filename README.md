## rules

1. scrapingはbrowser serviceを使うこと. 並列数は3, 各リクエスト間に適切なディレイを入れること.
2. fall backは問題が発覚しづらくなるから禁止.
   どうしても実装すべきだと思う場合はuserの許可を取ること.
