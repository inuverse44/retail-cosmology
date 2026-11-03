//! 使用例：店舗の二点相関関数を、いくつかのランダムカタログで測り、jackknife で誤差をつける。
//! 人口そのものの二点相関関数も、人口に比例して撒いた点で測る。

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use crate::access::{population, results, stores};
use crate::contract::PopulationGrid;
use crate::engine::estimator::{Estimate, chi2, chi2_sf_even, gls_mean, jackknife};
use crate::engine::pair_count::{Bins, JkCounts, Regions};
use crate::engine::random_catalog::{Weight, sample};
use crate::util::geo::{LatLon, Unit, chord2, km_to_chord2};
use crate::util::rng::SplitMix64;

/// χ² に使う最も短いビンの下端 [km]。格子（1 km）の影響を受ける 2–3 km のビンは除く。
const CHI2_MIN_KM: f64 = 3.0;
/// 比と dr/rr - 1 をまとめる距離の範囲 [km]（中点が 10–40 km の 4 ビン）
const MID_RANGE_KM: (f64, f64) = (7.0, 55.0);

/// 測り方の設定。
#[derive(Clone, Debug)]
pub struct Params {
    pub date: String,
    /// au（Coles と Woolworths、ABS の格子人口）か jp（トライアル、e-Stat のメッシュ）
    pub country: String,
    /// 店のファイル（リポジトリの根からの相対パス）。空なら国ごとの既定
    pub stores: String,
    pub n_random: usize,
    pub n_pop: usize,
    pub depth: u32,
    pub seed: u64,
    /// 出力のファイル名につける名前（空でなければ "_名前" の形）
    pub tag: String,
}

impl Params {
    pub fn new(date: String) -> Self {
        Self {
            date,
            country: "au".into(),
            stores: String::new(),
            n_random: 20_000,
            n_pop: 2_000,
            depth: 6,
            seed: 1,
            tag: String::new(),
        }
    }
}

/// 一つのカタログ：単位ベクトルと、jackknife の領域の番号。
struct Catalogue {
    unit: Vec<Unit>,
    region: Vec<usize>,
}

/// 国ごとに違うもの：人口の格子、店のファイルの既定の場所、チェーン、出力の名前の前置き。
struct Dataset {
    grid: PopulationGrid,
    store_path: String,
    chains: Vec<&'static str>,
    prefix: &'static str,
}

fn dataset(root: &Path, p: &Params) -> Result<Dataset, Box<dyn std::error::Error>> {
    let default_path;
    let d = match p.country.as_str() {
        "au" => {
            default_path = format!("data/osm/stores_clean_{}.csv", p.date);
            Dataset {
                grid: population::read_abs(&root.join("data/abs/apgr25_1_0_0.tif"))?,
                store_path: String::new(),
                chains: vec!["Coles", "Woolworths"],
                prefix: "",
            }
        }
        "jp" => {
            default_path = format!("data/trial_site/trial_site_clean_{}.csv", p.date);
            Dataset {
                grid: population::read_estat(&root.join("data/estat"))?,
                store_path: String::new(),
                chains: vec!["TRIAL"],
                prefix: "jp_",
            }
        }
        c => return Err(format!("知らない国：{c}").into()),
    };
    let store_path = if p.stores.is_empty() {
        default_path
    } else {
        p.stores.clone()
    };
    Ok(Dataset { store_path, ..d })
}

