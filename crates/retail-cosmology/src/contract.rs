//! 層の間で受け渡すデータの形（The Method のデータ契約、ADR 001）。
//! どの層もこの型を使ってよい。ここには計算の規則もファイルの読み書きも置かない。

use crate::util::geo::LatLon;
use crate::util::grid::GridGeometry;

/// チェーン。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Chain {
    Coles,
    Woolworths,
    Trial,
    Seiyu,
    Cosmos,
    Walmart,
}

impl Chain {
    pub fn name(self) -> &'static str {
        match self {
            Chain::Coles => "Coles",
            Chain::Woolworths => "Woolworths",
            Chain::Trial => "TRIAL",
            Chain::Seiyu => "SEIYU",
            Chain::Cosmos => "COSMOS",
            Chain::Walmart => "WALMART",
        }
    }
}

/// OSM の要素（店舗の候補）。`state` は ISO 3166-2 のコード（AU-NSW、JP-40、US-TX など）。
#[derive(Clone, Debug)]
pub struct OsmElement {
    pub kind: String,
    pub id: u64,
    pub pos: LatLon,
    pub state: String,
    pub shop: String,
    pub brand: String,
    pub wikidata: String,
    pub name: String,
}

/// 公式サイトの店（トライアル、西友）。
/// トライアルは `category` が業態、`prefecture` が都道府県名で、`address` は空。
/// 西友は `category` がロゴの種類（seiyu、livin、trial）、`address` が住所で、`prefecture` は空。
#[derive(Clone, Debug)]
pub struct SiteStore {
    pub id: String,
    pub name: String,
    pub category: String,
    pub pos: LatLon,
    pub prefecture: String,
    pub address: String,
}

/// 座標のない公式サイトの店（コスモス）。住所から座標を推定する前の形。
/// `area` はサイトの地区（店のページの URL の …/shop/<地区>/<県>/ の地区、kyushu など）。
/// `planned` は「オープン予定」の店（まだ開いていない）。
#[derive(Clone, Debug, PartialEq)]
pub struct ListedStore {
    pub id: String,
    pub area: String,
    pub name: String,
    pub address: String,
    pub planned: bool,
}

/// 住所から推定した座標。`matched` は住所検索が当てた住所の文字列（どの細かさで当たったかがわかる）。
#[derive(Clone, Debug, PartialEq)]
pub struct Geocoded {
    pub id: String,
    pub pos: LatLon,
    pub matched: String,
}

/// 選び出した店。OSM の店では `kind` と `id` が OSM の要素、公式サイトの店では `kind` が site、`id` が 0。
#[derive(Clone, Debug)]
pub struct Store {
    pub chain: Chain,
    pub region: String,
    pub state: String,
    pub pos: LatLon,
    pub kind: String,
    pub id: u64,
    pub brand: String,
    pub name: String,
}

/// 距離のビン。`edges` は狭義単調増加で、ビン k は `[edges[k], edges[k+1])`。
/// 二点と三点の数え上げ、分散の見積もりなど、いくつもの Engine が使うのでここに置く（ADR 001、ADR 007）。
#[derive(Clone, Debug)]
pub struct Bins {
    edges: Vec<f64>,
}

impl Bins {
    pub fn new(edges: Vec<f64>) -> Self {
        assert!(edges.len() >= 2, "ビンの端は 2 つ以上");
        assert!(
            edges.windows(2).all(|w| w[0] < w[1]),
            "ビンの端は狭義単調増加"
        );
        Self { edges }
    }

    /// `[lo, hi)` を `n` 等分する。
    pub fn linear(lo: f64, hi: f64, n: usize) -> Self {
        Self::new(
            (0..=n)
                .map(|k| lo + (hi - lo) * k as f64 / n as f64)
                .collect(),
        )
    }

    /// `[lo, hi)` を対数で `n` 等分する。
    pub fn log(lo: f64, hi: f64, n: usize) -> Self {
        let (a, b) = (lo.ln(), hi.ln());
        Self::new(
            (0..=n)
                .map(|k| (a + (b - a) * k as f64 / n as f64).exp())
                .collect(),
        )
    }

    /// 端を単調増加の関数 `f` で写したビン。距離の代わりに、その単調な関数で数えるときに使う。
    pub fn map(&self, f: impl Fn(f64) -> f64) -> Self {
        Self::new(self.edges.iter().map(|e| f(*e)).collect())
    }

