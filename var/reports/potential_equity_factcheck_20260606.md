# 潜在株式関連数値ファクトチェック

実施日: 2026-06-06

対象DB: `var/db/stocks.db`

## 判定

現状の潜在株式関連数値は、少なくとも大部分をそのまま分析・スクリーニングに使える状態ではない。

`potential_equity_terms.potential_common_shares` と
`preferred_share_dilution_terms.converted_common_shares` に、発行済株式/自己株式テーブルや
1株当たり情報の広いテキストブロックから誤抽出された値が多数入っている。

## 集計結果

| 項目 | 件数 |
| --- | ---: |
| `potential_equity_terms` 全行 | 114,707 |
| `potential_common_shares IS NOT NULL` | 53,866 |
| `potential_common_shares > 1e9` | 17,808 |
| `potential_common_shares > 1e12` | 13,581 |
| `potential_common_shares > 1e18` | 491 |
| `potential_common_shares < 0` | 374 |
| `preferred_share_dilution_terms` 全行 | 8,078 |
| `converted_common_shares IS NOT NULL` | 2,595 |
| `converted_common_shares > 1e9` | 267 |
| `converted_common_shares > 1e12` | 173 |
| 優先株式系の負値行 | 288 |

最新期だけに絞っても異常値は残っている。

| 対象 | 値あり銘柄数 | 現在発行済株式数超 | 10倍超 | 100倍超 |
| --- | ---: | ---: | ---: | ---: |
| `potential_equity_terms` | 2,223 | 1,479 | 1,440 | 1,399 |
| `preferred_share_dilution_terms` | 192 | 27 | 21 | 17 |

## 代表例

### 8316 三井住友FG 2025-03

DB値:

```text
potential_equity_terms
ticker=8316
period=2025-03
instrument_type=stock_option
source_kind=issued_shares_text
potential_common_shares=1.33752908426167e+37
```

公式EDINET PDF:

- docID: `S100W0S7`
- URL: https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100W0S7.pdf

同PDFの発行済株式注記では、普通株式の期首・増加・減少・期末株式数と、自己株式の増減理由が記載されている。
「ストック・オプションの権利行使による減少93,968株」は自己株式の減少理由であり、潜在普通株式数ではない。
同じページの新株予約権表は当年度末残高を百万円単位で示しており、DB値の `1.3375e37` 株を裏付ける数値はない。

原因:

`NotesRegardingIssuedSharesAndTreasurySharesTextBlock` 全体に
「ストック・オプション」という語が含まれるため、発行済株式表全体を潜在株式明細として分類し、
表中の隣接数値を株数として誤読している。

### 402A アクセルスペースHD 2025-05

DB値:

```text
preferred_share_dilution_terms
ticker=402A
period=2025-05
class_name=Ｄ種優先株式
preferred_shares=54,887
converted_common_shares=4.33900866810003e+29
conversion_ratio_to_common=200
```

公式EDINET PDF:

- docID: `S100WLO0`
- URL: https://disclosure2dl.edinet-fsa.go.jp/searchdocument/pdf/S100WLO0.pdf

同PDFでは、2024-10-04 の取得請求権行使により、D種優先株式 54,887 株を自己株式として取得し、
対価として普通株式 54,887 株を交付したと記載されている。
また、A/B/C/C2/D各種優先株式からの転換合計は 167,450 株である。
その後の「普通株式1株につき200株」の記載は株式分割であり、優先株式の転換倍率ではない。

原因:

優先株式名の後方テキストから、別イベントである普通株式の株式分割比率 `200` を
`conversion_ratio_to_common` として拾っている。さらに発行済株式表内の複数列の数値が連結的に読まれ、
`4.339e29` のような不可能な株数になっている。

### 9021 西日本旅客鉄道 2025-03

DB値:

```text
potential_equity_terms
ticker=9021
period=2025-03
instrument_type=share_based_compensation
source_kind=issued_shares_text
potential_common_shares=2.44001600244002e+34
```

元テキストは発行済株式と自己株式の増減表であり、普通株式の株式分割、自己株式消却、
譲渡制限付株式報酬への充当による自己株式減少を説明している。
潜在普通株式数の明細ではない。

原因:

「譲渡制限付株式報酬」という語で `share_based_compensation` に分類され、
発行済株式表中の普通株式数を潜在普通株式数として誤抽出している。

## 原因パターン

1. `issued_shares_text` を潜在株式明細として扱っている。
   発行済株式/自己株式の増減表は、実際の発行済株式・自己株式の増減を説明するテキストであり、
   潜在株式数そのものではない。

2. `per_share_information_text` の数値を株数として拾っている。
   1株当たり情報にはEPS、純利益、期中平均株式数、潜在株式調整後EPSなどが混在するため、
   単純な近傍抽出では負値や桁違いの値が発生する。

3. 優先株式の転換倍率抽出が広すぎる。
   `1株につき` だけを根拠にすると、普通株式の株式分割比率を優先株式の転換倍率として誤認する。

4. テーブル由来のテキストで列境界が失われている。
   XBRL/iXBRLからのテキスト化で列の区切りが消えるため、`141,00036,459,000` のように
   隣接列の数値が連結され、巨大な数値として解釈される。

## 修正方針

1. `potential_common_shares` の数値抽出対象を、
   `share_options_text` と `share_acquisition_rights_text` など明細性の高いソースに限定する。

2. `issued_shares_text` は、潜在株式の存在フラグとしても原則使わない。
   使う場合は数値抽出を禁止し、別ラベルで監査対象にする。

3. `per_share_information_text` は、EPS注記として別扱いにする。
   「普通株式増加額」などを潜在株式数として使うなら、専用の表構造パーサで抽出する。

4. 優先株式の転換倍率は、
   「対象優先株式1株につき普通株式N株」または同等に明示された表現だけに限定する。
   普通株式の株式分割比率は除外する。

5. 修正後は `RULES.md` に従い、XBRLを全件再パースする。

## 結論

潜在株式関連の数値は、現状では合っていない。
特に `issued_shares_text` と `per_share_information_text` 由来の値は信頼不可として扱うべきである。
ロジック修正後に全件再パースし、外れ値件数が解消されたことを再検証する必要がある。