/// 二点相関関数を測り、data/xi/xi_<前置き><日付><名前>.csv と chi2_….txt を書く。
/// 途中経過と、まとめた値・χ² は標準エラー出力に出す。
pub fn run(root: &Path, p: &Params) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let ds = dataset(root, p)?;
    let grid = &ds.grid;
    if !grid.has_land {
        let (_, total) = grid.land_and_total();
        eprintln!("メッシュ {} 個、人口の合計 {total:.0}", grid.values.len());
    }
    // 人口 0 の陸地の格子がなければ、陸地に一様な基準は作れない
    let baselines: Vec<Weight> = if grid.has_land {
        vec![Weight::Land, Weight::Inhabited, Weight::Population]
    } else {
        vec![Weight::Inhabited, Weight::Population]
    };

    let mut stores: Vec<(&str, Vec<LatLon>)> = ds.chains.iter().map(|c| (*c, Vec::new())).collect();
    for (chain, pos) in stores::read_selected(&root.join(&ds.store_path))
        .map_err(|e| format!("{} がない：{e}", ds.store_path))?
    {
        stores
            .iter_mut()
            .find(|(c, _)| *c == chain)
            .unwrap()
            .1
            .push(pos);
    }

    let mut rng = SplitMix64::new(p.seed);
    // 乱数を引く順：陸地の格子がある場合（オーストラリア）は「人が住む格子」を最後に引き、
    // それより前の基準の乱数の列を、この基準を足す前と同じに保つ（ノートの数字を再現できるように）。
    let deferred = grid.has_land;
    let mut randoms: Vec<(Weight, Vec<LatLon>)> = Vec::new();
    for w in baselines
        .iter()
        .filter(|w| !(deferred && **w == Weight::Inhabited))
    {
        randoms.push((*w, sample(grid, p.n_random, *w, &mut rng)));
    }
    let pop_pts = sample(grid, p.n_pop, Weight::Population, &mut rng);
    if deferred {
        randoms.push((
            Weight::Inhabited,
            sample(grid, p.n_random, Weight::Inhabited, &mut rng),
        ));
    }
    let r_pop = &randoms
        .iter()
        .find(|(w, _)| *w == Weight::Population)
        .unwrap()
        .1;

    // jackknife の領域：人口に比例するランダムの点の数がそろうように分ける
    let xy =
        |v: &[LatLon]| -> Vec<(f64, f64)> { v.iter().map(|q| grid.geometry.plane(q)).collect() };
    let regions = Regions::build(&xy(r_pop), p.depth);
    let k = regions.len();
    let cat = |v: &[LatLon]| Catalogue {
        unit: v.iter().map(Unit::from).collect(),
        region: xy(v).into_iter().map(|q| regions.assign(q)).collect(),
    };
    let c_rand: Vec<(Weight, Catalogue)> = randoms.iter().map(|(w, v)| (*w, cat(v))).collect();
    let c_pp = cat(&pop_pts);
    let c_stores: Vec<(&str, Catalogue)> = stores.iter().map(|(n, v)| (*n, cat(v))).collect();
    eprintln!(
        "ランダム {} 点 ×{}、人口の点 {}、領域 {k}、種 {}（{:.1} 秒）",
        p.n_random,
        baselines.len(),
        p.n_pop,
        p.seed,
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
        if *w != Weight::Population {
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
        let drr = jackknife(k, |o| c.pair_counts(o).dr_over_rr_minus_1());
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
            let pn = x.len();
            let hartlap = (k as f64 - pn as f64 - 2.0) / (k as f64 - 1.0);
            let line = match chi2(x, &cov) {
                Some(c2) if hartlap > 0.0 && pn.is_multiple_of(2) => {
                    let c2h = c2 * hartlap;
                    format!(
                        "{s} / {r}：{CHI2_MIN_KM} km 以上の {pn} ビン、χ² = {c2h:.1}（Hartlap {hartlap:.3} をかけた値）、上側確率 {:.3}",
                        chi2_sf_even(c2h, pn)
                    )
                }
                _ => format!("{s} / {r}：χ² を計算できない（領域 {k}、ビン {pn}）"),
            };
            eprintln!("{line}");
            writeln!(chi2_txt, "{line}").unwrap();
        }
    }
    for l in &summaries {
        writeln!(chi2_txt, "{l}").unwrap();
    }
    results::write_text(
        &root.join(format!("data/xi/xi_{}{}{}.csv", ds.prefix, p.date, p.tag)),
        &out,
    )?;
    results::write_text(
        &root.join(format!("data/xi/chi2_{}{}{}.txt", ds.prefix, p.date, p.tag)),
        &chi2_txt,
    )?;
    eprintln!("書いた（{:.1} 秒）", t0.elapsed().as_secs_f64());
    Ok(())
}
