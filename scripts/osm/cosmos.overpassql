// コスモス（コスモス薬品、ドラッグストアコスモス）の店を、都道府県ごとに取り出す（#51）。
// 出力は TSV。::type が area の行のあとに続く行が、その都道府県の店舗。
// 日付を固定して、同じ問い合わせが同じ結果を返すようにする（attic query）。
// 名前だけで「コスモス」を探すと、関係のない調剤薬局（コスモス調剤薬局など）やコスモ石油まで拾い、
// 問い合わせも時間切れになる。そこでトライアルと同じく、shop のタグがある要素に絞ってから、
// ブランド（brand:wikidata=Q11302127、brand）か、チェーンの店名（ドラッグストアコスモス、ディスカウントドラッグコスモス）で選ぶ。
[out:csv(::type,::id,::lat,::lon,"ISO3166-2",shop,brand,"brand:wikidata",name;true;"\t")]
[timeout:1800]
[date:"{{DATE}}"];
area["ISO3166-1"="JP"][admin_level=2]->.jp;
area(area.jp)[admin_level=4]["ISO3166-2"~"^JP-"]->.prefs;
foreach.prefs->.st(
  .st out tags;
  nwr(area.st)["shop"]->.shops;
  (
    nwr.shops["brand:wikidata"="Q11302127"];
    nwr.shops["brand"~"コスモス"];
    nwr.shops["name"~"ドラッグ ?ストア ?コスモス|ディスカウント ?ドラッグ ?コスモス|コスモス薬品"];
  );
  out center tags;
);
