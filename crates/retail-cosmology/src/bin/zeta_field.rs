//! Client：ζ = 0 の模擬データ（ガウス場で店の密度を揺らす模型）を日本の実際の地形の上で作り、
//! 店と同じ手順で三点相関関数を測る（#81）。
//!
//! cargo run --release --bin zeta_field -- 2026-11-12 --country jp --stores パス --chain COSMOS
//!     --footprint 50 --amp 0.5 --sigma 15 --centers 20000 --runs 3 [--n-random 8000] [--seed 1] [--sides 2:32:4]
//!
//! --amp A：δ の標準偏差。--sigma σ：δ を作るガウス関数の幅 [km]。--centers：ガウス関数の中心の数。
//! --amp2 A2 --sigma2 σ2：独立な符号を持つ二つ目の成分（両方渡す）。切る前の分散は A²+A2²。
//! --start k：run k から始める。同じ seed の別の run 範囲は同じ池と基準に条件づけて独立。
//! 二つ目の成分も --start も省略したときは、PR #90 の連続した乱数列を使う。
//! --tag NAME：分割計算では各ファイルに別の名前を付ける。既存出力は上書きしない。
//! --split on：店を半分ずつ A・B に分け、B の ξ・ζ を三通りで測る列を足す（#94）。
//!   half_true（正しい基準）、half_proc（B 自身から選択関数）、half_split（A から選択関数）。
//!   分け方は別の乱数列で決めるので、もとの列（true・proc）は --split なしと同じ。
//! --counts on：ζ の分子（DDD − 3DDR + 3DRR − RRR）と分母（RRR）を、組の数で割った形で
//!   true・proc それぞれ書く列（num_true,rrr_true,num_proc,rrr_proc）を足す（#94）。もとの列は変わらない。
//! --s-widths 15,25,…：W は --footprint の幅のまま、店と同じ手順の S の幅だけを変えて同じ模擬データを測る。
//!   幅と run ごとに別の乱数列を使うので、もとの列は変わらない。<stem>_swidth.csv と _swidth_xi.csv に書く（#94）。
//! --transform none|chi|negchi：単位分散の場 g にかける点ごとの変換（ADR 010）。δ = A T(g)。
//!   none は T(g) = g（既定、出力は従来と同じ）。chi は (g² − 1)/√2（ζ > 0）、negchi は (1 − g²)/√2（ζ < 0）。
//!   none 以外ではファイル名に _t<名前> を足し、expected の列に切る前の ζ の見込み ±2√2 A³ ρ₁₂ρ₂₃ρ₃₁ を書く。
//! --save-points on：1 回ごとの店の位置を <stem>_points.csv（run,lat,lon）に書く（ADR 011）。乱数列は変わらない。
//! --skip-measure on：ξ̂・ζ̂ の数え上げを飛ばし、<stem>_points.csv だけを書く（--save-points on が要る）。
//!   同じ引数・同じ種なら、店の位置は数え上げる実行と同じ。既に測った模擬データの位置を短い時間で作るのに使う。
//! --inhibit r:γ：店どうしが近づかない条件（ADR 012）。重みで受け入れた候補を、すでに選んだ店のうち r [km] 以内に
//!   あるものの数 k について確率 γ^k で受け入れる（γ = 0 なら r 以内に置かない）。ファイル名に _inh<r>x<γ> を足す。
//! --pop-power α：重みに (2 次メッシュの人口)^{α−1} をかける（ADR 012）。α < 1 で人口の多いマスの店が人口の比より少なくなる。
//!   1 以外ではファイル名に _pp<α> を足す。どちらも --counts とは一緒に使わない。expected の列は場だけの見込みのまま。
//! --cover R_c:β：近くに店のない所を先に埋める条件（ADR 014）。重みで受け入れた候補（--inhibit の後）を、すでに選んだ店が
//!   R_c [km] 以内に一つでもあれば確率 β で受け入れる（0 < β ≤ 1）。ファイル名に _cov<R_c>x<β> を足す。--counts とは一緒に使わない。
//! --pop-response R_p:P_0:m:P_1:s：店のまわりの人口に対する反応（ADR 015）。重みに h(P) = [1 + (P_0/P)^m]^{−1} [1 + (P/P_1)^s]^{−1}
//!   をかける（P は池の点から R_p [km] 以内の人口の格子の人口の和）。ファイル名に _pr<R_p>x<P_0>x<m>x<P_1>x<s> を足す。--counts とは一緒に使わない。
//! --catchment r_max:P_*:ν:γ：まわりの人口が少ない所ほど店どうしを離す条件（ADR 016、--pop-response が要る）。池の点の距離を
//!   r(P) = r_max min(1, (P_*/P)^ν) [km] とし、重みで受け入れた候補を、すでに選んだ店のうち r(P) 以内の数 k について確率 γ^k で
//!   受け入れる。ファイル名に _cat<r_max>x<P_*>x<ν>x<γ> を足す。
//! 書くもの：data/zeta/field_<jp_|us_><日付>_<チェーン>_a<A>_s<σ>[_t<変換>][_inh..][_pp..][_cov..][_pr..][_cat..][_名前].csv と _xi.csv
//! まとめは zeta_clustered_summary で作る（形は同じ）。

