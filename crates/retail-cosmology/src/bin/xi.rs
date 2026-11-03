//! 店舗の二点相関関数を、いくつかのランダムカタログで求め、jackknife で誤差をつける。
//! 人口そのものの二点相関関数も、人口に比例して撒いた点で求める。
//!
//! cargo run --release --bin xi -- 2026-11-01 [--country au|jp] [--n-random 20000] [--n-pop 2000]
//!                                            [--depth 6] [--seed 1] [--tag 名前]
//!
//! au（既定）：Coles と Woolworths。人口は ABS の格子人口。基準は陸地・人が住む格子・人口。
//!   読むもの：data/osm/stores_clean_<日付>.csv、data/abs/apgr25_1_0_0.tif
//!   書くもの：data/xi/xi_<日付>[_名前].csv、data/xi/chi2_<日付>[_名前].txt
//! jp：トライアル。人口は e-Stat の 1 km メッシュ。基準は人が住む格子・人口
//!   （メッシュのファイルには人口 0 の陸地が載っていないので、陸地の基準は作れない）。
//!   読むもの：data/osm/trial_clean_<日付>.csv、data/estat/tblT001140S*.txt
//!   書くもの：data/xi/xi_jp_<日付>[_名前].csv、data/xi/chi2_jp_<日付>[_名前].txt

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use retail_cosmology::albers::Albers;
use retail_cosmology::jackknife::{Estimate, JkCounts, Regions, chi2, gls_mean, jackknife};
use retail_cosmology::mesh::MeshGrid;
use retail_cosmology::popgrid::PopGrid;
use retail_cosmology::rng::SplitMix64;
use retail_cosmology::{Bins, LatLon, Unit, chord2, km_to_chord2};

/// χ² に使う最も短いビンの下端 [km]。格子（1 km）の影響を受ける 2–3 km のビンは除く。
const CHI2_MIN_KM: f64 = 3.0;
/// 比と dr/rr - 1 をまとめる距離の範囲 [km]（中点が 10–40 km の 4 ビン）
const MID_RANGE_KM: (f64, f64) = (7.0, 55.0);

struct Args {
    date: String,
    country: String,
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
        country: "au".into(),
        n_random: 20_000,
        n_pop: 2_000,
        depth: 6,
        seed: 1,
        tag: String::new(),
    };
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => a.country = v,
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

/// ランダムカタログの格子の重み。
#[derive(Clone, Copy)]
enum Weight {
    /// 陸地の格子すべてに等しい（オーストラリアだけ）
    Land,
    /// 人が住む格子（人口 > 0）すべてに等しい
    Inhabited,
    /// 格子の人口に比例
    Population,
}

impl Weight {
    fn name(self) -> &'static str {
        match self {
            Weight::Land => "land",
            Weight::Inhabited => "inhabited",
            Weight::Population => "population",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Weight::Land => "陸地",
            Weight::Inhabited => "人が住む格子",
            Weight::Population => "人口",
        }
    }

    /// 格子の人口 v に対する重み（陸地の外は呼ばれない）
    fn of(self, v: f64) -> f64 {
        match self {
            Weight::Land => 1.0,
            Weight::Inhabited => (v > 0.0) as u8 as f64,
            Weight::Population => v.max(0.0),
        }
    }
}

/// 人口の格子：オーストラリアは ABS の格子、日本は e-Stat のメッシュ。
enum Grid {
    Au(PopGrid),
    Jp(MeshGrid),
}

impl Grid {
    fn sample(&self, n: usize, w: Weight, rng: &mut SplitMix64) -> Vec<LatLon> {
        match self {
            Grid::Au(g) => g.sample(n, |v| w.of(v as f64), rng),
            Grid::Jp(g) => g.sample(n, |v| w.of(v), rng),
        }
    }
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
    let jp = match a.country.as_str() {
        "au" => false,
        "jp" => true,
        c => panic!("知らない国：{c}"),
    };
    let grid = if jp {
        let g =
            MeshGrid::read_dir(&root.join("data/estat")).expect("scripts/estat/fetch.sh を先に");
        eprintln!("メッシュ {} 個、人口の合計 {:.0}", g.cells.len(), g.total());
        Grid::Jp(g)
    } else {
        let g = PopGrid::read(&root.join("data/abs/apgr25_1_0_0.tif"))
            .expect("scripts/abs/fetch.sh を先に");
        assert_eq!(g.short_tiles, 0, "GeoTIFF のタイルを展開しきれなかった");
        Grid::Au(g)
    };
    let baselines: &[Weight] = if jp {
        &[Weight::Inhabited, Weight::Population]
    } else {
        &[Weight::Land, Weight::Inhabited, Weight::Population]
    };
    // jackknife の領域を作る平面座標。日本は北緯 36° で経度を縮めた正距円筒図法 [km] で足りる。
    let xy_of = |p: &LatLon| -> (f64, f64) {
        if jp {
            (p.lon * 36f64.to_radians().cos() * 111.32, p.lat * 110.57)
        } else {
            proj.forward(p)
        }
    };
    let prefix = if jp { "jp_" } else { "" };
    let store_file = if jp { "trial_clean" } else { "stores_clean" };

