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
}

impl Chain {
    pub fn name(self) -> &'static str {
        match self {
            Chain::Coles => "Coles",
            Chain::Woolworths => "Woolworths",
            Chain::Trial => "TRIAL",
            Chain::Seiyu => "SEIYU",
        }
    }
}

/// OSM の要素（店舗の候補）。`state` は ISO 3166-2 のコード（AU-NSW、JP-40 など）。
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

/// ビンごとの DD, DR, RR と、データ数 `n_d`・ランダム数 `n_r`。
#[derive(Clone, Debug)]
pub struct PairCounts {
    pub n_d: usize,
    pub n_r: usize,
    pub dd: Vec<u64>,
    pub dr: Vec<u64>,
    pub rr: Vec<u64>,
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