    pub fn len(&self) -> usize {
        self.edges.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn edges(&self) -> &[f64] {
        &self.edges
    }

    /// ビンの中点。
    pub fn centers(&self) -> Vec<f64> {
        self.edges.windows(2).map(|w| 0.5 * (w[0] + w[1])).collect()
    }

    /// 距離 `r` が入るビンの番号。範囲外なら `None`。
    pub fn index(&self, r: f64) -> Option<usize> {
        if !(r >= self.edges[0] && r < self.edges[self.edges.len() - 1]) {
            return None;
        }
        // edges[k] <= r となる最大の k
        Some(self.edges.partition_point(|&e| e <= r) - 1)
    }
}

/// ビンごとの DD, DR, RR と、データ数 `n_d`・ランダム数 `n_r`。
#[derive(Clone, Debug)]
pub struct PairCounts {
    pub n_d: usize,
    pub n_r: usize,
    pub dd: Vec<u64>,
    pub dr: Vec<u64>,
    pub rr: Vec<u64>,
}

/// 三角形のビンごとの DDD, DDR, DRR, RRR と、データ数 `n_d`・ランダム数 `n_r`（ADR 007）。
/// DDR はデータの二点とランダムの一点、DRR はデータの一点とランダムの二点の三つ組の数。
#[derive(Clone, Debug)]
pub struct TripleCounts {
    pub n_d: usize,
    pub n_r: usize,
    pub ddd: Vec<u64>,
    pub ddr: Vec<u64>,
    pub drr: Vec<u64>,
    pub rrr: Vec<u64>,
}

/// 二種類の点（1 と 2）の相互相関のための、ビンごとの対の数。
/// ランダムカタログは種類ごとに R1、R2 を持つ。`n_r1r2` は R1R2 の対の総数で、
/// R1 と R2 が別のカタログなら n_r1 n_r2、同じカタログなら n_r(n_r - 1)/2（`r1r2` はそのカタログの中の対の数）。
#[derive(Clone, Debug)]
pub struct CrossPairCounts {
    pub n_d1: usize,
    pub n_d2: usize,
    pub n_r1: usize,
    pub n_r2: usize,
    pub n_r1r2: f64,
    pub d1d2: Vec<u64>,
    pub d1r2: Vec<u64>,
    pub d2r1: Vec<u64>,
    pub r1r2: Vec<u64>,
}

/// xi が書いた χ² の一行（data/xi/chi2_….txt）。
#[derive(Clone, Debug, PartialEq)]
pub struct Chi2Record {
    /// 標本（例：TRIAL、TRIALxSEIYU）
    pub sample: String,
    /// 基準（例：population、footprint_50）
    pub random: String,
    /// 使ったビン（例：「3 km 以上の 14 ビン」）
    pub bins: String,
    /// Hartlap の補正をかけた χ²
    pub chi2: f64,
    /// χ² 分布から求めた上側確率
    pub p: f64,
}

/// xi が書いたビンごとの値の一行（data/xi/xi_….csv）か、模擬データのビンごとの値の一行
/// （scripts/null_mocks_cross.sh が書いた …_bins.csv）。
#[derive(Clone, Debug, PartialEq)]
pub struct BinValue {
    /// 模擬データの種（実際の店の値では None）
    pub seed: Option<u64>,
    /// 量（例：ls、ls_cross）
    pub quantity: String,
    /// 標本（例：TRIAL、TRIALxSEIYU）
    pub sample: String,
    /// 基準（例：footprint_50）。模擬データのファイルには列がないので空
    pub random: String,
    pub r_lo_km: f64,
    pub r_hi_km: f64,
    pub value: f64,
}

/// 決算資料にある地域ごとの店舗数。
#[derive(Clone, Debug)]
pub struct OfficialCount {
    pub chain: String,
    pub region: String,
    pub count: usize,
}

/// 人口の格子。`values` は格子ごとの人口で、陸地の外は負。
/// `has_land` は、人口 0 の陸地の格子を含むか（ABS の格子は含み、e-Stat のメッシュは含まない）。
pub struct PopulationGrid {
    pub values: Vec<f32>,
    pub geometry: GridGeometry,
    pub has_land: bool,
}

impl PopulationGrid {
    /// 陸地（値が 0 以上）の格子の数と、人口の合計。
    pub fn land_and_total(&self) -> (usize, f64) {
        let land = self.values.iter().filter(|v| **v >= 0.0).count();
        let total = self
            .values
            .iter()
            .filter(|v| **v > 0.0)
            .map(|v| *v as f64)
            .sum();
        (land, total)
    }
}

/// 州の境界。`state` は ISO 3166-2 のコード（US-TX など）、`rings` は多角形の輪（経度・緯度）。
/// 外周と穴は区別せず、偶奇の規則で内側を決める。
#[derive(Clone, Debug)]
pub struct StateBoundary {
    pub state: String,
    pub rings: Vec<Vec<LatLon>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bins_index_is_half_open() {
        let b = Bins::linear(0.0, 1.0, 4);
        assert_eq!(b.index(0.0), Some(0));
        assert_eq!(b.index(0.25), Some(1));
        assert_eq!(b.index(0.999), Some(3));
        assert_eq!(b.index(1.0), None);
        assert_eq!(b.index(-0.1), None);
        assert_eq!(b.index(f64::NAN), None);
    }
}