    let csv = fs::read_to_string(root.join(format!("data/osm/{store_file}_{}.csv", a.date)))
        .expect("osm_census を先に");
    let mut stores: Vec<(&str, Vec<LatLon>)> = if jp {
        vec![("TRIAL", vec![])]
    } else {
        vec![("Coles", vec![]), ("Woolworths", vec![])]
    };
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
    // 乱数を引く順：オーストラリアでは「人が住む格子」を最後に引き、それより前の基準の乱数の列を
    // この基準を足す前と同じに保つ（ノートの数字を再現できるように）。
    let mut randoms: Vec<(Weight, Vec<LatLon>)> = Vec::new();
    for w in baselines
        .iter()
        .filter(|w| jp || !matches!(w, Weight::Inhabited))
    {
        randoms.push((*w, grid.sample(a.n_random, *w, &mut rng)));
    }
    let pop_pts = grid.sample(a.n_pop, Weight::Population, &mut rng);
    if !jp {
        randoms.push((
            Weight::Inhabited,
            grid.sample(a.n_random, Weight::Inhabited, &mut rng),
        ));
    }
    let r_pop = &randoms
        .iter()
        .find(|(w, _)| matches!(w, Weight::Population))
        .unwrap()
        .1;

    // jackknife の領域：人口に比例するランダムの点の数がそろうように分ける
    let xy = |v: &[LatLon]| -> Vec<(f64, f64)> { v.iter().map(xy_of).collect() };
    let regions = Regions::build(&xy(r_pop), a.depth);
    let k = regions.len();
    let cat = |v: &[LatLon]| Catalogue {
        unit: v.iter().map(Unit::from).collect(),
        region: xy(v).into_iter().map(|p| regions.assign(p)).collect(),
    };
    let c_rand: Vec<(Weight, Catalogue)> = randoms.iter().map(|(w, v)| (*w, cat(v))).collect();
    let c_pp = cat(&pop_pts);
    let c_stores: Vec<(&str, Catalogue)> = stores.iter().map(|(n, v)| (*n, cat(v))).collect();
    eprintln!(
        "ランダム {} 点 ×{}、人口の点 {}、領域 {k}、種 {}（{:.1} 秒）",
        a.n_random,
        baselines.len(),
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
        for (w, r) in &c_rand {
            counts.push((name.to_string(), w.name(), jk(c, r)));
        }
        eprintln!("{name}：数え終えた（{:.1} 秒）", t0.elapsed().as_secs_f64());
    }
    for (w, r) in &c_rand {
        if !matches!(w, Weight::Population) {
            counts.push(("population".into(), w.name(), jk(&c_pp, r)));
        }
    }
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
    let pop_counts = |r: &str| {
        &counts
            .iter()
            .find(|(s, rr, _)| s == "population" && *rr == r)
            .unwrap()
            .2
    };
    let label = |r: &str| baselines.iter().find(|w| w.name() == r).unwrap().label();

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

        if *r != "population" && s != "population" {
            // 店と人口の比を、同じ領域を除いて計算し、まとめて jackknife にかける
            let pc = pop_counts(r);
            let ratio = jackknife(k, |o| {
                let a = c.pair_counts(o).landy_szalay();
                let b = pc.pair_counts(o).landy_szalay();
                a.iter().zip(&b).map(|(x, y)| x / y).collect()
            });
            put("ratio_to_population", s, r, &ratio);
            summaries.push(summarize(
                &format!("{s} の ξ_LS ÷ 人口の ξ_LS（{}が基準）", label(r)),
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
        root.join(format!("data/xi/xi_{prefix}{}{}.csv", a.date, a.tag)),
        out,
    )
    .unwrap();
    fs::write(
        root.join(format!("data/xi/chi2_{prefix}{}{}.txt", a.date, a.tag)),
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
