//! Presentation-only HSV picker geometry and rasterization.
//!
//! The hue ring preserves saturation/value; the inner triangle uses pure hue,
//! white and black vertices. Both hit testing and pixels use the same geometry.

pub const PART_COLOR_PICKER_SIZE: u32 = 130;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickerHsv {
    pub hue: f32,
    pub saturation: f32,
    pub value: f32,
}

impl PickerHsv {
    pub fn from_rgb(rgb: [u8; 3], previous: Self) -> Self {
        let [r, g, b] = rgb.map(|channel| channel as f32 / 255.0);
        let maximum = r.max(g).max(b);
        let minimum = r.min(g).min(b);
        let chroma = maximum - minimum;
        let hue = if chroma <= f32::EPSILON {
            // Hue remains useful when the user later leaves white/gray/black.
            previous.hue
        } else if maximum == r {
            60.0 * ((g - b) / chroma).rem_euclid(6.0)
        } else if maximum == g {
            60.0 * ((b - r) / chroma + 2.0)
        } else {
            60.0 * ((r - g) / chroma + 4.0)
        };
        Self {
            hue,
            saturation: if maximum <= f32::EPSILON {
                previous.saturation
            } else {
                chroma / maximum
            },
            value: maximum,
        }
    }

    pub fn rgb(self) -> [u8; 3] {
        hsv_to_rgb(self.hue, self.saturation, self.value)
    }
}

/// Converts HSV (hue degrees, saturation/value 0..1) to display RGB bytes.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerRegion {
    HueRing,
    SaturationValue,
}

struct PickerGeometry {
    center: f32,
    outer_radius: f32,
    inner_radius: f32,
    triangle_radius: f32,
    triangle_half_height: f32,
}

impl PickerGeometry {
    fn new(size: u32) -> Self {
        let center = size as f32 / 2.0;
        let outer_radius = (center - 1.5).max(0.0);
        let inner_radius = outer_radius * 0.81;
        let triangle_radius = inner_radius * 0.90;
        Self {
            center,
            outer_radius,
            inner_radius,
            triangle_radius,
            triangle_half_height: triangle_radius * 3.0_f32.sqrt() / 2.0,
        }
    }

    // Barycentric weights in hue / white / black vertex order.
    fn weights(&self, x: f32, y: f32) -> [f32; 3] {
        let dx = x - self.center;
        let dy = y - self.center;
        let hue = (dx + self.triangle_radius / 2.0) / (1.5 * self.triangle_radius);
        let white = (1.0 - hue - dy / self.triangle_half_height) / 2.0;
        [hue, white, 1.0 - hue - white]
    }

    fn region(&self, x: f32, y: f32) -> Option<PickerRegion> {
        let distance = (x - self.center).hypot(y - self.center);
        if distance >= self.inner_radius && distance <= self.outer_radius {
            Some(PickerRegion::HueRing)
        } else if self.weights(x, y).iter().all(|weight| *weight >= -0.00001) {
            Some(PickerRegion::SaturationValue)
        } else {
            None
        }
    }

    fn triangle_marker(&self, hsv: PickerHsv) -> (f32, f32) {
        let hue = hsv.saturation * hsv.value;
        let white = (1.0 - hsv.saturation) * hsv.value;
        let black = 1.0 - hsv.value;
        (
            self.center + self.triangle_radius * (hue - (white + black) / 2.0),
            self.center + self.triangle_half_height * (black - white),
        )
    }
}

/// Select a drag region only on pointer-down, so a hue drag cannot change S/V.
pub fn picker_region_at(x: f32, y: f32, size: u32) -> Option<PickerRegion> {
    if size < 4 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    PickerGeometry::new(size).region(x, y)
}

/// Sample a latched drag region. Out-of-triangle movement clamps to its edges.
pub fn sample_picker_color(
    x: f32,
    y: f32,
    size: u32,
    previous: PickerHsv,
    region: PickerRegion,
) -> Option<PickerHsv> {
    if size < 4 || !x.is_finite() || !y.is_finite() {
        return None;
    }
    let geometry = PickerGeometry::new(size);
    match region {
        PickerRegion::HueRing => Some(PickerHsv {
            hue: (y - geometry.center)
                .atan2(x - geometry.center)
                .to_degrees()
                .rem_euclid(360.0),
            ..previous
        }),
        PickerRegion::SaturationValue => {
            let weights = geometry.weights(x, y).map(|weight| weight.max(0.0));
            let total = weights.iter().sum::<f32>();
            if !total.is_finite() || total <= f32::EPSILON {
                return None;
            }
            let value = ((weights[0] + weights[1]) / total).clamp(0.0, 1.0);
            Some(PickerHsv {
                saturation: if value <= f32::EPSILON {
                    previous.saturation
                } else {
                    (weights[0] / (weights[0] + weights[1])).clamp(0.0, 1.0)
                },
                value,
                ..previous
            })
        }
    }
}

