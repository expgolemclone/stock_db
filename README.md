# stock_db

`rust/src/screening.rs` は下流 Rust 実装向けの screening read API を提供する

日本株の財務データ・株価データを収集して SQLite に集約するツールキット

PL,BS,CFの全項目を読み取る仕様変更にあたり, [axiora](https://axiora.dev), [edinet-xbrl](https://github.com/axioradev/edinet-xbrl) を参考にさせて頂きました.

---

> [!NOTE]
> 仕様は[ARCHITECTURE.md](ARCHITECTURE.md)を参照
