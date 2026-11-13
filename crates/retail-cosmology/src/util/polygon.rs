//! 多角形の内側にある格子を塗る（走査線で塗りつぶす）。人口の格子を国の境界の中に限るのに使う。

/// 平面の多角形の輪の集まり（外周と穴を区別せず、偶奇の規則で内側を決める）の内側に中心がある格子を真にする。
/// 格子は行ごとに並び、左上の格子の左上の角が (x0, y0)、大きさは dx × dy。
pub fn cells_inside(
    rings: &[Vec<(f64, f64)>],
    width: usize,
    height: usize,
    (x0, y0): (f64, f64),
    (dx, dy): (f64, f64),
) -> Vec<bool> {
    let mut inside = vec![false; width * height];
    let mut xs = Vec::new();
    for row in 0..height {
        let y = y0 - (row as f64 + 0.5) * dy;
        xs.clear();
        for ring in rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                // 辺の下端を含み上端を含まない（頂点を二度数えない）
                if (a.1 <= y) != (b.1 <= y) {
                    xs.push(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        for pair in xs.as_chunks::<2>().0 {
            // 中心 x0 + (col + 0.5) dx が [pair[0], pair[1]) に入る列
            let c0 = ((pair[0] - x0) / dx - 0.5).ceil().max(0.0) as usize;
            let c1 = (((pair[1] - x0) / dx - 0.5).ceil().max(0.0) as usize).min(width);
            for c in c0..c1 {
                inside[row * width + c] = true;
            }
        }
    }
    inside
}

/// 点 (x, y) が輪の集まりの内側にあるか（偶奇の規則）。`xy` は輪の頂点から平面座標を取り出す。
pub fn point_inside<T>(
    rings: &[Vec<T>],
    (x, y): (f64, f64),
    xy: impl Fn(&T) -> (f64, f64),
) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (xy(&ring[i]), xy(&ring[(i + 1) % n]));
            if (a.1 <= y) != (b.1 <= y) && x < a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0) {
                inside = !inside;
            }
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_with_a_hole() {
        // 10×10 の格子（1 辺 1）に、外周 [1, 9]² と穴 [4, 6]² の多角形
        let outer = vec![(1.0, -1.0), (9.0, -1.0), (9.0, -9.0), (1.0, -9.0)];
        let hole = vec![(4.0, -4.0), (6.0, -4.0), (6.0, -6.0), (4.0, -6.0)];
        let m = cells_inside(&[outer, hole], 10, 10, (0.0, 0.0), (1.0, 1.0));
        let count = m.iter().filter(|b| **b).count();
        assert_eq!(count, 64 - 4);
        assert!(m[5 * 10 + 2] && !m[5 * 10 + 5] && !m[0]);
        let rings = [
            vec![(1.0, -1.0), (9.0, -1.0), (9.0, -9.0), (1.0, -9.0)],
            vec![(4.0, -4.0), (6.0, -4.0), (6.0, -6.0), (4.0, -6.0)],
        ];
        assert!(point_inside(&rings, (2.5, -5.5), |p| *p));
        assert!(!point_inside(&rings, (5.5, -5.5), |p| *p));
        assert!(!point_inside(&rings, (0.5, -0.5), |p| *p));
    }

    #[test]
    fn triangle_area() {
        // 直角三角形（底辺と高さ 100）の中にある格子の数は、面積 5000 に近い
        let tri = vec![(0.0, 0.0), (100.0, 0.0), (0.0, -100.0)];
        let m = cells_inside(&[tri], 100, 100, (0.0, 0.0), (1.0, 1.0));
        let count = m.iter().filter(|b| **b).count() as f64;
        assert!((count - 5000.0).abs() < 100.0, "{count}");
    }
}