/// Rasterize the ring/triangle and contrast-safe selection markers on demand into RGBA8 bytes.
pub fn generate_color_wheel_image(size: u32, hsv: PickerHsv) -> Vec<u8> {
    let geometry = PickerGeometry::new(size);
    let hue_rgb = hsv_to_rgb(hsv.hue, 1.0, 1.0);
    let mut bytes = vec![0u8; (size * size * 4) as usize];
    if size < 4 {
        return bytes;
    }

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - geometry.center;
            let dy = py - geometry.center;
            let distance = dx.hypot(dy);
            let ring_coverage = (geometry.outer_radius - distance + 0.5)
                .min(distance - geometry.inner_radius + 0.5)
                .clamp(0.0, 1.0);
            let weights = geometry.weights(px, py);
            let triangle_coverage = (weights.iter().copied().fold(f32::INFINITY, f32::min)
                * geometry.triangle_radius
                * 1.5
                + 0.5)
                .clamp(0.0, 1.0);
            let pixel = &mut bytes[((y * size + x) * 4) as usize..][..4];
            if ring_coverage > 0.0 {
                let rgb = hsv_to_rgb(dy.atan2(dx).to_degrees().rem_euclid(360.0), 1.0, 1.0);
                pixel[..3].copy_from_slice(&rgb);
                pixel[3] = (ring_coverage * 255.0).round() as u8;
            } else if triangle_coverage > 0.0 {
                for channel in 0..3 {
                    pixel[channel] = (weights[0] * hue_rgb[channel] as f32 + weights[1] * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
                pixel[3] = (triangle_coverage * 255.0).round() as u8;
            }
        }
    }
    let angle = hsv.hue.to_radians();
    let marker_radius = (geometry.outer_radius + geometry.inner_radius) / 2.0;
    draw_marker(
        &mut bytes,
        size,
        geometry.center + angle.cos() * marker_radius,
        geometry.center + angle.sin() * marker_radius,
    );
    let (x, y) = geometry.triangle_marker(hsv);
    draw_marker(&mut bytes, size, x, y);
    bytes
}

fn draw_marker(bytes: &mut [u8], size: u32, center_x: f32, center_y: f32) {
    // A dark outer stroke and white inner stroke remain visible on every hue.
    for y in (center_y - 5.0).max(0.0) as u32..=(center_y + 5.0).min(size as f32 - 1.0) as u32 {
        for x in (center_x - 5.0).max(0.0) as u32..=(center_x + 5.0).min(size as f32 - 1.0) as u32 {
            let distance = (x as f32 + 0.5 - center_x).hypot(y as f32 + 0.5 - center_y);
            let color = if (3.0..=4.5).contains(&distance) {
                Some(24)
            } else if (1.8..3.0).contains(&distance) {
                Some(255)
            } else {
                None
            };
            if let Some(color) = color {
                let pixel = &mut bytes[((y * size + x) * 4) as usize..][..4];
                pixel.copy_from_slice(&[color, color, color, 255]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HSV: PickerHsv = PickerHsv {
        hue: 30.0,
        saturation: 0.7,
        value: 0.4,
    };

    #[test]
    fn hsv_to_rgb_primary_colors() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), [0, 0, 255]);
        assert_eq!(hsv_to_rgb(0.0, 0.0, 1.0), [255, 255, 255]);
    }

    #[test]
    fn rgb_round_trip_and_achromatic_hue_are_preserved() {
        for rgb in [
            [249, 79, 44],
            [10, 90, 170],
            [3, 2, 1],
            [128, 128, 128],
            [0, 0, 0],
        ] {
            assert_eq!(PickerHsv::from_rgb(rgb, HSV).rgb(), rgb);
        }
        assert_eq!(PickerHsv::from_rgb([255, 255, 255], HSV).hue, HSV.hue);
        assert_eq!(
            PickerHsv::from_rgb([0, 0, 0], HSV).saturation,
            HSV.saturation
        );
    }

    #[test]
    fn hue_ring_preserves_dark_color_and_triangle_reaches_black_and_white() {
        let geometry = PickerGeometry::new(PART_COLOR_PICKER_SIZE);
        let ring = sample_picker_color(
            geometry.center,
            5.0,
            PART_COLOR_PICKER_SIZE,
            HSV,
            PickerRegion::HueRing,
        )
        .expect("finite ring point");
        assert_eq!(ring.saturation, HSV.saturation);
        assert_eq!(ring.value, HSV.value);
        assert!((ring.hue - 270.0).abs() < 0.001);
        for (saturation, value) in [(1.0, 1.0), (0.0, 1.0), (0.7, 0.0)] {
            let expected = PickerHsv {
                saturation,
                value,
                ..HSV
            };
            let (x, y) = geometry.triangle_marker(expected);
            let sampled = sample_picker_color(
                x,
                y,
                PART_COLOR_PICKER_SIZE,
                HSV,
                PickerRegion::SaturationValue,
            )
            .expect("finite triangle point");
            assert!((sampled.saturation - saturation).abs() < 0.001);
            assert!((sampled.value - value).abs() < 0.001);
            assert_eq!(sampled.rgb(), expected.rgb());
        }
        assert_eq!(picker_region_at(0.0, 0.0, PART_COLOR_PICKER_SIZE), None);
        assert_eq!(
            picker_region_at(f32::NAN, 0.0, PART_COLOR_PICKER_SIZE),
            None
        );
    }

    #[test]
    fn triangle_marker_round_trip_covers_saturation_and_value() {
        let geometry = PickerGeometry::new(PART_COLOR_PICKER_SIZE);
        for saturation in [0.0, 0.2, 0.7, 1.0] {
            for value in [0.1, 0.3, 0.8, 1.0] {
                let expected = PickerHsv {
                    saturation,
                    value,
                    ..HSV
                };
                let (x, y) = geometry.triangle_marker(expected);
                let sampled = sample_picker_color(
                    x,
                    y,
                    PART_COLOR_PICKER_SIZE,
                    HSV,
                    PickerRegion::SaturationValue,
                )
                .expect("finite triangle point");
                assert!((sampled.saturation - saturation).abs() < 0.00001);
                assert!((sampled.value - value).abs() < 0.00001);
            }
        }
    }

    #[test]
    fn color_wheel_image_has_correct_dimensions() {
        let img = generate_color_wheel_image(PART_COLOR_PICKER_SIZE, HSV);
        assert_eq!(
            img.len(),
            (PART_COLOR_PICKER_SIZE * PART_COLOR_PICKER_SIZE * 4) as usize
        );
    }
}
