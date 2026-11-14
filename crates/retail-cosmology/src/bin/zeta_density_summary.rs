//! Client：clustered_mocks の一点密度の診断をまとめる。
//! zeta_density_summary <clustered stem>
use retail_cosmology::manager::three_point::summarize_mock_density;
use std::path::PathBuf;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 1, "clustered の stem（.csv の前まで）を渡す");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match summarize_mock_density(&root, &args[0]) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}
