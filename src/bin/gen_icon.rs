//! Uygulama ikonunu üretir: 1024x1024 PNG. `scripts/make_app.sh` bunu .icns'e çevirir.

const N: usize = 1024;

/// Yuvarlatılmış karenin işaretli mesafesi (negatif = içeride).
fn rounded_box_sdf(x: f32, y: f32, cx: f32, cy: f32, half: f32, r: f32) -> f32 {
    let qx = (x - cx).abs() - (half - r);
    let qy = (y - cy).abs() - (half - r);
    let ax = qx.max(0.0);
    let ay = qy.max(0.0);
    (ax * ax + ay * ay).sqrt() + qx.max(qy).min(0.0) - r
}

/// Aşağı/yukarı ok: dikdörtgen gövde + üçgen kafa.
fn in_arrow(x: f32, y: f32, cx: f32, cy: f32, h: f32, up: bool) -> bool {
    let ly = if up { cy - y } else { y - cy };
    let dx = (x - cx).abs();
    let half = h / 2.0;
    let head_top = h * 0.05;

    if ly >= -half && ly < head_top {
        dx <= h * 0.115
    } else if ly >= head_top && ly <= half {
        let t = (half - ly) / (half - head_top);
        dx <= h * 0.30 * t
    } else {
        false
    }
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn main() {
    let n = N as f32;
    let inset = n * 0.085;
    let half = (n - 2.0 * inset) / 2.0;
    let (cx, cy) = (n / 2.0, n / 2.0);
    let radius = half * 2.0 * 0.2237; // macOS squircle'a yakın

    let top = [0.219, 0.522, 0.976]; // #38 85 F9
    let bot = [0.086, 0.278, 0.796]; // #16 47 CB

    let arrow_h = half * 1.16;
    let dx_off = half * 0.40;

    let mut rgba = vec![0u8; N * N * 4];

    for py in 0..N {
        for px in 0..N {
            // 3x3 alt örnekleme
            let (mut bg_cov, mut fg_cov) = (0.0f32, 0.0f32);
            for sy in 0..3 {
                for sx in 0..3 {
                    let x = px as f32 + (sx as f32 + 0.5) / 3.0;
                    let y = py as f32 + (sy as f32 + 0.5) / 3.0;

                    if rounded_box_sdf(x, y, cx, cy, half, radius) <= 0.0 {
                        bg_cov += 1.0;
                        if in_arrow(x, y, cx - dx_off, cy, arrow_h, false)
                            || in_arrow(x, y, cx + dx_off, cy, arrow_h, true)
                        {
                            fg_cov += 1.0;
                        }
                    }
                }
            }
            if bg_cov == 0.0 {
                continue;
            }
            let bg_cov = bg_cov / 9.0;
            let fg = (fg_cov / 9.0) / bg_cov.max(1e-6);

            let t = (py as f32 / n).clamp(0.0, 1.0);
            let base = lerp(top, bot, t);
            let c = lerp(base, [1.0, 1.0, 1.0], fg.clamp(0.0, 1.0));

            let i = (py * N + px) * 4;
            rgba[i] = (c[0] * 255.0) as u8;
            rgba[i + 1] = (c[1] * 255.0) as u8;
            rgba[i + 2] = (c[2] * 255.0) as u8;
            rgba[i + 3] = (bg_cov * 255.0) as u8;
        }
    }

    let path = std::env::args().nth(1).unwrap_or_else(|| "icon.png".into());
    let file = std::fs::File::create(&path).expect("dosya açılamadı");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), N as u32, N as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(&rgba).unwrap();
    println!("yazıldı: {path}");
}