use std::path::PathBuf;

use retail_cosmology::manager::three_point::{FieldModel, FieldTransform, Params, field_mocks};

fn main() {
    let mut it = std::env::args().skip(1);
    let mut p = Params::new(it.next().expect("日付（YYYY-MM-DD）を渡す"));
    let (mut amp, mut sigma, mut centers, mut runs) = (0.5f64, 15.0f64, 20_000usize, 3usize);
    let (mut amp2, mut sigma2, mut start) = (None, None, None);
    let (mut split, mut counts) = (false, false);
    let (mut save_points, mut skip_measure) = (false, false);
    let mut transform = FieldTransform::Identity;
    let (mut inhibit, mut pop_power, mut cover) = (None, 1.0f64, None);
    let (mut pop_response, mut catchment) = (None, None);
    while let Some(k) = it.next() {
        let v = it.next().expect("引数の値がない");
        match k.as_str() {
            "--country" => p.country = v,
            "--stores" => p.stores = v,
            "--chain" => p.chain = v,
            "--n-random" => p.n_random = v.parse().unwrap(),
            "--n-ref" => p.n_ref = v.parse().unwrap(),
            "--seed" => p.seed = v.parse().unwrap(),
            "--footprint" => p.footprint = Some(v.parse().unwrap()),
            "--amp" => amp = v.parse().unwrap(),
            "--sigma" => sigma = v.parse().unwrap(),
            "--amp2" => amp2 = Some(v.parse().unwrap()),
            "--sigma2" => sigma2 = Some(v.parse().unwrap()),
            "--start" => start = Some(v.parse().unwrap()),
            "--centers" => centers = v.parse().unwrap(),
            "--runs" => runs = v.parse().unwrap(),
            "--sides" => {
                let s: Vec<&str> = v.split(':').collect();
                assert_eq!(s.len(), 3, "--sides lo:hi:n");
                p.sides_km = (
                    s[0].parse().unwrap(),
                    s[1].parse().unwrap(),
                    s[2].parse().unwrap(),
                );
            }
            "--tag" => p.tag = format!("_{v}"),
            "--s-widths" => p.s_widths = v.split(',').map(|s| s.parse().unwrap()).collect(),
            "--split" => {
                assert!(v == "on" || v == "off", "--split は on か off");
                split = v == "on";
            }
            "--counts" => {
                assert!(v == "on" || v == "off", "--counts は on か off");
                counts = v == "on";
            }
            "--save-points" => {
                assert!(v == "on" || v == "off", "--save-points は on か off");
                save_points = v == "on";
            }
            "--skip-measure" => {
                assert!(v == "on" || v == "off", "--skip-measure は on か off");
                skip_measure = v == "on";
            }
            "--inhibit" => {
                let s: Vec<&str> = v.split(':').collect();
                assert_eq!(s.len(), 2, "--inhibit r:γ");
                inhibit = Some((s[0].parse().unwrap(), s[1].parse().unwrap()));
            }
            "--pop-power" => pop_power = v.parse().unwrap(),
            "--cover" => {
                let s: Vec<&str> = v.split(':').collect();
                assert_eq!(s.len(), 2, "--cover R_c:β");
                cover = Some((s[0].parse().unwrap(), s[1].parse().unwrap()));
            }
            "--pop-response" => {
                let s: Vec<f64> = v.split(':').map(|x| x.parse().unwrap()).collect();
                assert_eq!(s.len(), 5, "--pop-response R_p:P_0:m:P_1:s");
                pop_response = Some([s[0], s[1], s[2], s[3], s[4]]);
            }
            "--catchment" => {
                let s: Vec<f64> = v.split(':').map(|x| x.parse().unwrap()).collect();
                assert_eq!(s.len(), 4, "--catchment r_max:P_*:ν:γ");
                catchment = Some([s[0], s[1], s[2], s[3]]);
            }
            "--transform" => {
                transform = [
                    FieldTransform::Identity,
                    FieldTransform::Chi,
                    FieldTransform::NegChi,
                ]
                .into_iter()
                .find(|t| t.name() == v)
                .unwrap_or_else(|| panic!("--transform は none・chi・negchi のどれか：{v}"));
            }
            _ => panic!("知らない引数：{k}"),
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert_eq!(
        amp2.is_some(),
        sigma2.is_some(),
        "--amp2 と --sigma2 は両方渡す"
    );
    let model = FieldModel {
        amp,
        sigma_km: sigma,
        second: amp2.zip(sigma2),
        n_centers: centers,
        runs,
        start,
        split,
        counts,
        transform,
        save_points,
        skip_measure,
        inhibit,
        pop_power,
        cover,
        pop_response,
        catchment,
    };
    match field_mocks(&root, &p, &model) {
        Ok(path) => eprintln!("{path} を書いた"),
        Err(e) => panic!("{e}"),
    }
}
