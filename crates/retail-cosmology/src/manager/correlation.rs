//! 使用例：店舗の二点相関関数を、いくつかのランダムカタログで測り、jackknife で誤差をつける。
//! 人口そのものの二点相関関数も、人口に比例して撒いた点で測る。
//! `cross` を渡すと、二つのチェーンの相互相関関数（Landy–Szalay）も測る。

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use crate::access::{population, results, stores};
use crate::contract::PopulationGrid;
use crate::engine::estimator::{
    Estimate, chi2, chi2_sf, chi2_sf_even, gls_mean, inverse_variance_mean, jackknife, plain_mean,
};
use crate::engine::pair_count::{Bins, JkCounts, JkCross, Regions, count_auto_by_region};
use crate::engine::random_catalog::{Weight, sample, sample_selected};
use crate::engine::selection_function::{
    SmoothRatio, Tabulated, fit_density_exponent, mean_log_density,
};
use crate::util::geo::{LatLon, Unit, chord2, km_to_chord2};
use crate::util::rng::SplitMix64;

/// χ² に使う最も短いビンの下端 [km]。格子（1 km）の影響を受ける 2–3 km のビンは除く。
const CHI2_MIN_KM: f64 = 3.0;
/// 比と dr/rr - 1 をまとめる距離の範囲 [km]（中点が 10–40 km の 4 ビン）
const MID_RANGE_KM: (f64, f64) = (7.0, 55.0);
/// 選択関数の人口の密度を表す、人口に比例して撒いた参照の点の数（既定）
pub const N_REF: usize = 50_000;

/// 測り方の設定。
#[derive(Clone, Debug)]
pub struct Params {
    pub date: String,
    /// au（Coles と Woolworths、ABS の格子人口）か jp（トライアル、e-Stat のメッシュ）
    pub country: String,
    /// 店のファイル（リポジトリの根からの相対パス）。カンマで区切って複数渡せる。空なら国ごとの既定
    pub stores: String,
    pub n_random: usize,
    /// 選択関数の人口の密度を表す参照の点の数（既定は N_REF）
    pub n_ref: usize,
    pub n_pop: usize,
    pub depth: u32,
    pub seed: u64,
    /// 出力のファイル名につける名前（空でなければ "_名前" の形）
    pub tag: String,
    /// 選択関数（出店している地方）をならす幅 R [km] の一覧。空なら選択関数を使わない
    pub footprint: Vec<f64>,
    /// 帰無仮説の模擬データ：(R [km], 種)。実際の店から幅 R の選択関数を作り、それに従って店と同じ数の点を
    /// 独立に撒いて、店の代わりに使う（短い距離の相関がない店）。選択関数の方法の確かめに使う。
    /// R = 0 なら選択関数を使わず、人口だけに比例して撒く（人口が基準の χ² の校正に使う、#9）。
    pub mock: Option<(f64, u64)>,
    /// 模擬データを撒く重みに、人口密度 ρ（格子の人口）の γ 乗も掛ける。γ はチェーンごとに、
    /// 店の位置の ln ρ の平均が実際の店と同じになるように決める（人の密集地への寄り方をそろえる）。
    pub mock_density: bool,
    /// 選択関数を約 10 km の粗い格子ごとに一度だけ求め、格子の中では同じ値を使う（速くするための近似、#12）。
    pub selection_grid: bool,
    /// 模擬データで撒き直さず、実際の位置のまま残すチェーン（#13）。店の固まりを残した帰無仮説に使う
    pub mock_keep: Vec<String>,
    /// 相互相関を測る二つのチェーンの名前（例：TRIAL と SEIYU）。None なら測らない
    pub cross: Option<(String, String)>,
}

