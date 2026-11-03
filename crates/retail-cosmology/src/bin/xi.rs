//! Coles と Woolworths の二点相関関数を、二種類のランダムカタログで求める。
//! 人口そのものの二点相関関数も、人口に比例して撒いた点で求める。
//!
//! cargo run --release --bin xi -- 2026-11-01
//!
//! 読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//! 書くもの：data/xi/xi_<日付>.csv

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use retail_cosmology::popgrid::{PopGrid, population, uniform_land};
use retail_cosmology::rng::SplitMix64;
use retail_cosmology::{Bins, LatLon, PairCounts, Unit, chord2, km_to_chord2};

/// ランダムカタログの点の数
const N_RANDOM: usize = 20_000;
/// 人口の二点相関関数に使う点の数（店舗の数と同じ程度）
const N_POP: usize = 2_000;

fn main() {
    let date = std::env::args().nth(1).expect("日付（YYYY-MM-DD）を渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t0 = Instant::now();

    let grid = PopGrid::read(&root.join("data/abs/apgr25_1_0_0.tif"))
        .expect("scripts/abs/fetch.sh を先に");
    let (land, total) = grid.land_and_total();
    eprintln!(
        "格子 {}×{}、格子の幅 {}×{} m、陸地の格子 {land}、人口の合計 {total:.0}、展開しきれなかったタイル {}",
        grid.width, grid.height, grid.dx, grid.dy, grid.short_tiles
    );
    assert_eq!(grid.short_tiles, 0, "GeoTIFF のタイルを展開しきれなかった");

    // 店舗を読み、格子との重なりを確かめる
    let csv = fs::read_to_string(root.join(format!("data/osm/stores_clean_{date}.csv")))
        .expect("osm_census を先に");
    let mut chains: Vec<(&str, Vec<LatLon>)> = vec![("Coles", vec![]), ("Woolworths", vec![])];
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.splitn(6, ',').collect();
        let p = LatLon {
            lat: f[3].parse().unwrap(),
            lon: f[4].parse().unwrap(),
        };
        chains
            .iter_mut()
            .find(|(c, _)| *c == f[0])
            .unwrap()
            .1
            .push(p);
    }
    for (name, pts) in &chains {
        let (mut sea, mut empty) = (0, 0);
        for p in pts {
            match grid.cell_of(p).map(|i| grid.values[i]) {
                Some(v) if v > 0.0 => {}
                Some(0.0) => empty += 1,
                _ => sea += 1,
            }
        }
        eprintln!(
            "{name}: {} 店。陸地の外の格子に {sea} 店、人口 0 の格子に {empty} 店",
            pts.len()
        );
    }

    let mut rng = SplitMix64::new(20261103);
    let r_land = grid.sample(N_RANDOM, uniform_land, &mut rng);
    let r_pop = grid.sample(N_RANDOM, population, &mut rng);
    let pop_pts = grid.sample(N_POP, population, &mut rng);
    eprintln!(
        "ランダムカタログを撒いた（{:.1} 秒）",
        t0.elapsed().as_secs_f64()
    );

    let bins = Bins::log(2.0, 2000.0, 15);
    let cbins = bins.map(km_to_chord2);
    let to_unit = |v: &[LatLon]| -> Vec<Unit> { v.iter().map(Unit::from).collect() };
    let (u_land, u_pop) = (to_unit(&r_land), to_unit(&r_pop));

    let mut cases: Vec<(String, &str, Vec<Unit>, &Vec<Unit>)> = Vec::new();
    for (name, pts) in &chains {
        cases.push((name.to_string(), "land", to_unit(pts), &u_land));
        cases.push((name.to_string(), "population", to_unit(pts), &u_pop));
    }
    cases.push(("population".into(), "land", to_unit(&pop_pts), &u_land));

    let mut out = String::from("sample,random,r_lo_km,r_hi_km,n_d,n_r,dd,dr,rr,xi_ls\n");
    for (sample, random, data, randoms) in &cases {
        let c = PairCounts::compute(data, randoms, &cbins, chord2);
        let ls = c.landy_szalay();
        for k in 0..bins.len() {
            let e = bins.edges();
            writeln!(
                out,
                "{sample},{random},{},{},{},{},{},{},{},{}",
                e[k],
                e[k + 1],
                c.n_d,
                c.n_r,
                c.dd[k],
                c.dr[k],
                c.rr[k],
                ls[k]
            )
            .unwrap();
        }
        eprintln!(
            "{sample} / {random}：数え終えた（{:.1} 秒）",
            t0.elapsed().as_secs_f64()
        );
    }
    fs::create_dir_all(root.join("data/xi")).unwrap();
    let path = root.join(format!("data/xi/xi_{date}.csv"));
    fs::write(&path, out).unwrap();
    eprintln!("{}", path.display());
}
