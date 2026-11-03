//! toy model：正方形に一様に撒いた点（相関なし）について、4 つの推定量を CSV に書く。
//!
//! cargo run --release --bin toy > data/toy/poisson.csv

use retail_cosmology::{Bins, PairCounts, Xy, euclid, rng::SplitMix64};

fn main() {
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
    println!("r,dd,dr,rr,natural,davis_peebles,hamilton,landy_szalay");
    for (k, r) in bins.centers().iter().enumerate() {
        println!(
            "{r},{},{},{},{},{},{},{}",
            c.dd[k], c.dr[k], c.rr[k], nat[k], dp[k], ham[k], ls[k]
        );
    }
}
