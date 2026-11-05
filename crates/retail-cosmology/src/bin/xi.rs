//! Client：店舗の二点相関関数を、いくつかのランダムカタログで測り、jackknife で誤差をつける。
//!
//! cargo run --release --bin xi -- 2026-11-01 [--country au|jp] [--stores パス] [--n-random 20000]
//!                                            [--n-pop 2000] [--depth 6] [--seed 1] [--tag 名前]
//!                                            [--footprint 50,100,200]
//!
//! --footprint：出店している地方を選択関数としてランダムカタログに入れる。値はならす幅 R [km] の一覧。
//! --mock R:種：店の代わりに、幅 R の選択関数に従って独立に撒いた模擬の店を使う（方法の確かめ）。
//!   チェーンごとに独立に撒くので、相互相関では「二社が互いに無関係に出店した」帰無仮説になる。
//! --mock-keep A：--mock のとき、チェーン A は撒き直さず実際の位置のまま使う（店の固まりを残した帰無仮説、#13）。
//! --selection-grid on：選択関数を約 10 km の粗い格子ごとに一度だけ求める近似を使う（速くするため、#12）。
//! --mock-density on：模擬データの重みに人口密度 ρ の γ 乗も掛け、人の密集地への寄り方を実際の店にそろえる。
//! --stores a.csv,b.csv：店のファイルをカンマで区切って複数渡せる。
//! --cross A,B：チェーン A と B の相互相関関数も測る（例：--cross TRIAL,SEIYU）。
//!
//! au（既定）：Coles と Woolworths。人口は ABS の格子人口。基準は陸地・人が住む格子・人口。
//!   読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//!   書くもの：data/xi/xi_<日付>[_名前].csv、data/xi/chi2_<日付>[_名前].txt
//! jp：トライアルと西友。人口は e-Stat の 1 km メッシュ。基準は人が住む格子・人口
//!   （メッシュのファイルには人口 0 の陸地が載っていないので、陸地の基準は作れない）。
//!   店は既定ではトライアルの公式サイトから取ったもの（日付はサイトから取った日）。--stores で OSM の店や、
//!   西友（data/seiyu_site/seiyu_site_clean_<日>.csv）に替えたり、足したりできる。
//!   読むもの：data/trial_site/trial_site_clean_<日付>.csv、data/estat/tblT001140S*.txt
//!   書くもの：data/xi/xi_jp_<日付>[_名前].csv、data/xi/chi2_jp_<日付>[_名前].txt

use std::path::PathBuf;

use retail_cosmology::manager::correlation::{Params, run};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => p.country = v,
            "--stores" => p.stores = v,
            "--n-random" => p.n_random = v.parse().unwrap(),
            "--n-pop" => p.n_pop = v.parse().unwrap(),
            "--depth" => p.depth = v.parse().unwrap(),
            "--seed" => p.seed = v.parse().unwrap(),
            "--tag" => p.tag = format!("_{v}"),
            "--mock" => {
                let (r, seed) = v.split_once(':').expect("--mock R:種");
                p.mock = Some((r.parse().unwrap(), seed.parse().unwrap()));
            }
            "--mock-density" => p.mock_density = v == "on",
            "--selection-grid" => p.selection_grid = v == "on",
            "--mock-keep" => p.mock_keep = v.split(',').map(String::from).collect(),
            "--cross" => {
                let (a, b) = v.split_once(',').expect("--cross チェーン,チェーン");
                p.cross = Some((a.into(), b.into()));
            }
            "--footprint" => p.footprint = v.split(',').map(|x| x.parse().unwrap()).collect(),
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if let Err(e) = run(&root, &p) {
        panic!("{e}");
    }
}
