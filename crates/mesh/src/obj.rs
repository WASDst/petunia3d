//! Petunia3D mesh — import/export OBJ.

use super::{Face, Mesh, Vertex};

impl Mesh {
    // ---------------- OBJ ----------------

    pub fn to_obj(&self) -> String {
        let mut s = String::from("# petunia3d\n");
        for v in &self.verts {
            s.push_str(&format!("v {} {} {}\n", v.pos[0], v.pos[1], v.pos[2]));
        }
        // 1 vt por uso (vértice-de-face), índice sequencial
        let mut vt_index: Vec<Vec<usize>> = Vec::new();
        let mut next = 1usize;
        for f in &self.faces {
            let mut row = Vec::new();
            for q in &f.uv {
                s.push_str(&format!("vt {} {}\n", q[0], 1.0 - q[1]));
                row.push(next);
                next += 1;
            }
            vt_index.push(row);
        }
        for (f, vt) in self.faces.iter().zip(vt_index) {
            s.push('f');
            for (k, &vi) in f.verts.iter().enumerate() {
                s.push_str(&format!(" {}/{}", vi + 1, vt[k]));
            }
            s.push('\n');
        }
        s
    }

    pub fn from_obj(text: &str) -> Self {
        // Float finito ou nada (M5: NaN/inf poluiriam o export).
        fn fin(s: &str) -> Option<f32> {
            s.parse::<f32>().ok().filter(|x| x.is_finite())
        }
        let mut m = Mesh::default();
        let mut vt: Vec<[f32; 2]> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("v ") {
                let p: Vec<f32> = rest.split_whitespace().filter_map(fin).collect();
                if p.len() >= 3 {
                    m.verts.push(Vertex::new(p[0], p[1], p[2]));
                }
            } else if let Some(rest) = line.strip_prefix("vt ") {
                let p: Vec<f32> = rest.split_whitespace().filter_map(fin).collect();
                if p.len() >= 2 {
                    vt.push([p[0], 1.0 - p[1]]);
                }
            } else if let Some(rest) = line.strip_prefix("f ") {
                let mut idx = Vec::new();
                let mut uvs = Vec::new();
                for tok in rest.split_whitespace() {
                    let v: Vec<&str> = tok.split('/').collect();
                    if let Ok(i) = v[0].parse::<u32>()
                        && i > 0
                    {
                        idx.push(i - 1);
                    }
                    let uv = if v.len() > 1 {
                        v[1].parse::<usize>()
                            .ok()
                            .and_then(|t| vt.get(t.wrapping_sub(1)).copied())
                    } else {
                        None
                    };
                    uvs.push(uv.unwrap_or([0.0, 0.0]));
                }
                // M1: descarta faces com índices fora da malha em vez de panicar depois.
                if idx.len() >= 3 && idx.iter().all(|&i| (i as usize) < m.verts.len()) {
                    for k in 1..idx.len() - 1 {
                        m.push_face(Face::with_uv(
                            vec![idx[0], idx[k], idx[k + 1]],
                            vec![uvs[0], uvs[k], uvs[k + 1]],
                        ));
                    }
                }
            }
        }
        if m.faces
            .iter()
            .all(|f| f.uv.iter().all(|u| *u == [0.0, 0.0]))
        {
            m.layout_uv_charts(crate::primitives::DEFAULT_UV_PADDING);
        }
        m
    }
}
