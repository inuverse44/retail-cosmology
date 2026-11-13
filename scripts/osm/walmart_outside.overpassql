// 米国本土の外（アラスカ、ハワイ、プエルトリコ）の Walmart を取り出す（#51）。
// 解析には使わず、10-K の全米の店数（50 州、D.C.、プエルトリコ）と OSM の店数を比べるためだけに使う。
// 選び方は walmart.overpassql と同じ（店の選び方は Rust の engine/selection.rs）。日付を固定しない理由も同じ。
// 矩形：アラスカ（北緯 51–72°、西経 129–180°）、ハワイ（北緯 18.5–22.5°、西経 154–161°）、プエルトリコ（北緯 17.8–18.6°、西経 65.2–67.5°）。
[out:csv(::type,::id,::lat,::lon,"ISO3166-2",shop,brand,"brand:wikidata",name;true;"\t")]
[timeout:300];
(
  nwr["brand:wikidata"~"^(Q483551|Q7963529)$"](51,-180,72,-129);
  nwr["brand:wikidata"~"^(Q483551|Q7963529)$"](18.5,-161,22.5,-154);
  nwr["brand:wikidata"~"^(Q483551|Q7963529)$"](17.8,-67.5,18.6,-65.2);
  nwr["brand"="Walmart"](51,-180,72,-129);
  nwr["brand"="Walmart"](18.5,-161,22.5,-154);
  nwr["brand"="Walmart"](17.8,-67.5,18.6,-65.2);
);
out center tags;
