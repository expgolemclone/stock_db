1. IFRS, J-GAAP, US-GAAPに対応させること
2. taxonomy関連を行うときは,
   - 10年分のXBRLを正しくparseしたいため, 古いタグにも対応する必要がある.
   - `https://axiora.dev/en/blog/edinet-for-developers`を読んでから実装すること.
3. 後方互換性は維持不要. 下流repoへの配慮は不要. 設計の美しさを重要視しろ.
4. 段階的にpureなRustに移行しろ.
5. 下流repo向けの公開境界は Rust package `edinet-xbrl` の library crate `edinet_xbrl` と `edinet-xbrl downstream-*` JSON CLI とする. `stock_db.api` は公開境界として復活させないこと.
6. 待機時間が長い実行コマンドが多い. 1hおきに確認しろ. 頻繁な確認は禁止.
