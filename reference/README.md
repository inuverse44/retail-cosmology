# 公式の店舗数

`official_store_counts.csv` は各社の決算資料にある店舗数。Coles と Woolworths は 2026年6月28日、トライアルと西友は 2026年6月30日、コスモスは 2026年5月31日時点。地域の分け方は各社の資料のまま。

| source | 資料 | 表 | SHA-256（取得日 2026-11-03） |
|---|---|---|---|
| `coles_fy26` | Coles Group, [2026 Full Year Results Release](https://www.colesgroup.com.au/DownloadFile.axd?file=%2FReport%2FComNews%2F20260825%2F03125402.pdf)（2026-08-25） | Appendix 1: Number of retail stores（Supermarkets の行、計 870） | `c546caca909daffedadc698bfd367326a94f0c2b4f23134f55f8a056b59ef571` |
| `trial_fy26` | トライアルホールディングス, [本決算説明資料 2026年6月期](https://pdf.irpocket.com/C141A/xoA3/MuFf/T9TP/zgQJ.pdf)（2026-08-13） | 地域別の出店状況（17 ページ。トライアル 378 店と西友 243 店を別に載せる） | `96c50e2571af2d775cdf186b757acd221cb6061c9d5272335c661e83004d6d7e` |
| `cosmos_fy26` | コスモス薬品, [2026年5月期 決算短信〔日本基準〕（連結）](https://www2.jpx.co.jp/disc/33490/140120260625579741.pdf)（2026-07-13、TDnet） | 1. 経営成績等の概況の地区別店舗数の表（2026年5月末、計 1,708）。取得日 2026-11-11 | `812915baa53408e16ca6fb6aa082cc860ab532aa2d1780d2152116efcfa100d1` |
| `wow_f26` | Woolworths Group, [F26 results announcement](https://www.woolworthsgroup.com.au/content/dam/wwg/investors/asx-announcements/2026/3108623.pdf)（2026-08-26） | Appendix Six: Five-year store and trading area analysis（Total Australian Food、計 1,130） | `350d5cfd02ac0c7878b17064f1577e5de2ec4234e23d78c54a96aabf28118b1d` |

Woolworths の Total Australian Food は、スーパーと Metro の合計である。Appendix Seven の純増（Supermarkets +15、Metro −2）を足すと 13 店で、1,117 店から 1,130 店への増加と合う。Metro だけの店舗数は資料にない。

トライアルの地方別の数は、資料の棒グラフから業態ごとに読み取った。業態ごとに地方を足すと、資料の合計（メガセンター 31、スーパーセンター 229、smart 64、小型店 54）と一致する。資料に地方の定義は書かれていないので、8 地方区分（中国と四国はまとめる）を仮定している。

西友の地方別の数も、同じページの棒グラフから業態ごとに読み取った（ハイパーマーケット＋スーパーマーケット＋トライアル西友）。東北 2＋17＝19、関東 47＋84＋3＝134、中部 8＋63＝71、近畿 12＋7＝19。業態ごとに足すと資料の合計（ハイパーマーケット 69、スーパーマーケット 171、トライアル西友 3、計 243）と一致する。トライアル西友は西友の数に入っている（資料の 2 ページの注）。

コスモスの地区別の数は、決算短信の表の「当連結会計年度末店舗数」の列。地区の定義は資料に書かれていないので、8 地方区分の近畿を関西とし、中国と四国を分けたものと仮定している。北海道と東北には店がない（次期に福島県へ初出店の予定と資料にある）。
