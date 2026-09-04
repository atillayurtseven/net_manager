//! Tepsi ikonu: aşağı/yukarı ok çifti, kod içinde 4x4 süperörnekleme ile çizilir.

use tray_icon::Icon;

const S: usize = 32;

/// Üçgen kafalı, dikdörtgen gövdeli ok. `up` yönü belirler.
fn in_arrow(x: f32, y: f32, cx: f32, up: bool) -> bool {
    // Yerel koordinat: oku her zaman aşağı bakar gibi hesapla, gerekirse çevir.
    let ly = if up { 31.0 - y } else { y };
    let dx = (x - cx).abs();

    let stem = ly >= 4.0 && ly < 17.0 && dx <= 2.5;
    let head = ly >= 17.0 && ly <= 28.0 && dx <= 7.5 * (28.0 - ly) / 11.0;
    stem || head
}

pub fn build() -> Icon {
    let mut rgba = vec![0u8; S * S * 4];

    for py in 0..S {
        for px in 0..S {
            // 4x4 alt örnekleme -> yumuşak kenar
            let mut hits = 0u32;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = px as f32 + (sx as f32 + 0.5) / 4.0;
                    let y = py as f32 + (sy as f32 + 0.5) / 4.0;
                    if in_arrow(x, y, 8.5, false) || in_arrow(x, y, 23.5, true) {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                let a = (hits * 255 / 16) as u8;
                let i = (py * S + px) * 4;
                // Siyah + alfa: macOS template modunda menü çubuğu rengine uyar.
                rgba[i + 3] = a;
            }
        }
    }

    Icon::from_rgba(rgba, S as u32, S as u32).expect("ikon oluşturulamadı")
}
