//! Client：toy model（正方形に一様に撒いた点）の 4 つの推定量を CSV で出す。
//!
//! cargo run --release --bin toy > data/toy/poisson.csv

fn main() {
    print!("{}", retail_cosmology::manager::toy::run());
}
