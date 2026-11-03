//! Coles と Woolworths の二点相関関数を、二種類のランダムカタログで求め、jackknife で誤差をつける。
//! 人口そのものの二点相関関数も、人口に比例して撒いた点で求める。
//!
//! cargo run --release --bin xi -- 2026-11-01 [--n-random 20000] [--n-pop 2000] [--depth 6] [--seed 1] [--tag 名前]
//!
//! 読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//! 書くもの：data/xi/xi_<日付>[_名前].csv（値と jackknife の標準偏差）
//!           data/xi/chi2_<日付>[_名前].txt（Landy–Szalay が 0 と合うかの χ²）

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use retail_cosmology::albers::Albers;
use retail_cosmology::jackknife::{Estimate, JkCounts, Regions, chi2, gls_mean, jackknife};
use retail_cosmology::popgrid::{PopGrid, population, uniform_land};
use retail_cosmology::rng::SplitMix64;
use retail_cosmology::{Bins, LatLon, Unit, chord2, km_to_chord2};

/// χ² に使う最も短いビンの下端 [km]。格子（1 km）の影響を受ける 2–3 km のビンは除く。
const CHI2_MIN_KM: f64 = 3.0;
/// 比と dr/rr - 1 をまとめる距離の範囲 [km]（中点が 10–40 km の 4 ビン）
const MID_RANGE_KM: (f64, f64) = (7.0, 55.0);

struct Args {
    date: String,
    n_random: usize,
    n_pop: usize,
    depth: u32,
    seed: u64,
    tag: String,
}

fn args() -> Args {
    let mut it = std::env::args().skip(1);
    let mut a = Args {
        date: it.next().expect("日付（YYYY-MM-DD）を渡す"),
        n_random: 20_000,
        n_pop: 2_000,
        depth: 6,
        seed: 1,
        tag: String::new(),
    };
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--n-random" => a.n_random = v.parse().unwrap(),
            "--n-pop" => a.n_pop = v.parse().unwrap(),
            "--depth" => a.depth = v.parse().unwrap(),
            "--seed" => a.seed = v.parse().unwrap(),
            "--tag" => a.tag = format!("_{v}"),
            _ => panic!("知らない引数：{k}"),
        }
    }
    a
}

/// 自由度が偶数のときの χ² 分布の上側確率 P(χ² > x) = e^{-x/2} Σ_{i<k/2} (x/2)^i / i!
fn chi2_sf_even(x: f64, dof: usize) -> f64 {
    assert!(dof.is_multiple_of(2));
    let h = x / 2.0;
    let mut term = 1.0;
    let mut s = 1.0;
    for i in 1..dof / 2 {
        term *= h / i as f64;
        s += term;
    }
    (-h).exp() * s
}

/// 一つのカタログ：点、単位ベクトル、jackknife の領域の番号。
struct Catalogue {
    unit: Vec<Unit>,
    region: Vec<usize>,
}

