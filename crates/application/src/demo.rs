//! Cena de demonstração para capturas e diagnóstico.

use petunia_module_paint::PaintModule;

use crate::Application;

impl Application {
    /// Cena demo: vaso (revolve) + cápsula + referência procedural +
    /// canvas com círculo + textura ligada.
    pub fn build_demo_scene(&mut self) {
        use petunia_mesh::Mesh;
        let s = &mut self.state;
        if let Ok(mut vase) = Mesh::revolve(
            &[[0.0, 0.0], [0.8, 0.1], [1.0, 1.0], [0.6, 2.0], [0.0, 2.2]],
            14,
        ) {
            for v in &mut vase.verts {
                v.pos[0] -= 3.2;
            }
            s.project.add("Vase", vase);
        }
        let mut cap = Mesh::capsule(10, 0.5, 2.2);
        for v in &mut cap.verts {
            v.pos[0] += 3.2;
        }
        s.project.add("Capsule", cap);
        // referência procedural (gradiente + mira) — exercita o pipeline
        let (w, h) = (256u32, 256u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = (x as f32 / w as f32 * 255.0) as u8;
                rgba[i + 1] = (y as f32 / h as f32 * 255.0) as u8;
                rgba[i + 2] = 128;
                rgba[i + 3] = 255;
            }
        }
        for k in 0..w {
            let i = ((128 * w + k) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
        }
        for k in 0..h {
            let i = ((k * w + 128) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 0;
            rgba[i + 2] = 0;
        }
        let mut r = petunia_core::ReferenceImage::from_rgba("demo-ref".into(), w, h, rgba);
        r.axis = petunia_core::RefAxis::Front;
        s.project.refs.push(r);
        // canvas com círculo no CUBO (asset 0, central) + preview texturizado
        s.project.active = 0;
        PaintModule::ensure_canvas(s);
        if let Some(o) = s.project.active_mut()
            && let Some(cv) = o.texture.as_mut()
        {
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let dx = x as i32 - 128;
                    let dy = y as i32 - 128;
                    if dx * dx + dy * dy < 60 * 60 {
                        cv.set(x, y, [220, 40, 40, 255]);
                    }
                }
            }
        }
        s.textured = true;
        s.render.canvas_dirty = true;
        s.sync_selection();
        s.mark_dirty();
    }
}
