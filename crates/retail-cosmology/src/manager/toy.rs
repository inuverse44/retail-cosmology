//! toy model：正方形に一様に撒いた点（相関なし）について、4 つの推定量を求め、CSV の文字列で返す。

use std::fmt::Write as _;

use crate::contract::PairCounts;
use crate::engine::pair_count::Bins;
use crate::util::geo::{Xy, euclid};
use crate::util::rng::SplitMix64;

pub fn run() -> String {
    let mut rng = SplitMix64::new(1);
    let mut uniform = |n: usize| -> Vec<Xy> {
        (0..n)
            .map(|_| Xy {
                x: rng.next_f64(),
                y: rng.next_f64(),
            })
            .collect()
    };
    let data = uniform(2000);
    let randoms = uniform(8000);
    let bins = Bins::linear(0.01, 0.3, 29);
    let c = PairCounts::compute(&data, &randoms, &bins, euclid);
    let (nat, dp, ham, ls) = (
        c.natural(),
        c.davis_peebles(),
        c.hamilton(),
        c.landy_szalay(),
    );
    let mut out = String::from("r,dd,dr,rr,natural,davis_peebles,hamilton,landy_szalay\n");
    for (k, r) in bins.centers().iter().enumerate() {
        writeln!(
            out,
            "{r},{},{},{},{},{},{},{}",
            c.dd[k], c.dr[k], c.rr[k], nat[k], dp[k], ham[k], ls[k]
        )
        .unwrap();
    }
    out
}
