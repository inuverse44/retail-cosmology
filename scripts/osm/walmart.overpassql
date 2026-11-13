// Walmart の店を、米国本土を囲む矩形（北緯 24–50°、西経 60–130°）から取り出す（#51）。
// 出力は TSV。州は Rust の側で Natural Earth の州境から決める（矩形にはカナダとメキシコの一部も入るが、そこで落ちる）。
// ブランド（Walmart は brand:wikidata=Q483551、Neighborhood Market は Q7963529）に加え、
// ブランドのない店を店名の完全な一致で拾う。
// ガソリンスタンドや薬局など、店の中の売り場もブランドが同じなので、店の選び方は Rust（engine/selection.rs）で決める。
//
// ほかの国と違い、日付を固定した問い合わせ（attic query）はしない。2026-11-13 に試したところ、
// 州ごとに分けると 1 州に約 2 分かかって 30 分の時間切れに届かず、矩形一つでも 15 分で時間切れになった。
// 日付を固定しなければ約 3 分で返る。代わりに、取り出したデータの時刻（timestamp_osm_base）を fetch.sh が別のファイルに残す。
[out:csv(::type,::id,::lat,::lon,"ISO3166-2",shop,brand,"brand:wikidata",name;true;"\t")]
[timeout:900]
[bbox:24,-130,50,-60];
(
  nwr["brand:wikidata"="Q483551"];
  nwr["brand:wikidata"="Q7963529"];
  nwr["brand"="Walmart"];
  nwr["brand"="Walmart Neighborhood Market"];
  nwr["name"="Walmart"];
  nwr["name"="Walmart Supercenter"];
  nwr["name"="Walmart Neighborhood Market"];
);
out center tags;
