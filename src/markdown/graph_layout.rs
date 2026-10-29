//! 力導向圖佈局演算法 (Fruchterman-Reingold GraphLayout)
//!
//! 純邏輯、具決定性（Deterministic，相同輸入必定產生完全相同的幾何座標）：
//! - 初始節點均勻分佈於同心圓周
//! - 兩兩節點施加斥力 (Repulsive force)
//! - 關聯邊施加引力 (Attractive force)
//! - 施加向心引力，防止孤立筆記節點無限飄散
//! - 迭代退火降溫後，縮放正規化至指定邊距畫布內

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn compute(
    n: usize,
    edges: &[(usize, usize)],
    width: f64,
    height: f64,
    iterations: usize,
) -> Vec<Point> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Point {
            x: width / 2.0,
            y: height / 2.0,
        }];
    }

    let mut pos = Vec::with_capacity(n);
    let cx = width / 2.0;
    let cy = height / 2.0;
    let r0 = width.min(height) / 3.0;

    // 決定性初始位置：均勻圓周
    for i in 0..n {
        let a = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
        pos.push(Point {
            x: cx + r0 * a.cos(),
            y: cy + r0 * a.sin(),
        });
    }

    let area = width * height;
    let k = (area / (n as f64)).sqrt(); // 理想節點間距
    let mut t = width.min(height) / 10.0; // 溫度（最大單步位移）
    let cool = t / (iterations as f64 + 1.0);

    for _ in 0..iterations {
        let mut disp = vec![Point { x: 0.0, y: 0.0 }; n];

        // 1. 斥力：所有節點兩兩相斥
        let cutoff = 2.5 * k;
        for i in 0..n {
            for j in (i + 1)..n {
                let mut dx = pos[i].x - pos[j].x;
                let mut dy = pos[i].y - pos[j].y;
                let mut dist = (dx * dx + dy * dy).sqrt();
                if dist < 0.01 {
                    dx = 0.01 * (i as f64 - j as f64);
                    dy = 0.01;
                    dist = (dx * dx + dy * dy).sqrt();
                }
                if dist > cutoff {
                    continue;
                }
                let force = (k * k) / dist; // fr = k^2 / d
                let ux = dx / dist;
                let uy = dy / dist;
                disp[i].x += ux * force;
                disp[i].y += uy * force;
                disp[j].x -= ux * force;
                disp[j].y -= uy * force;
            }
        }

        // 2. 向心重力：將孤立節點拉回畫面中央
        for i in 0..n {
            disp[i].x += (cx - pos[i].x) * 0.10;
            disp[i].y += (cy - pos[i].y) * 0.10;
        }

        // 3. 引力：相連節點相互吸引
        for &(u, v) in edges {
            if u >= n || v >= n || u == v {
                continue;
            }
            let dx = pos[u].x - pos[v].x;
            let dy = pos[u].y - pos[v].y;
            let mut dist = (dx * dx + dy * dy).sqrt();
            if dist < 0.01 {
                dist = 0.01;
            }
            let force = (dist * dist) / k; // fa = d^2 / k
            let ux = dx / dist;
            let uy = dy / dist;
            disp[u].x -= ux * force;
            disp[u].y -= uy * force;
            disp[v].x += ux * force;
            disp[v].y += uy * force;
        }

        // 4. 套用位移並限制在畫布範圍
        for i in 0..n {
            let dl = (disp[i].x * disp[i].x + disp[i].y * disp[i].y).sqrt();
            if dl < 0.01 {
                continue;
            }
            let step = dl.min(t);
            pos[i].x += (disp[i].x / dl) * step;
            pos[i].y += (disp[i].y / dl) * step;
            pos[i].x = pos[i].x.clamp(0.0, width);
            pos[i].y = pos[i].y.clamp(0.0, height);
        }

        t = (t - cool).max(1.0);
    }

    // 5. 正規化縮放至帶有邊距的畫布
    let mut min_x = pos[0].x;
    let mut max_x = pos[0].x;
    let mut min_y = pos[0].y;
    let mut max_y = pos[0].y;

    for p in &pos {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let margin = width.min(height) * 0.1 + 20.0;
    let span_x = (max_x - min_x).max(1.0);
    let span_y = (max_y - min_y).max(1.0);
    let sx = (width - 2.0 * margin) / span_x;
    let sy = (height - 2.0 * margin) / span_y;
    let s = sx.min(sy);

    for p in &mut pos {
        p.x = margin + (p.x - min_x) * s;
        p.y = margin + (p.y - min_y) * s;
    }

    pos
}
