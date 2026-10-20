//! Seleção UV da Application (V01; website/docs/07-uv/architecture-decision.md,
//! "UV Selection", "Sync Selection", "Ownership").
//!
//! A representação canônica é um conjunto de FaceCorners. Os modos Corner, Edge,
//! Face e Island decidem apenas como um clique se expande; UV vertex não é um
//! conceito separado (é um canto ou um conjunto de cantos coincidentes).
//!
//! Sync Selection é uma opção explícita: ligada, a seleção UV acompanha a de
//! faces 3D e vice-versa; desligada, as duas são independentes.

use std::collections::{BTreeSet, HashSet};

use glam::Vec2;
use petunia_mesh::{FaceCornerIndex, Mesh};

/// Como um clique no UV Editor escolhe cantos.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum UvSelectionMode {
    /// Um vértice UV (o canto mais próximo e os coincidentes).
    Corner,
    /// Uma UV edge (dois cantos consecutivos de uma face e os coincidentes).
    Edge,
    /// Todos os cantos da face sob o cursor.
    #[default]
    Face,
    /// Todos os cantos da ilha UV derivada sob o cursor.
    Island,
}

/// Seleção UV da sessão (não é persistida nem entra no Undo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UvSelectionState {
    pub mode: UvSelectionMode,
    /// Sync Selection explícito entre UV e faces 3D.
    pub sync: bool,
    corners: BTreeSet<FaceCornerIndex>,
    /// Faces 3D da última sincronização 3D → UV: o Sync reage a **mudanças**
    /// da seleção 3D, não a toda divergência (seleções de canto/edge ou
    /// definidas pelo próprio UV Editor sobrevivem a outros comandos).
    last_synced_faces: Vec<usize>,
}

impl Default for UvSelectionState {
    /// Sync começa ligado: preserva o fluxo anterior (clique no UV seleciona a
    /// face 3D), agora como opção que o usuário pode desligar.
    fn default() -> Self {
        Self {
            mode: UvSelectionMode::default(),
            sync: true,
            corners: BTreeSet::new(),
            last_synced_faces: Vec::new(),
        }
    }
}

impl UvSelectionState {
    pub fn is_empty(&self) -> bool {
        self.corners.is_empty()
    }

    pub fn clear(&mut self) {
        self.corners.clear();
    }

