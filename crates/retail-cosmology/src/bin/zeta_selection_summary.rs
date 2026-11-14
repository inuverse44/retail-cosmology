//! Client：外部の地方区分による選択関数の偏りと実際の店の二点・三点をまとめる。
//! zeta_selection_summary <clustered stem> <SmoothRatio real.csv> <regional real.csv>
use retail_cosmology::manager::three_point::summarize_external_selection;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        3,
        "clustered の stem、旧と新の実際の ζ の CSV を渡す"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match summarize_external_selection(&root, &args[0], &args[1], &args[2]) {
        Ok(report) => print!("{report}"),
        Err(e) => panic!("{e}"),
    }
}
