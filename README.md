# retail-cosmology

オーストラリアの Coles と Woolworths の店舗座標に、宇宙論で銀河の分布を測るときの二点相関関数（Landy–Szalay 推定量ほか）を当てる研究。
小売に宇宙論をどこまで適用できるかを調べる。

- 発表予定：[TRIAL＆RetailAI Advent Calendar 2026](https://qiita.com/advent-calendar/2026/trial-retail-ai) の 12 月 23 日
- 理論の下書き（書きかけ）：[Landy-Szalay推定量①](https://inuverse.dev/posts/2026-04-12-retail/)、[Landy-Szalay推定量②](https://inuverse.dev/posts/2026-04-13-retail/)
- ノート：[`notes/retail-cosmology.pdf`](notes/retail-cosmology.pdf)

## 構成

```
crates/retail-cosmology/  対の数え上げと推定量（Rust）
scripts/                  描画だけ（Python, matplotlib）
notes/                    ノート（TeX）
docs/figures/             ノートに載せる図
data/                     取得したデータと計算結果（コミットしない）
```

## 動かす

```sh
cargo test
cargo run --release --bin toy > data/toy/poisson.csv   # 先に mkdir -p data/toy
uv run scripts/plot_toy.py                             # docs/figures/toy_poisson.{pdf,png}
cd notes && latexmk -lualatex retail-cosmology.tex
```

## データ

未定。取得元・取得日・ライセンスをここに書く。
