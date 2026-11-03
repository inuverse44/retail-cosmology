# 公式の店舗数

`official_store_counts.csv` は各社の決算資料にある 2026年6月28日時点の店舗数。州の分け方は各社の表のまま。

| source | 資料 | 表 | SHA-256（取得日 2026-11-03） |
|---|---|---|---|
| `coles_fy26` | Coles Group, [2026 Full Year Results Release](https://www.colesgroup.com.au/DownloadFile.axd?file=%2FReport%2FComNews%2F20260825%2F03125402.pdf)（2026-08-25） | Appendix 1: Number of retail stores（Supermarkets の行、計 870） | `c546caca909daffedadc698bfd367326a94f0c2b4f23134f55f8a056b59ef571` |
| `wow_f26` | Woolworths Group, [F26 results announcement](https://www.woolworthsgroup.com.au/content/dam/wwg/investors/asx-announcements/2026/3108623.pdf)（2026-08-26） | Appendix Six: Five-year store and trading area analysis（Total Australian Food、計 1,130） | `350d5cfd02ac0c7878b17064f1577e5de2ec4234e23d78c54a96aabf28118b1d` |

Woolworths の Total Australian Food は、スーパーと Metro の合計である。Appendix Seven の純増（Supermarkets +15、Metro −2）を足すと 13 店で、1,117 店から 1,130 店への増加と合う。Metro だけの店舗数は資料にない。