    /// Cantos selecionados, em ordem determinística.
    pub fn corners(&self) -> impl Iterator<Item = FaceCornerIndex> + '_ {
        self.corners.iter().copied()
    }

    pub fn corner_set(&self) -> HashSet<FaceCornerIndex> {
        self.corners.iter().copied().collect()
    }

    pub fn contains_corner(&self, corner: FaceCornerIndex) -> bool {
        self.corners.contains(&corner)
    }

    /// Faces com **todos** os cantos selecionados — o filtro que operações por
    /// face (Stitch, Relax, Pack, Pin) recebem.
    pub fn faces(&self, mesh: &Mesh) -> HashSet<usize> {
        let touched: BTreeSet<usize> = self.corners.iter().map(|c| c.face).collect();
        touched
            .into_iter()
            .filter(|&fi| {
                mesh.faces.get(fi).is_some_and(|f| {
                    !f.uv.is_empty()
                        && (0..f.uv.len())
                            .all(|c| self.contains_corner(FaceCornerIndex::new(fi, c)))
                })
            })
            .collect()
    }

    /// Seleciona faces inteiras (substitui a seleção).
    pub fn set_faces(&mut self, mesh: &Mesh, faces: impl IntoIterator<Item = usize>) {
        self.corners.clear();
        for fi in faces {
            self.corners.extend(face_corners(mesh, fi));
        }
    }

    /// Clique no UV Editor em `point` (UV, origem embaixo). `radius` em unidades
    /// UV vale para Corner/Edge. Sem `extend` substitui; com `extend` alterna o
    /// alvo (se todo ele já estava selecionado, sai; senão, entra). Retorna
    /// `false` quando nada foi atingido (e, sem `extend`, limpa a seleção).
    pub fn click(&mut self, mesh: &Mesh, point: Vec2, radius: f32, extend: bool) -> bool {
        let target = self.hit(mesh, point, radius);
        if target.is_empty() {
            if !extend {
                self.corners.clear();
            }
            return false;
        }
        if extend {
            if target.iter().all(|c| self.corners.contains(c)) {
                for c in &target {
                    self.corners.remove(c);
                }
            } else {
                self.corners.extend(target);
            }
        } else {
            self.corners = target;
        }
        true
    }

    /// O alvo sob `point` já está inteiro na seleção (pressionar nele para
    /// arrastar não deve trocar a seleção por um item só).
    pub fn hits_selection(&self, mesh: &Mesh, point: Vec2, radius: f32) -> bool {
        let target = self.hit(mesh, point, radius);
        !target.is_empty() && target.iter().all(|corner| self.corners.contains(corner))
    }

    /// Seleção por caixa em UV (`min`/`max`, origem embaixo). O que a caixa
    /// atinge depende do modo: cantos dentro, arestas com os dois extremos
    /// dentro, faces com o centro dentro ou ilhas que a caixa toca. Sem
    /// `extend`/`subtract` substitui; caixa vazia sem eles limpa. Retorna se a
    /// seleção mudou.
    pub fn box_select(
        &mut self,
        mesh: &Mesh,
        min: Vec2,
        max: Vec2,
        extend: bool,
        subtract: bool,
    ) -> bool {
        if !min.is_finite() || !max.is_finite() {
            return false;
        }
        let inside =
            |uv: &[f32; 2]| uv[0] >= min.x && uv[0] <= max.x && uv[1] >= min.y && uv[1] <= max.y;
        let centroid_inside = |fi: usize| {
            let uv = &mesh.faces[fi].uv;
            if uv.is_empty() {
                return false;
            }
            let n = uv.len() as f32;
            let c = [
                uv.iter().map(|p| p[0]).sum::<f32>() / n,
                uv.iter().map(|p| p[1]).sum::<f32>() / n,
            ];
            inside(&c)
        };
        let mut target: BTreeSet<FaceCornerIndex> = BTreeSet::new();
        match self.mode {
            UvSelectionMode::Corner => {
                for (fi, face) in mesh.faces.iter().enumerate() {
                    for (ci, uv) in face.uv.iter().enumerate() {
                        if inside(uv) {
                            target.insert(FaceCornerIndex::new(fi, ci));
                        }
                    }
                }
            }
            UvSelectionMode::Edge => {
                for (fi, face) in mesh.faces.iter().enumerate() {
                    let n = face.uv.len();
                    for ci in 0..n {
                        let next = (ci + 1) % n;
                        if inside(&face.uv[ci]) && inside(&face.uv[next]) {
                            target.insert(FaceCornerIndex::new(fi, ci));
                            target.insert(FaceCornerIndex::new(fi, next));
                        }
                    }
                }
            }
            UvSelectionMode::Face => {
                for fi in 0..mesh.faces.len() {
                    if centroid_inside(fi) {
                        target.extend(face_corners(mesh, fi));
                    }
                }
            }
            UvSelectionMode::Island => {
                for island in mesh.uv_islands() {
                    if island.faces.iter().any(|&fi| centroid_inside(fi)) {
                        for fi in island.faces {
                            target.extend(face_corners(mesh, fi));
                        }
                    }
                }
            }
        }
        let before = self.corners.clone();
        if subtract {
            for corner in &target {
                self.corners.remove(corner);
            }
        } else if extend {
            self.corners.extend(target);
        } else {
            self.corners = target;
        }
        self.corners != before
    }

    /// Cantos que um clique em `point` atinge no modo atual.
    pub fn hit(&self, mesh: &Mesh, point: Vec2, radius: f32) -> BTreeSet<FaceCornerIndex> {
        let mut out = BTreeSet::new();
        if !point.is_finite() || !radius.is_finite() {
            return out;
        }
        match self.mode {
            UvSelectionMode::Corner => {
                if let Some(corner) = mesh.uv_corner_near(point, radius) {
                    out.extend(mesh.uv_coincident_corners(corner));
                }
            }
            UvSelectionMode::Edge => {
                if let Some(edge) = mesh.uv_edge_near(point, radius) {
                    for corner in [edge.corner_a, edge.corner_b] {
                        out.extend(
                            mesh.uv_coincident_corners(FaceCornerIndex::new(edge.face, corner)),
                        );
                    }
                }
            }
            UvSelectionMode::Face => {
                if let Some(fi) = mesh.uv_face_at(point) {
                    out.extend(face_corners(mesh, fi));
                }
            }
            UvSelectionMode::Island => {
                if let Some(fi) = mesh.uv_face_at(point)
                    && let Some(island) = mesh
                        .uv_islands()
                        .into_iter()
                        .find(|i| i.faces.contains(&fi))
                {
                    for f in island.faces {
                        out.extend(face_corners(mesh, f));
                    }
                }
            }
        }
        out
    }

    /// Remove cantos que deixaram de existir (malha trocada ou editada).
    pub fn retain_valid(&mut self, mesh: &Mesh) {
        self.corners.retain(|c| {
            mesh.faces
                .get(c.face)
                .is_some_and(|f| c.corner < f.uv.len())
        });
    }

    /// Sync 3D → UV: com `sync` ligado, a seleção UV passa a ser exatamente as
    /// faces 3D selecionadas. Desligado — ou quando as faces 3D já são as faces
    /// totalmente selecionadas em UV — não faz nada, preservando seleções de
    /// canto/edge vindas do próprio UV Editor. Retorna se mudou.
    pub fn sync_from_faces(&mut self, mesh: &Mesh, faces: &[usize]) -> bool {
        if !self.sync {
            return false;
        }
        let mut sorted = faces.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted == self.last_synced_faces {
            return false;
        }
        self.last_synced_faces = sorted;
        let wanted: HashSet<usize> = faces.iter().copied().collect();
        if wanted == self.faces(mesh) {
            return false;
        }
        self.set_faces(mesh, wanted);
        true
    }

    /// Sync UV → 3D: com `sync` ligado, as faces 3D selecionadas passam a ser
    /// as faces totalmente selecionadas em UV. Retorna se alguma face mudou.
    pub fn sync_to_faces(&mut self, mesh: &mut Mesh) -> bool {
        if !self.sync {
            return false;
        }
        let selected = self.faces(mesh);
        let mut changed = false;
        for (fi, face) in mesh.faces.iter_mut().enumerate() {
            let want = selected.contains(&fi);
            changed |= face.selected != want;
            face.selected = want;
        }
        // O 3D agora espelha o UV: a volta 3D → UV não deve reaplicar nada.
        let mut synced: Vec<usize> = selected.into_iter().collect();
        synced.sort_unstable();
        self.last_synced_faces = synced;
        changed
    }
}

