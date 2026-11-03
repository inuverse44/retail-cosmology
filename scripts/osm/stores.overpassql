// Coles と Woolworths の店舗を、州・特別地域ごとに取り出す。
// 出力は TSV。::type が area の行のあとに続く行が、その州の店舗。
// 日付を固定して、同じ問い合わせが同じ結果を返すようにする（attic query）。
[out:csv(::type,::id,::lat,::lon,"ISO3166-2",shop,brand,"brand:wikidata",name;true;"\t")]
[timeout:600]
[date:"{{DATE}}"];
area["ISO3166-1"="AU"][admin_level=2]->.au;
area(area.au)[admin_level=4]["ISO3166-2"~"^AU-"]->.states;
foreach.states->.st(
  .st out tags;
  nwr(area.st)["brand"~"^(Coles|Woolworths)"];
  out center tags;
);
