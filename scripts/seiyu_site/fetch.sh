#!/usr/bin/env bash
# 西友の公式サイトの店舗情報（店名、ロゴの種類、住所、緯度・経度）を取ってくる。
# 使い方：scripts/seiyu_site/fetch.sh
#
# 店舗検索（https://www.seiyu.co.jp/searchshop/）の都道府県ごとのページに、店ごとの
# data-logotype・data-title・data-latlng と住所が載っている。トライアル西友の店は
# /searchtrialshop/<都道府県>/ に別に載っているので、そちらも取る。
# トライアル西友の一覧には東京のページしかなく、ほかの県のトライアル西友（浦安、二俣川など）が載っていない。
# そこでサイトマップ（wp-sitemap-posts-shop-1.xml）の店のページと一覧を突き合わせ、一覧にない店のページも取る。
# 店のページの地図の data-ll は、一覧の data-latlng と同じ値である（西友青井店で確かめた）。
# サイトの「このサイトについて」（https://www.seiyu.co.jp/terms/）により内容の著作権は西友にあり、
# 無断の転用・転載はできないので、取ったデータはリポジトリにコミットしない（data/ に置く）。
# 間隔をあけて少ない回数で取る。
#
# 出力：data/seiyu_site/<取得日>/searchshop_<都道府県>.html、searchtrialshop_<都道府県>.html、
#       sitemap_shop.xml、shop_<番号>.html（一覧にない店のページ）
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
day="$(date +%Y-%m-%d)"
out="$root/data/seiyu_site/$day"
mkdir -p "$out"
ua="retail-cosmology research script"
base="https://www.seiyu.co.jp"

curl -sSL -A "$ua" "$base/searchshop/" -o "$out/searchshop_index.html"
prefs="$(grep -oE 'href="\./[a-z]+/"' "$out/searchshop_index.html" | sed -E 's|.*\./([a-z]+)/"|\1|' | sort -u)"
trial="$(grep -oE 'href=" ?/searchtrialshop/[a-z]+/"' "$out/searchshop_index.html" | grep -oE '/[a-z]+/"$' | tr -d '/"' | sort -u)"
echo "都道府県 $(echo "$prefs" | wc -l | tr -d ' ') 個、トライアル西友 $(echo "$trial" | wc -l | tr -d ' ') 個"
for p in $prefs; do
  sleep 1
  curl -sSL -A "$ua" "$base/searchshop/$p/" -o "$out/searchshop_$p.html"
done
for p in $trial; do
  sleep 1
  curl -sSL -A "$ua" "$base/searchtrialshop/$p/" -o "$out/searchtrialshop_$p.html"
done
n="$(cat "$out"/searchshop_*.html "$out"/searchtrialshop_*.html | grep -c 'data-latlng=' || true)"
echo "一覧の店 $n 件"

# 一覧にない店：サイトマップの店のページの名前と、一覧の data-link を比べる。
# 一覧の名前には【改装中】などの前置きがつき、英字の大小が違うこともあるので、前置きを除き小文字にして比べる。
sleep 1
curl -sSL -A "$ua" "$base/wp-sitemap-posts-shop-1.xml" -o "$out/sitemap_shop.xml"
norm() { sed -E 's/【[^】]*】//g' | tr '[:upper:]' '[:lower:]'; }
cat "$out"/searchshop_*.html "$out"/searchtrialshop_*.html | grep -oE "data-link='/shop/[^']*'" \
  | sed -E "s#data-link='/shop/(.*)/'#\\1#" | norm | sort -u > "$out/listed.txt"
i=0
grep -oE '<loc>[^<]*' "$out/sitemap_shop.xml" | sed 's/<loc>//' | while read -r url; do
  name="$(python3 -I -c 'import sys, urllib.parse; print(urllib.parse.unquote(sys.argv[1]).rstrip("/").split("/shop/")[1])' "$url" | norm)"
  if ! grep -qxF "$name" "$out/listed.txt"; then
    i=$((i + 1))
    sleep 1
    curl -sSL -A "$ua" "$url" -o "$out/shop_$i.html"
    echo "一覧にない店：$name"
  fi
done