impl crate::context::CommandContext<'_> {
    /// Seleciona faces inteiras em UV (lista de ilhas na Estrutura): sem
    /// `extend` substitui; com `extend` acrescenta. Índices fora da malha são
    /// ignorados; retorna `false` quando nenhuma face válida foi dada.
    pub fn uv_select_faces(&mut self, faces: &[usize], extend: bool) -> bool {
        let project = &mut self.document.project;
        let Some(asset) = project.assets.get_mut(project.active) else {
            return false;
        };
        let valid: Vec<usize> = faces
            .iter()
            .copied()
            .filter(|&fi| asset.mesh.faces.get(fi).is_some_and(|f| !f.uv.is_empty()))
            .collect();
        if valid.is_empty() {
            return false;
        }
        let selection = &mut self.editor.tools.uv_selection;
        if extend {
            let mut all: Vec<usize> = selection.faces(&asset.mesh).into_iter().collect();
            all.extend(valid);
            selection.set_faces(&asset.mesh, all);
        } else {
            selection.set_faces(&asset.mesh, valid);
        }
        if selection.sync_to_faces(&mut asset.mesh) {
            self.sync_selection();
        }
        true
    }

    /// Clique no UV Editor (Application): aplica o modo de seleção UV e, com
    /// Sync ligado, leva o resultado às faces 3D. Retorna se algo foi atingido.
    pub fn uv_click(&mut self, point: Vec2, radius: f32, extend: bool) -> bool {
        let project = &mut self.document.project;
        let Some(asset) = project.assets.get_mut(project.active) else {
            return false;
        };
        let selection = &mut self.editor.tools.uv_selection;
        let hit = selection.click(&asset.mesh, point, radius, extend);
        if selection.sync_to_faces(&mut asset.mesh) {
            self.sync_selection();
        }
        hit
    }

    /// Pressionar no UV Editor: se o alvo já está na seleção (e não é Shift),
    /// **mantém** a seleção para que o arrasto mova todos; se não há alvo, não
    /// mexe na seleção (o soltar decide); senão age como o clique. Retorna se
    /// algum alvo foi atingido.
    pub fn uv_press(&mut self, point: Vec2, radius: f32, extend: bool) -> bool {
        let project = &mut self.document.project;
        let Some(asset) = project.assets.get_mut(project.active) else {
            return false;
        };
        let selection = &self.editor.tools.uv_selection;
        if selection.hit(&asset.mesh, point, radius).is_empty() {
            // Espaço vazio: não limpa agora. Soltar sem arrastar limpa (clique);
            // arrastar vira uma caixa de seleção.
            return false;
        }
        if !extend && selection.hits_selection(&asset.mesh, point, radius) {
            return true;
        }
        self.uv_click(point, radius, extend)
    }

    /// Seleção por caixa no UV Editor (Application), com Sync para o 3D.
    pub fn uv_box_select(&mut self, min: Vec2, max: Vec2, extend: bool, subtract: bool) -> bool {
        let project = &mut self.document.project;
        let Some(asset) = project.assets.get_mut(project.active) else {
            return false;
        };
        let selection = &mut self.editor.tools.uv_selection;
        let changed = selection.box_select(&asset.mesh, min, max, extend, subtract);
        if selection.sync_to_faces(&mut asset.mesh) {
            self.sync_selection();
        }
        changed
    }

    /// Liga/desliga o Sync Selection. Ao ligar, a seleção UV adota as faces 3D.
    pub fn set_uv_sync(&mut self, enabled: bool) {
        let selection = &mut self.editor.tools.uv_selection;
        selection.sync = enabled;
        if enabled
            && let Some(asset) = self
                .document
                .project
                .assets
                .get(self.document.project.active)
        {
            selection.sync_from_faces(&asset.mesh, &self.editor.selection.faces);
        }
    }
}