impl Params {
    pub fn new(date: String) -> Self {
        Self {
            date,
            country: "au".into(),
            stores: String::new(),
            n_random: 20_000,
            n_ref: N_REF,
            n_pop: 2_000,
            depth: 6,
            seed: 1,
            tag: String::new(),
            footprint: Vec::new(),
            mock: None,
            mock_density: false,
            selection_grid: false,
            mock_keep: Vec::new(),
            cross: None,
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
                chains: vec!["TRIAL", "SEIYU"],
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
    for path in ds.store_path.split(',') {
        for (chain, pos) in
            stores::read_selected(&root.join(path)).map_err(|e| format!("{path} がない：{e}"))?
        {
            stores
                .iter_mut()
                .find(|(c, _)| *c == chain)
                .ok_or_else(|| format!("{path} に知らないチェーン：{chain}"))?
                .1
                .push(pos);
        }
    }
    // 店のファイルに出てこないチェーンは測らない
    stores.retain(|(_, v)| !v.is_empty());
    let cross_pair: Option<(usize, usize)> = match &p.cross {
        None => None,
        Some((a, b)) => {
            let idx = |n: &str| {
                stores
                    .iter()
                    .position(|(c, _)| *c == n)
                    .ok_or_else(|| format!("相互相関の {n} の店がない"))
            };
            Some((idx(a)?, idx(b)?))
        }
    };

    if let Some((r_km, mock_seed)) = p.mock {
        let mut rng_m = SplitMix64::new(mock_seed ^ 0x4D0C_C0DE_0000_0000);
        let refs_ll = sample(grid, p.n_ref, Weight::Population, &mut rng_m);
        let refs: Vec<Unit> = refs_ll.iter().map(Unit::from).collect();
        // 格子の人口（人口密度 [人/km²] にあたる）。格子の外か人口 0 なら None
        let rho = |q: &LatLon| -> Option<f64> {
            let v = grid.values[grid.geometry.cell_of(q)?] as f64;
            (v > 0.0).then_some(v)
        };
        for (name, pts) in stores.iter_mut() {
            if p.mock_keep.iter().any(|k| k == name) {
                eprintln!("{name}：撒き直さず、実際の {} 店のまま使う", pts.len());
                continue;
            }
            let units: Vec<Unit> = pts.iter().map(Unit::from).collect();
            // R = 0 なら選択関数を使わず（S = 1）、人口だけに比例して撒く（人口が基準の帰無仮説、#9）
            let sr = (r_km > 0.0).then(|| SmoothRatio::new(&units, &refs, r_km));
            let tab = sr.as_ref().map(|sr| Tabulated::new(sr, &grid.geometry));
            let s_of = |q: &LatLon| match (&sr, &tab) {
                (Some(_), Some(tab)) if p.selection_grid => tab.eval(q),
                (Some(sr), _) => sr.eval(&Unit::from(q)),
                _ => 1.0,
            };
            let n = pts.len();
            let gamma = if p.mock_density {
                // 基準は「人口 × 選択関数」。人口に比例する参照の点に選択関数の重みをつけて表す
                let (lr, w): (Vec<f64>, Vec<f64>) = refs_ll
                    .iter()
                    .zip(&refs)
                    .filter_map(|(q, _)| rho(q).map(|r| (r.ln(), s_of(q))))
                    .unzip();
                let ls: Vec<f64> = pts.iter().filter_map(|q| rho(q).map(f64::ln)).collect();
                let target = ls.iter().sum::<f64>() / ls.len() as f64;
                let g = fit_density_exponent(&lr, &w, target, 3.0);
                eprintln!(
                    "{name}：店の ln ρ の平均 {target:.3}（人口 0 の格子の {} 店を除く）、基準の平均 {:.3}、γ = {g:.3}",
                    n - ls.len(),
                    mean_log_density(&lr, &w, 0.0)
                );
                g
            } else {
                0.0
            };
            let sel = sample_selected(
                grid,
                n,
                Weight::Population,
                |q| {
                    let s = s_of(q);
                    if gamma == 0.0 {
                        s
                    } else {
                        s * rho(q).map_or(0.0, |r| r.powf(gamma))
                    }
                },
                if p.mock_density { 100 * n } else { 20 * n },
                &mut rng_m,
            );
            *pts = sel.points;
            if p.mock_density {
                let ls: Vec<f64> = pts.iter().filter_map(|q| rho(q).map(f64::ln)).collect();
                eprintln!(
                    "{name}：模擬の店の ln ρ の平均 {:.3}、候補 {} 点、上限を超えた候補 {}",
                    ls.iter().sum::<f64>() / ls.len() as f64,
                    sel.candidates,
                    sel.clipped
                );
            }
            eprintln!(
                "{name}：模擬データ（R = {r_km} km の選択関数に従う {n} 点、種 {mock_seed}）"
            );
        }
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

    // 基準のランダムカタログどうしの組（RR）は、どの標本にも共通なので一度だけ数える
    let rr_rand: Vec<_> = c_rand
        .iter()
        .map(|(_, r)| count_auto_by_region(&r.unit, &r.region, k, &cbins, chord2))
        .collect();
    let jk_shared = |d: &Catalogue, i: usize| {
        let r = &c_rand[i].1;
        JkCounts::with_rr(
            &d.unit,
            &d.region,
            &r.unit,
            &r.region,
            k,
            &cbins,
            chord2,
            rr_rand[i].clone(),
        )
    };
    eprintln!(
        "基準の RR：数え終えた（{:.1} 秒）",
        t0.elapsed().as_secs_f64()
    );

    // 計算する組：(標本, 基準) → 対の数
    let mut counts: Vec<(String, String, JkCounts)> = Vec::new();
    for (name, c) in &c_stores {
        for (i, (w, _)) in c_rand.iter().enumerate() {
            counts.push((name.to_string(), w.name().to_string(), jk_shared(c, i)));
        }
        eprintln!("{name}：数え終えた（{:.1} 秒）", t0.elapsed().as_secs_f64());
    }
    for (i, (w, _)) in c_rand.iter().enumerate() {
        if *w != Weight::Population {
            counts.push((
                "population".into(),
                w.name().to_string(),
                jk_shared(&c_pp, i),
            ));
        }
    }
    eprintln!("人口：数え終えた（{:.1} 秒）", t0.elapsed().as_secs_f64());

    // 選択関数：店の密度と人口の密度を幅 R でならした比 S を、人口に掛けてランダムカタログを撒く。
    // 乱数はほかの基準とは別の列から引き、選択関数を使わないときの結果を変えない。
    // 選択関数つきのランダムカタログ：(R [km], チェーンの番号, カタログ)。相互相関でも使う
    let mut fp_cats: Vec<(f64, usize, Catalogue)> = Vec::new();
    if !p.footprint.is_empty() {
        let mut rng_fp = SplitMix64::new(p.seed ^ 0xF007_9417_0000_0000);
        let refs: Vec<Unit> = sample(grid, p.n_ref, Weight::Population, &mut rng_fp)
            .iter()
            .map(Unit::from)
            .collect();
        // 1 つ目のチェーンは rng_fp の列をそのまま使い、2 つ目以降はチェーンごとに別の列を使う。
        // チェーンを足しても、ほかのチェーンのランダムカタログ（と結果）が変わらないようにするため。
        let mut rng_more: Vec<SplitMix64> = (1..c_stores.len())
            .map(|i| SplitMix64::new(p.seed ^ 0xF007_9417_0000_0000 ^ ((i as u64) << 24)))
            .collect();
        for r_km in &p.footprint {
            for (i, (name, c)) in c_stores.iter().enumerate() {
                let sr = SmoothRatio::new(&c.unit, &refs, *r_km);
                let tab = Tabulated::new(&sr, &grid.geometry);
                let rng_c = if i == 0 {
                    &mut rng_fp
                } else {
                    &mut rng_more[i - 1]
                };
                let sel = sample_selected(
                    grid,
                    p.n_random,
                    Weight::Population,
                    |q| {
                        if p.selection_grid {
                            tab.eval(q)
                        } else {
                            sr.eval(&Unit::from(q))
                        }
                    },
                    2 * p.n_random,
                    rng_c,
                );
                eprintln!(
                    "{name}：選択関数 R = {r_km} km、候補 {} 点から {} 点（上限を超えた候補 {}）（{:.1} 秒）",
                    sel.candidates,
                    sel.points.len(),
                    sel.clipped,
                    t0.elapsed().as_secs_f64()
                );
                let rc = cat(&sel.points);
                counts.push((name.to_string(), format!("footprint_{r_km}"), jk(c, &rc)));
                fp_cats.push((*r_km, i, rc));
            }
        }
    }

    // 相互相関：基準が共通なら、チェーンごとに数えた DR と RR を使い回して D1D2 だけを数える。
    // 選択関数つきでは、チェーンごとの選択関数で撒いた R1、R2 を使う。
    let mut cross_counts: Vec<(String, String, JkCross)> = Vec::new();
    if let Some((ia, ib)) = cross_pair {
        let (na, ca) = &c_stores[ia];
        let (nb, cb) = &c_stores[ib];
        let label = format!("{na}x{nb}");
        let find = |n: &str, r: &str| {
            &counts
                .iter()
                .find(|(s, rr, _)| s == n && rr == r)
                .unwrap()
                .2
        };
        for (w, _) in &c_rand {
            let jc = JkCross::from_shared(
                (&ca.unit, &ca.region),
                (&cb.unit, &cb.region),
                find(na, w.name()),
                find(nb, w.name()),
                &cbins,
                chord2,
            );
            cross_counts.push((label.clone(), w.name().to_string(), jc));
        }
        for r_km in &p.footprint {
            let get = |i: usize| {
                &fp_cats
                    .iter()
                    .find(|(r, j, _)| r == r_km && *j == i)
                    .unwrap()
                    .2
            };
            let (ra, rb) = (get(ia), get(ib));
            let jc = JkCross::compute(
                (&ca.unit, &ca.region),
                (&cb.unit, &cb.region),
                (&ra.unit, &ra.region),
                (&rb.unit, &rb.region),
                k,
                &cbins,
                chord2,
            );
            cross_counts.push((label.clone(), format!("footprint_{r_km}"), jc));
        }
        eprintln!(
            "{label}：相互相関を数え終えた（{:.1} 秒）",
            t0.elapsed().as_secs_f64()
        );
    }

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
            .find(|(s, rr, _)| s == "population" && rr.as_str() == r)
            .unwrap()
            .2
    };
    let label = |r: &str| baselines.iter().find(|w| w.name() == r).unwrap().label();

    let ed = bins.edges().to_vec();
    let mid: Vec<usize> = (0..bins.len())
        .filter(|b| ed[*b] >= MID_RANGE_KM.0 && ed[b + 1] <= MID_RANGE_KM.1)
        .collect();
    let summarize_over = |label: &str, e: &Estimate, mid: &[usize]| {
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
        // GLS と並べて、重みが前もって決まる二つのまとめ方も書く（#18）。
        // 描画のスクリプトは「まとめた値」の行を読むので、こちらはその言葉を使わない。
        let range = format!(
            "{:.1}–{:.1} km の {} ビン",
            ed[mid[0]],
            ed[mid[mid.len() - 1] + 1],
            mid.len()
        );
        let (pm, ps) = plain_mean(&x, &cov);
        let (im, is) = inverse_variance_mean(&x, &cov);
        let line = format!(
            "{line}\n{label}：{range}の単純な平均 {pm:.3} ± {ps:.3}\n{label}：{range}の分散の逆数の重みの平均 {im:.3} ± {is:.3}"
        );
        eprintln!("{line}");
        line
    };
    let summarize = |label: &str, e: &Estimate| summarize_over(label, e, &mid);
    let mut summaries = Vec::new();

    for (s, r, c) in &counts {
        let r = r.as_str();
        let ls = jackknife(k, |o| c.pair_counts(o).landy_szalay());
        let nat = jackknife(k, |o| c.pair_counts(o).natural());
        let drr = jackknife(k, |o| c.pair_counts(o).dr_over_rr_minus_1());
        put("ls", s, r, &ls);
        put("natural", s, r, &nat);
        put("dr_over_rr_minus_1", s, r, &drr);

        if (r == "land" || r == "inhabited") && s != "population" {
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
        if let Some(r_km) = r
            .strip_prefix("footprint_")
            .and_then(|v| v.parse::<f64>().ok())
        {
            // 選択関数はならす幅 R より長い距離の構造を消すので、R/2 より短いビンだけを読む
            let use_bins: Vec<usize> = (0..bins.len())
                .filter(|b| ed[*b] >= CHI2_MIN_KM && ed[b + 1] <= r_km / 2.0)
                .collect();
            let x: Vec<f64> = use_bins.iter().map(|b| ls.value[*b]).collect();
            let cov: Vec<Vec<f64>> = use_bins
                .iter()
                .map(|i| use_bins.iter().map(|j| ls.cov[*i][*j]).collect())
                .collect();
            let pn = x.len();
            let hartlap = (k as f64 - pn as f64 - 2.0) / (k as f64 - 1.0);
            let line = match chi2(&x, &cov) {
                Some(c2) if hartlap > 0.0 && pn > 0 => {
                    let c2h = c2 * hartlap;
                    format!(
                        "{s} / {r}：{CHI2_MIN_KM}–{} km の {pn} ビン、χ² = {c2h:.1}（Hartlap {hartlap:.3} をかけた値）、上側確率 {:.3}",
                        r_km / 2.0,
                        chi2_sf(c2h, pn)
                    )
                }
                _ => format!("{s} / {r}：χ² を計算できない（領域 {k}、ビン {pn}）"),
            };
            eprintln!("{line}");
            writeln!(chi2_txt, "{line}").unwrap();
            let mid_r: Vec<usize> = mid
                .iter()
                .copied()
                .filter(|b| ed[b + 1] <= r_km / 2.0)
                .collect();
            if !mid_r.is_empty() {
                summaries.push(summarize_over(&format!("{s} の ξ_LS（{r}）"), &ls, &mid_r));
            }
        }
        if r == "population" {
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
    for (s, r, c) in &cross_counts {
        let r = r.as_str();
        let ls = jackknife(k, |o| c.pair_counts(o).landy_szalay());
        put("ls_cross", s, r, &ls);
        // 選択関数つきは R/2 より短いビンだけ、共通の基準は 3 km 以上の全部のビンで、0 と合うかを χ² で見る
        let r_max = r
            .strip_prefix("footprint_")
            .and_then(|v| v.parse::<f64>().ok())
            .map_or(f64::INFINITY, |r_km| r_km / 2.0);
        let use_bins: Vec<usize> = (0..bins.len())
            .filter(|b| ed[*b] >= CHI2_MIN_KM && ed[b + 1] <= r_max)
            .collect();
        let x: Vec<f64> = use_bins.iter().map(|b| ls.value[*b]).collect();
        let cov: Vec<Vec<f64>> = use_bins
            .iter()
            .map(|i| use_bins.iter().map(|j| ls.cov[*i][*j]).collect())
            .collect();
        let pn = x.len();
        let hartlap = (k as f64 - pn as f64 - 2.0) / (k as f64 - 1.0);
        let line = match chi2(&x, &cov) {
            Some(c2) if hartlap > 0.0 && pn > 0 => {
                let c2h = c2 * hartlap;
                format!(
                    "{s} / {r}：相互相関、{CHI2_MIN_KM}–{:.0} km の {pn} ビン、χ² = {c2h:.1}（Hartlap {hartlap:.3} をかけた値）、上側確率 {:.3}",
                    ed[use_bins[pn - 1] + 1],
                    chi2_sf(c2h, pn)
                )
            }
            _ => format!("{s} / {r}：相互相関の χ² を計算できない（領域 {k}、ビン {pn}）"),
        };
        eprintln!("{line}");
        writeln!(chi2_txt, "{line}").unwrap();
        let mid_r: Vec<usize> = mid.iter().copied().filter(|b| ed[b + 1] <= r_max).collect();
        if !mid_r.is_empty() {
            summaries.push(summarize_over(
                &format!("{s} の相互相関 ξ_LS（{r}）"),
                &ls,
                &mid_r,
            ));
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
