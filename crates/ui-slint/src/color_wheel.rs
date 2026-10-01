//! Geração da imagem da roda de cores HSV para o color picker de seleção.

/// Converte HSV (Hue 0..360, Saturation 0..1, Value 0..1) em RGB [u8; 3].
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let h_prime = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Amostra a cor da roda dada uma posição relativa ao centro `(dx, dy)` e raio máximo.
pub fn sample_wheel_color(dx: f32, dy: f32, radius: f32) -> [u8; 3] {
    let dist = (dx * dx + dy * dy).sqrt();
    let sat = (dist / radius.max(1.0)).clamp(0.0, 1.0);
    let angle_deg = dy.atan2(dx).to_degrees().rem_euclid(360.0);
    hsv_to_rgb(angle_deg, sat, 1.0)
}

/// Gera uma `slint::Image` circular com a roda de cores HSV antisserrilhada.
pub fn generate_color_wheel_image(size: u32) -> slint::Image {
    let center = size as f32 / 2.0;
    let radius = center - 1.5;
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size, size);
    let bytes = buf.make_mut_bytes();

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;

            if dist <= radius {
                let sat = (dist / radius).min(1.0);
                let angle_deg = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let rgb = hsv_to_rgb(angle_deg, sat, 1.0);
                bytes[idx] = rgb[0];
                bytes[idx + 1] = rgb[1];
                bytes[idx + 2] = rgb[2];
                bytes[idx + 3] = 255;
            } else if dist <= radius + 1.2 {
                // Borda suavizada
                let alpha = (((radius + 1.2 - dist) / 1.2).clamp(0.0, 1.0) * 255.0) as u8;
                let angle_deg = dy.atan2(dx).to_degrees().rem_euclid(360.0);
                let rgb = hsv_to_rgb(angle_deg, 1.0, 1.0);
                bytes[idx] = rgb[0];
                bytes[idx + 1] = rgb[1];
                bytes[idx + 2] = rgb[2];
                bytes[idx + 3] = alpha;
            } else {
                bytes[idx] = 0;
                bytes[idx + 1] = 0;
                bytes[idx + 2] = 0;
                bytes[idx + 3] = 0;
            }
        }
    }
    slint::Image::from_rgba8(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_to_rgb_primary_colors() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), [0, 0, 255]);
        assert_eq!(hsv_to_rgb(0.0, 0.0, 1.0), [255, 255, 255]);
    }

    #[test]
    fn sample_wheel_center_is_white() {
        let center = sample_wheel_color(0.0, 0.0, 50.0);
        assert_eq!(center, [255, 255, 255]);
    }

    #[test]
    fn color_wheel_image_has_correct_dimensions() {
        let img = generate_color_wheel_image(120);
        assert_eq!(img.size().width, 120);
        assert_eq!(img.size().height, 120);
    }
}
