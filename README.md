# retail-cosmology

宇宙論で使う推定量を、オーストラリアの小売データにどこまで当てはめられるかを調べるリポジトリ。

- 動機：[Trial Retail AI Advent Calendar 2026](https://qiita.com/advent-calendar/2026/trial-retail-ai)
- 理論の下書き（書きかけ）：
  - https://inuverse.dev/posts/2026-04-12-retail/
  - https://inuverse.dev/posts/2026-04-13-retail/

## 問い

小売のデータに宇宙論の推定量を当てはめたとき、何がそのまま使え、何が使えないか。
使えないなら、どの前提（一様性・等方性・定常性など）が崩れているせいか。

## 構成

```
src/retail_cosmology/  推定量とデータ読み込みのコード
notebooks/             試行錯誤
data/raw/              取得したデータ（コミットしない）
figures/               図
docs/                  メモ・理論
tests/                 テスト（toy model で推定量を検算する）
```

## セットアップ

```sh
uv sync
uv run pytest
```

## データ

未定。取得元・取得日・ライセンスをここに書く。