fn main() {
    let a = args();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let t0 = Instant::now();
    let proj = Albers::epsg3577();

    let grid = PopGrid::read(&root.join("data/abs/apgr25_1_0_0.tif"))
        .expect("scripts/abs/fetch.sh を先に");
    assert_eq!(grid.short_tiles, 0, "GeoTIFF のタイルを展開しきれなかった");

    let csv = fs::read_to_string(root.join(format!("data/osm/stores_clean_{}.csv", a.date)))
        .expect("osm_census を先に");
    let mut stores: Vec<(&str, Vec<LatLon>)> = vec![("Coles", vec![]), ("Woolworths", vec![])];
    for line in csv.lines().skip(1) {
        let f: Vec<&str> = line.splitn(6, ',').collect();
        let p = LatLon {
            lat: f[3].parse().unwrap(),
            lon: f[4].parse().unwrap(),
        };
        stores
            .iter_mut()
            .find(|(c, _)| *c == f[0])
            .unwrap()
            .1
            .push(p);
    }

    let mut rng = SplitMix64::new(a.seed);
    let r_land = grid.sample(a.n_random, uniform_land, &mut rng);
    let r_pop = grid.sample(a.n_random, population, &mut rng);
    let pop_pts = grid.sample(a.n_pop, population, &mut rng);

    // jackknife の領域：人口に比例するランダムの点の数がそろうように分ける
    let xy = |v: &[LatLon]| -> Vec<(f64, f64)> { v.iter().map(|p| proj.forward(p)).collect() };
    let regions = Regions::build(&xy(&r_pop), a.depth);
    let k = regions.len();
    let cat = |v: &[LatLon]| Catalogue {
        unit: v.iter().map(Unit::from).collect(),
        region: xy(v).into_iter().map(|p| regions.assign(p)).collect(),
    };
    let (c_land, c_pop, c_pp) = (cat(&r_land), cat(&r_pop), cat(&pop_pts));
    let c_stores: Vec<(&str, Catalogue)> = stores.iter().map(|(n, v)| (*n, cat(v))).collect();
    eprintln!(
        "ランダム {} 点 ×2、人口の点 {}、領域 {k}、種 {}（{:.1} 秒）",
        a.n_random,
        a.n_pop,
        a.seed,
        t0.elapsed().as_secs_f64()
    );

    let bins = Bins::log(2.0, 2000.0, 15);
    let cbins = bins.map(km_to_chord2);
    let jk = |d: &Catalogue, r: &Catalogue| {
        JkCounts::compute(&d.unit, &d.region, &r.unit, &r.region, k, &cbins, chord2)
    };

    // 計算する組：(標本, 基準) → 対の数
    let mut counts: Vec<(String, &str, JkCounts)> = Vec::new();
    for (name, c) in &c_stores {
        counts.push((name.to_string(), "land", jk(c, &c_land)));
        counts.push((name.to_string(), "population", jk(c, &c_pop)));
        eprintln!("{name}：数え終えた（{:.1} 秒）", t0.elapsed().as_secs_f64());
    }
    counts.push(("population".into(), "land", jk(&c_pp, &c_land)));
    eprintln!("人口：数え終えた（{:.1} 秒）", t0.elapsed().as_secs_f64());

    let mut out = String::from("quantity,sample,random,r_lo_km,r_hi_km,value,sigma\n");
    let mut put = |q: &str, s: &str, r: &str, e: &Estimate| {
        let ed = bins.edges();
        for b in 0..bins.len() {
            writeln!(
                out,
                "{q},{s},{r},{},{},{},{}",
                ed[b],
                ed[b + 1],
                e.value[b],
                e.sigma[b]
            )
            .unwrap();
        }
    };
    let mut chi2_txt = String::new();
    let first = bins.edges().iter().position(|e| *e >= CHI2_MIN_KM).unwrap();
    let pop_land = &counts
        .iter()
        .find(|(s, r, _)| s == "population" && *r == "land")
        .unwrap()
        .2;

    let ed = bins.edges().to_vec();
    let mid: Vec<usize> = (0..bins.len())
        .filter(|b| ed[*b] >= MID_RANGE_KM.0 && ed[b + 1] <= MID_RANGE_KM.1)
        .collect();
    let summarize = |label: &str, e: &Estimate| {
        let x: Vec<f64> = mid.iter().map(|b| e.value[*b]).collect();
        let cov: Vec<Vec<f64>> = mid
            .iter()
            .map(|i| mid.iter().map(|j| e.cov[*i][*j]).collect())
            .collect();
        let hartlap = (k as f64 - mid.len() as f64 - 2.0) / (k as f64 - 1.0);
        let line = match gls_mean(&x, &cov, hartlap) {
            Some((m, sd)) => format!(
                "{label}：{:.1}–{:.1} km の {} ビンをまとめた値 {m:.3} ± {sd:.3}",
                ed[mid[0]],
                ed[mid[mid.len() - 1] + 1],
                mid.len()
            ),
            None => format!("{label}：まとめられない"),
        };
        eprintln!("{line}");
        line
    };
    let mut summaries = Vec::new();

    for (s, r, c) in &counts {
        let ls = jackknife(k, |o| c.pair_counts(o).landy_szalay());
        let nat = jackknife(k, |o| c.pair_counts(o).natural());
        let drr = jackknife(k, |o| {
            let p = c.pair_counts(o);
            let (nd, nr) = (p.n_d as f64, p.n_r as f64);
            (0..p.dd.len())
                .map(|b| {
                    (p.dr[b] as f64 / (nd * nr)) / (p.rr[b] as f64 / (nr * (nr - 1.0) / 2.0)) - 1.0
                })
                .collect()
        });
        put("ls", s, r, &ls);
        put("natural", s, r, &nat);
        put("dr_over_rr_minus_1", s, r, &drr);

        if *r == "land" && s != "population" {
            // 店と人口の比を、同じ領域を除いて計算し、まとめて jackknife にかける
            let ratio = jackknife(k, |o| {
                let a = c.pair_counts(o).landy_szalay();
                let b = pop_land.pair_counts(o).landy_szalay();
                a.iter().zip(&b).map(|(x, y)| x / y).collect()
            });
            put("ratio_to_population", s, r, &ratio);
            summaries.push(summarize(
                &format!("{s} の ξ_LS ÷ 人口の ξ_LS（陸地が基準）"),
                &ratio,
            ));
        }
        if *r == "population" {
            summaries.push(summarize(&format!("{s} の dr/rr - 1（人口が基準）"), &drr));
            // Landy–Szalay が全部のビンで 0 と合うかを、共分散を使った χ² で調べる。
            // jackknife の共分散の逆行列の偏りを Hartlap の因子 (K - p - 2)/(K - 1) で直す。
            let x = &ls.value[first..];
            let cov: Vec<Vec<f64>> = ls.cov[first..]
                .iter()
                .map(|row| row[first..].to_vec())
                .collect();
            let p = x.len();
            let hartlap = (k as f64 - p as f64 - 2.0) / (k as f64 - 1.0);
            let line = match chi2(x, &cov) {
                Some(c2) if hartlap > 0.0 && p.is_multiple_of(2) => {
                    let c2h = c2 * hartlap;
                    format!(
                        "{s} / {r}：{CHI2_MIN_KM} km 以上の {p} ビン、χ² = {c2h:.1}（Hartlap {hartlap:.3} をかけた値）、上側確率 {:.3}",
                        chi2_sf_even(c2h, p)
                    )
                }
                _ => format!("{s} / {r}：χ² を計算できない（領域 {k}、ビン {p}）"),
            };
            eprintln!("{line}");
            writeln!(chi2_txt, "{line}").unwrap();
        }
    }
    for l in &summaries {
        writeln!(chi2_txt, "{l}").unwrap();
    }
    fs::create_dir_all(root.join("data/xi")).unwrap();
    fs::write(
        root.join(format!("data/xi/xi_{}{}.csv", a.date, a.tag)),
        out,
    )
    .unwrap();
    fs::write(
        root.join(format!("data/xi/chi2_{}{}.txt", a.date, a.tag)),
        chi2_txt,
    )
    .unwrap();
    eprintln!("書いた（{:.1} 秒）", t0.elapsed().as_secs_f64());
}

#[cfg(test)]
mod tests {
    use super::chi2_sf_even;

    #[test]
    fn chi2_survival_matches_known_values() {
        // 自由度 2 では P(χ² > x) = e^{-x/2}
        assert!((chi2_sf_even(2.0, 2) - (-1.0f64).exp()).abs() < 1e-12);
        // 自由度 14 の 95% 点は 23.685（χ² 分布表）
        assert!((chi2_sf_even(23.685, 14) - 0.05).abs() < 1e-4);
    }
}
