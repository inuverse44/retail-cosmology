// トライアル（TRIAL）の店を、都道府県ごとに取り出す。
// 出力は TSV。::type が area の行のあとに続く行が、その都道府県の店舗。
// 日付を固定して、同じ問い合わせが同じ結果を返すようにする（attic query）。
[out:csv(::type,::id,::lat,::lon,"ISO3166-2",shop,brand,"brand:wikidata",name;true;"\t")]
[timeout:1800]
[date:"{{DATE}}"];
area["ISO3166-1"="JP"][admin_level=2]->.jp;
area(area.jp)[admin_level=4]["ISO3166-2"~"^JP-"]->.prefs;
foreach.prefs->.st(
  .st out tags;
  // 数えるのは shop のタグがある要素だけなので、先に shop で絞って名前を調べる
  nwr(area.st)["shop"]->.shops;
  (
    nwr.shops["brand:wikidata"="Q11321723"];
    nwr.shops["brand"~"トライアル|TRIAL"];
    nwr.shops["name"~"トライアル|TRIAL"];
  );
  out center tags;
);
