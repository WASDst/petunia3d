//! Cache da cena de consultas da viewport (picking, hover, oclusão, pintura).
//!
//! `ViewportSceneQuery::new` tria todas as malhas visíveis; refazê-la a cada
//! movimento do mouse custava O(triângulos da cena) por evento. A cena só
//! muda quando a geometria muda, então ela é reconstruída apenas quando a
//! chave de geometria muda (topologia, posições, transformações, procedural,
//! visibilidade e bloqueio dos objetos). Cores, texturas e seleção não entram
//! na chave: pintar não invalida a cena.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use petunia_core::viewport_query::ViewportSceneQuery;

use crate::{PetuniaViewport, SlintUiBridge};

/// Cena em cache e a chave de geometria com que foi construída.
pub(crate) type SceneCache = std::cell::RefCell<Option<(u64, Arc<ViewportSceneQuery>)>>;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn scene_geometry_key(&self) -> u64 {
        let project = &self.state.project.project;
        let clock = project.revision_clock();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // topology, position, transform, spline, procedural
        for index in [0usize, 1, 8, 9, 10] {
            clock[index].hash(&mut hasher);
        }
        project.assets.len().hash(&mut hasher);
        for asset in &project.assets {
            (asset.id, asset.visible, asset.locked, asset.modifiers.len()).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Cena de consultas atual, reconstruída só quando a geometria muda.
    pub(crate) fn scene_query(&self) -> Arc<ViewportSceneQuery> {
        let key = self.scene_geometry_key();
        let mut slot = self.scene_query_cache.borrow_mut();
        if let Some((cached, scene)) = slot.as_ref()
            && *cached == key
        {
            return Arc::clone(scene);
        }
        puffin::profile_scope!("scene_query_rebuild");
        let scene = Arc::new(crate::perf::measure("scene_query_rebuild", || {
            ViewportSceneQuery::new(&self.state.project.project)
        }));
        *slot = Some((key, Arc::clone(&scene)));
        scene
    }
}