fn face_corners(mesh: &Mesh, fi: usize) -> impl Iterator<Item = FaceCornerIndex> {
    let n = mesh.faces.get(fi).map_or(0, |f| f.uv.len());
    (0..n).map(move |c| FaceCornerIndex::new(fi, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dois quads vizinhos com UV contínua na edge compartilhada.
    fn strip() -> Mesh {
        let mut a = Mesh::plane(2.0);
        let mut b = Mesh::plane(2.0);
        a.faces[0].uv = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        b.faces[0].uv = vec![[1.0, 0.0], [1.0, 1.0], [2.0, 1.0], [2.0, 0.0]];
        b.translate_selected([2.0, 0.0, 0.0]);
        a.join(&b);
        let _remap = a.weld_with_remap(1e-4);
        a
    }

    #[test]
    fn face_mode_selects_every_corner_and_faces_round_trip() {
        let m = strip();
        let mut sel = UvSelectionState::default();
        assert!(sel.click(&m, Vec2::new(0.5, 0.5), 0.01, false));
        assert_eq!(sel.faces(&m), HashSet::from([0]));
        assert!(sel.click(&m, Vec2::new(1.5, 0.5), 0.01, true));
        assert_eq!(sel.faces(&m), HashSet::from([0, 1]));
        // Shift no mesmo alvo alterna para fora.
        sel.click(&m, Vec2::new(1.5, 0.5), 0.01, true);
        assert_eq!(sel.faces(&m), HashSet::from([0]));
        // Clique no vazio sem Shift limpa.
        assert!(!sel.click(&m, Vec2::new(5.0, 5.0), 0.01, false));
        assert!(sel.is_empty());
    }

    #[test]
    fn corner_mode_selects_the_uv_vertex_across_faces() {
        let m = strip();
        let mut sel = UvSelectionState {
            mode: UvSelectionMode::Corner,
            ..UvSelectionState::default()
        };
        // Canto compartilhado (u = 1, v = 0) existe nas duas faces.
        sel.click(&m, Vec2::new(1.0, 0.0), 0.05, false);
        let faces: BTreeSet<usize> = sel.corners().map(|c| c.face).collect();
        assert_eq!(faces, BTreeSet::from([0, 1]));
        assert!(sel.faces(&m).is_empty());
    }

    #[test]
    fn island_mode_takes_the_whole_island() {
        let m = strip();
        let mut sel = UvSelectionState {
            mode: UvSelectionMode::Island,
            ..UvSelectionState::default()
        };
        sel.click(&m, Vec2::new(0.5, 0.5), 0.01, false);
        assert_eq!(sel.faces(&m), HashSet::from([0, 1]));
    }

    #[test]
    fn sync_is_explicit_in_both_directions() {
        let mut m = strip();
        let mut sel = UvSelectionState {
            sync: false,
            ..UvSelectionState::default()
        };
        assert!(!sel.sync_from_faces(&m, &[1]));
        assert!(sel.is_empty());

        sel.sync = true;
        assert!(sel.sync_from_faces(&m, &[1]));
        assert_eq!(sel.faces(&m), HashSet::from([1]));
        sel.set_faces(&m, [0]);
        assert!(sel.sync_to_faces(&mut m));
        assert!(m.faces[0].selected && !m.faces[1].selected);
    }

    #[test]
    fn unchanged_3d_selection_never_resets_a_uv_only_selection() {
        let m = strip();
        let mut sel = UvSelectionState::default();
        sel.set_faces(&m, [0]);
        // Outros comandos sincronizam com a mesma seleção 3D (vazia): nada muda.
        assert!(!sel.sync_from_faces(&m, &[]));
        assert!(!sel.sync_from_faces(&m, &[]));
        assert_eq!(sel.faces(&m), HashSet::from([0]));
        // Uma mudança real da seleção 3D é refletida.
        assert!(sel.sync_from_faces(&m, &[1]));
        assert_eq!(sel.faces(&m), HashSet::from([1]));
    }

    /// Dois quads separados no espaço UV: `a` em [0, 0.4]² e `b` em [0.6, 1]×[0, 0.4].
    fn two_islands() -> Mesh {
        let mut a = Mesh::plane(2.0);
        let mut b = Mesh::plane(2.0);
        a.faces[0].uv = vec![[0.0, 0.0], [0.0, 0.4], [0.4, 0.4], [0.4, 0.0]];
        b.faces[0].uv = vec![[0.6, 0.0], [0.6, 0.4], [1.0, 0.4], [1.0, 0.0]];
        b.translate_selected([3.0, 0.0, 0.0]);
        a.join(&b);
        a
    }

    #[test]
    fn a_target_inside_the_selection_is_a_hit_so_dragging_keeps_every_island() {
        let m = two_islands();
        let mut sel = UvSelectionState::default();
        sel.set_faces(&m, [0, 1]);
        assert!(sel.hits_selection(&m, Vec2::new(0.2, 0.2), 0.01));
        assert!(sel.hits_selection(&m, Vec2::new(0.8, 0.2), 0.01));
        // Só a face 1 selecionada: apontar a face 0 não conta como acerto.
        sel.set_faces(&m, [1]);
        assert!(!sel.hits_selection(&m, Vec2::new(0.2, 0.2), 0.01));
        // Espaço vazio nunca é acerto.
        assert!(!sel.hits_selection(&m, Vec2::new(0.5, 0.9), 0.01));
    }

    #[test]
    fn box_select_in_face_mode_replaces_extends_and_subtracts() {
        let m = two_islands();
        let mut sel = UvSelectionState::default();
        let a = (Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.5));
        let b = (Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.5));
        assert!(sel.box_select(&m, a.0, a.1, false, false));
        assert_eq!(sel.faces(&m), HashSet::from([0]));
        assert!(sel.box_select(&m, b.0, b.1, true, false), "Shift soma");
        assert_eq!(sel.faces(&m), HashSet::from([0, 1]));
        assert!(sel.box_select(&m, a.0, a.1, false, true), "Ctrl subtrai");
        assert_eq!(sel.faces(&m), HashSet::from([1]));
        // Caixa vazia sem Shift/Ctrl limpa; repetir não muda nada.
        let empty = (Vec2::new(0.45, 0.8), Vec2::new(0.55, 0.9));
        assert!(sel.box_select(&m, empty.0, empty.1, false, false));
        assert!(sel.is_empty());
        assert!(!sel.box_select(&m, empty.0, empty.1, false, false));
    }

    #[test]
    fn box_select_follows_the_selection_mode() {
        let m = two_islands();
        // Canto: só os cantos dentro da caixa.
        let mut corner = UvSelectionState {
            mode: UvSelectionMode::Corner,
            ..UvSelectionState::default()
        };
        corner.box_select(&m, Vec2::new(-0.1, -0.1), Vec2::new(0.1, 0.1), false, false);
        assert_eq!(corner.corners().count(), 1);
        // Aresta: precisa dos dois extremos dentro (a de baixo da face 0).
        let mut edge = UvSelectionState {
            mode: UvSelectionMode::Edge,
            ..UvSelectionState::default()
        };
        edge.box_select(&m, Vec2::new(-0.1, -0.1), Vec2::new(0.5, 0.1), false, false);
        assert_eq!(edge.corners().count(), 2);
        // Ilha: tocar o centro de uma face leva a ilha inteira (a face 1 não entra).
        let mut island = UvSelectionState {
            mode: UvSelectionMode::Island,
            ..UvSelectionState::default()
        };
        island.box_select(&m, Vec2::new(0.1, 0.1), Vec2::new(0.3, 0.3), false, false);
        assert_eq!(island.faces(&m), HashSet::from([0]));
    }

    #[test]
    fn box_select_ignores_non_finite_boxes() {
        let m = two_islands();
        let mut sel = UvSelectionState::default();
        assert!(!sel.box_select(&m, Vec2::new(f32::NAN, 0.0), Vec2::ONE, false, false));
        assert!(sel.is_empty());
    }

    #[test]
    fn retain_valid_drops_corners_of_removed_faces() {
        let mut m = strip();
        let mut sel = UvSelectionState::default();
        sel.set_faces(&m, [0, 1]);
        m.faces.truncate(1);
        sel.retain_valid(&m);
        assert_eq!(sel.faces(&m), HashSet::from([0]));
    }
}
