//! Biblioteca de Prefabs do projeto (cap. 36, revisão 2026-09-30).
//!
//! Um `Prefab` é um **snapshot reutilizável** de um ou mais objetos da cena,
//! guardado à parte de `Project::assets`. Salvar um prefab nunca adiciona
//! objetos à cena; instanciar cria cópias novas e registra o vínculo de origem
//! (`PrefabLink`) para indicar instâncias desatualizadas.
//!
//! Layout persistente: `prefabs` e `prefab_links` ficam no fim de `Project`
//! (append-only para o layout postcard legado) e usam `serde(default)`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model_library::{ModelLibraryQuery, ModelLibrarySort};
use crate::{Asset, Project};

/// Tamanho máximo do nome de um prefab.
pub const PREFAB_NAME_MAX_LEN: usize = 80;

/// Snapshot reutilizável de objetos, com geometria centrada na origem local.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prefab {
    pub id: Uuid,
    pub name: String,
    /// Partes do prefab; posições relativas ao centro dos limites do grupo.
    pub parts: Vec<Asset>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    /// Incrementa a cada re-captura; instâncias com revisão menor estão desatualizadas.
    #[serde(default)]
    pub revision: u32,
}

/// Vínculo entre um objeto da cena e o prefab de origem.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrefabLink {
    pub asset_id: Uuid,
    pub prefab_id: Uuid,
    pub revision: u32,
}

/// Resumo leve para a grade da biblioteca.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabSummary {
    pub id: Uuid,
    /// Posição na ordem de criação.
    pub original_index: usize,
    pub name: String,
    pub parts: usize,
    pub triangles: usize,
    pub vertices: usize,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub collection: Option<String>,
    /// Instâncias vivas na cena.
    pub instances: usize,
}

fn clean_name(raw: &str) -> String {
    raw.trim().chars().take(PREFAB_NAME_MAX_LEN).collect()
}

fn shift(asset: &mut Asset, delta: [f32; 3]) {
    for v in &mut asset.mesh.verts {
        v.pos[0] += delta[0];
        v.pos[1] += delta[1];
        v.pos[2] += delta[2];
    }
    if let Some(origin) = &mut asset.origin {
        origin[0] += delta[0];
        origin[1] += delta[1];
        origin[2] += delta[2];
    }
}

fn bounds_center(parts: &[Asset]) -> [f32; 3] {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for part in parts {
        for v in &part.mesh.verts {
            for axis in 0..3 {
                min[axis] = min[axis].min(v.pos[axis]);
                max[axis] = max[axis].max(v.pos[axis]);
            }
        }
    }
    if min[0] > max[0] {
        return [0.0; 3];
    }
    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

impl Project {
    fn unique_prefab_name(&self, base: &str, ignore: Option<Uuid>) -> String {
        let base = if base.is_empty() { "Prefab" } else { base };
        let taken = |name: &str| {
            self.prefabs
                .iter()
                .any(|p| Some(p.id) != ignore && p.name.eq_ignore_ascii_case(name))
        };
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|candidate| !taken(candidate))
            .unwrap_or_else(|| base.to_string())
    }

    fn capture_parts(&self, asset_ids: &[Uuid]) -> Vec<Asset> {
        let mut parts: Vec<Asset> = asset_ids
            .iter()
            .filter_map(|id| self.assets.iter().find(|a| a.id == *id))
            .cloned()
            .collect();
        let center = bounds_center(&parts);
        for part in &mut parts {
            part.id = Uuid::new_v4();
            part.visible = true;
            part.locked = false;
            shift(part, [-center[0], -center[1], -center[2]]);
        }
        parts
    }

    /// Salva os objetos como prefab da biblioteca. Não altera a cena.
    pub fn create_prefab(&mut self, asset_ids: &[Uuid], name: Option<&str>) -> Option<Uuid> {
        let parts = self.capture_parts(asset_ids);
        if parts.is_empty() {
            return None;
        }
        let requested = name.map(clean_name).filter(|n| !n.is_empty());
        let base = requested.unwrap_or_else(|| {
            if parts.len() == 1 {
                clean_name(&parts[0].name)
            } else {
                clean_name(&format!("{} +{}", parts[0].name, parts.len() - 1))
            }
        });
        let id = Uuid::new_v4();
        let name = self.unique_prefab_name(&base, None);
        self.prefabs.push(Prefab {
            id,
            name,
            parts,
            tags: Vec::new(),
            collection: None,
            favorite: false,
            revision: 0,
        });
        Some(id)
    }

    /// Re-captura o prefab a partir de objetos da cena e avança a revisão.
    pub fn update_prefab_from(&mut self, prefab_id: Uuid, asset_ids: &[Uuid]) -> bool {
        let parts = self.capture_parts(asset_ids);
        if parts.is_empty() {
            return false;
        }
        let Some(prefab) = self.prefabs.iter_mut().find(|p| p.id == prefab_id) else {
            return false;
        };
        prefab.parts = parts;
        prefab.revision = prefab.revision.saturating_add(1);
        true
    }

    /// Cria cópias do prefab na cena com o centro do grupo em `position`.
    /// Retorna os ids dos objetos criados (vazio se o prefab não existe).
    pub fn instantiate_prefab(&mut self, prefab_id: Uuid, position: [f32; 3]) -> Vec<Uuid> {
        let Some(prefab) = self.prefabs.iter().find(|p| p.id == prefab_id) else {
            return Vec::new();
        };
        let revision = prefab.revision;
        let mut created = Vec::with_capacity(prefab.parts.len());
        let copies: Vec<Asset> = prefab
            .parts
            .iter()
            .map(|part| {
                let mut copy = part.clone();
                copy.id = Uuid::new_v4();
                shift(&mut copy, position);
                copy
            })
            .collect();
        for copy in copies {
            created.push(copy.id);
            self.prefab_links.push(PrefabLink {
                asset_id: copy.id,
                prefab_id,
                revision,
            });
            self.assets.push(copy);
        }
        if !created.is_empty() {
            self.active = self.assets.len() - 1;
        }
        created
    }

    /// Remove o prefab. As instâncias ficam na cena, sem vínculo.
    pub fn remove_prefab(&mut self, prefab_id: Uuid) -> bool {
        let before = self.prefabs.len();
        self.prefabs.retain(|p| p.id != prefab_id);
        self.prefab_links.retain(|l| l.prefab_id != prefab_id);
        self.prefabs.len() != before
    }

    pub fn rename_prefab(&mut self, prefab_id: Uuid, name: &str) -> bool {
        let cleaned = clean_name(name);
        if cleaned.is_empty() {
            return false;
        }
        let unique = self.unique_prefab_name(&cleaned, Some(prefab_id));
        match self.prefabs.iter_mut().find(|p| p.id == prefab_id) {
            Some(prefab) if prefab.name != unique => {
                prefab.name = unique;
                true
            }
            _ => false,
        }
    }

    pub fn set_prefab_favorite(&mut self, prefab_id: Uuid, favorite: bool) -> bool {
        match self.prefabs.iter_mut().find(|p| p.id == prefab_id) {
            Some(prefab) if prefab.favorite != favorite => {
                prefab.favorite = favorite;
                true
            }
            _ => false,
        }
    }

    /// Substitui as tags (normalizadas: minúsculas, sem vazios/duplicadas, máx. 12).
    pub fn set_prefab_tags(&mut self, prefab_id: Uuid, tags: &[String]) -> bool {
        let mut normalized: Vec<String> = Vec::new();
        for tag in tags {
            let tag = tag.trim().to_lowercase();
            if !tag.is_empty() && tag.len() <= 32 && !normalized.contains(&tag) {
                normalized.push(tag);
            }
        }
        normalized.truncate(12);
        match self.prefabs.iter_mut().find(|p| p.id == prefab_id) {
            Some(prefab) if prefab.tags != normalized => {
                prefab.tags = normalized;
                true
            }
            _ => false,
        }
    }

    pub fn set_prefab_collection(&mut self, prefab_id: Uuid, collection: Option<&str>) -> bool {
        let collection = collection.map(clean_name).filter(|c| !c.is_empty());
        match self.prefabs.iter_mut().find(|p| p.id == prefab_id) {
            Some(prefab) if prefab.collection != collection => {
                prefab.collection = collection;
                true
            }
            _ => false,
        }
    }

    /// Instâncias do prefab ainda presentes na cena.
    pub fn prefab_instance_count(&self, prefab_id: Uuid) -> usize {
        self.prefab_links
            .iter()
            .filter(|l| l.prefab_id == prefab_id && self.assets.iter().any(|a| a.id == l.asset_id))
            .count()
    }

    /// O objeto veio de um prefab cuja revisão avançou depois da instanciação?
    pub fn prefab_instance_outdated(&self, asset_id: Uuid) -> bool {
        self.prefab_links
            .iter()
            .find(|l| l.asset_id == asset_id)
            .and_then(|l| {
                self.prefabs
                    .iter()
                    .find(|p| p.id == l.prefab_id)
                    .map(|p| p.revision > l.revision)
            })
            .unwrap_or(false)
    }

    /// Descarta vínculos de objetos removidos da cena.
    pub fn prune_prefab_links(&mut self) {
        let live: std::collections::HashSet<Uuid> = self.assets.iter().map(|a| a.id).collect();
        self.prefab_links.retain(|l| live.contains(&l.asset_id));
    }

    /// Busca/ordenação da biblioteca, com as mesmas regras de `ModelLibraryQuery`.
    pub fn prefab_summaries(&self, query: &ModelLibraryQuery) -> Vec<PrefabSummary> {
        let needle = query.search.trim().to_lowercase();
        let mut out: Vec<PrefabSummary> = self
            .prefabs
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                (needle.is_empty()
                    || p.name.to_lowercase().contains(&needle)
                    || p.tags.iter().any(|t| t.contains(&needle)))
                    && (!query.only_favorites || p.favorite)
                    && query.tag.as_ref().is_none_or(|t| p.tags.contains(t))
                    && query
                        .collection
                        .as_ref()
                        .is_none_or(|c| p.collection.as_ref() == Some(c))
            })
            .map(|(index, p)| PrefabSummary {
                id: p.id,
                original_index: index,
                name: p.name.clone(),
                parts: p.parts.len(),
                triangles: p.parts.iter().map(|a| a.mesh.tri_count()).sum(),
                vertices: p.parts.iter().map(|a| a.mesh.vert_count()).sum(),
                favorite: p.favorite,
                tags: p.tags.clone(),
                collection: p.collection.clone(),
                instances: self.prefab_instance_count(p.id),
            })
            .collect();
        match query.sort {
            ModelLibrarySort::NameAsc => {
                out.sort_by_key(|s| s.name.to_lowercase());
            }
            ModelLibrarySort::NameDesc => {
                out.sort_by_key(|s| std::cmp::Reverse(s.name.to_lowercase()));
            }
            ModelLibrarySort::TrianglesAsc => out.sort_by_key(|s| s.triangles),
            ModelLibrarySort::TrianglesDesc => out.sort_by_key(|s| std::cmp::Reverse(s.triangles)),
            ModelLibrarySort::VerticesAsc => out.sort_by_key(|s| s.vertices),
            ModelLibrarySort::VerticesDesc => out.sort_by_key(|s| std::cmp::Reverse(s.vertices)),
            ModelLibrarySort::IndexAsc => out.sort_by_key(|s| s.original_index),
            ModelLibrarySort::IndexDesc => out.sort_by_key(|s| std::cmp::Reverse(s.original_index)),
        }
        out
    }

    /// Tags dos prefabs com contagem, mais usadas primeiro.
    pub fn prefab_tags_with_counts(&self) -> Vec<(String, usize)> {
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for prefab in &self.prefabs {
            for tag in &prefab.tags {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }
        let mut list: Vec<(String, usize)> = counts.into_iter().collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::Mesh;

    fn project_with_two_cubes() -> (Project, Uuid, Uuid) {
        let mut project = Project::default();
        let mut a = Asset::new("Cube A", Mesh::cube(1.0));
        let mut b = Asset::new("Cube B", Mesh::cube(1.0));
        shift(&mut a, [4.0, 0.0, 0.0]);
        shift(&mut b, [6.0, 0.0, 0.0]);
        let (ia, ib) = (a.id, b.id);
        project.assets.push(a);
        project.assets.push(b);
        (project, ia, ib)
    }

    #[test]
    fn saving_a_prefab_does_not_touch_the_scene() {
        let (mut project, a, _) = project_with_two_cubes();
        let scene_before = project.assets.len();
        let id = project.create_prefab(&[a], None).expect("prefab");
        assert_eq!(project.assets.len(), scene_before, "cena intacta");
        assert_eq!(project.prefabs.len(), 1);
        assert_eq!(project.prefabs[0].id, id);
        assert_eq!(project.prefabs[0].name, "Cube A");
    }

    #[test]
    fn prefab_geometry_is_centered_and_instances_land_at_position() {
        let (mut project, a, b) = project_with_two_cubes();
        let id = project.create_prefab(&[a, b], Some("Pair")).unwrap();
        let center = bounds_center(&project.prefabs[0].parts);
        assert!(
            center.iter().all(|c| c.abs() < 1e-4),
            "centrado: {center:?}"
        );
        let created = project.instantiate_prefab(id, [0.0, 10.0, 0.0]);
        assert_eq!(created.len(), 2);
        let placed: Vec<Asset> = project
            .assets
            .iter()
            .filter(|asset| created.contains(&asset.id))
            .cloned()
            .collect();
        let c = bounds_center(&placed);
        assert!((c[1] - 10.0).abs() < 1e-4 && c[0].abs() < 1e-4);
        assert_eq!(project.prefab_instance_count(id), 2);
        assert_ne!(created[0], a, "instâncias recebem ids novos");
    }

    #[test]
    fn names_are_unique_and_rename_keeps_uniqueness() {
        let (mut project, a, b) = project_with_two_cubes();
        let first = project.create_prefab(&[a], Some("Chair")).unwrap();
        let second = project.create_prefab(&[b], Some("chair")).unwrap();
        assert_eq!(project.prefabs[1].name, "chair 2");
        assert!(project.rename_prefab(second, "Chair"));
        assert_eq!(project.prefabs[1].name, "Chair 2");
        assert!(!project.rename_prefab(first, "   "), "nome vazio recusado");
    }

    #[test]
    fn updating_a_prefab_marks_older_instances_outdated() {
        let (mut project, a, b) = project_with_two_cubes();
        let id = project.create_prefab(&[a], Some("Box")).unwrap();
        let instance = project.instantiate_prefab(id, [0.0; 3])[0];
        assert!(!project.prefab_instance_outdated(instance));
        assert!(project.update_prefab_from(id, &[b]));
        assert_eq!(project.prefabs[0].revision, 1);
        assert!(project.prefab_instance_outdated(instance));
    }

    #[test]
    fn removing_a_prefab_keeps_instances_without_links() {
        let (mut project, a, _) = project_with_two_cubes();
        let id = project.create_prefab(&[a], None).unwrap();
        let instance = project.instantiate_prefab(id, [1.0, 0.0, 0.0])[0];
        assert!(project.remove_prefab(id));
        assert!(project.assets.iter().any(|x| x.id == instance));
        assert!(project.prefab_links.is_empty());
        assert!(!project.remove_prefab(id));
    }

    #[test]
    fn query_filters_sorts_and_normalizes_tags() {
        let (mut project, a, b) = project_with_two_cubes();
        let one = project.create_prefab(&[a], Some("Table")).unwrap();
        let two = project.create_prefab(&[b], Some("Lamp")).unwrap();
        assert!(project.set_prefab_tags(one, &[" Wood ".into(), "wood".into(), "".into()]));
        assert_eq!(project.prefabs[0].tags, vec!["wood".to_string()]);
        assert!(project.set_prefab_favorite(two, true));
        let by_name = project.prefab_summaries(&ModelLibraryQuery::default());
        assert_eq!(by_name[0].name, "Lamp");
        let favorites = project.prefab_summaries(&ModelLibraryQuery {
            only_favorites: true,
            ..Default::default()
        });
        assert_eq!(favorites.len(), 1);
        let by_tag = project.prefab_summaries(&ModelLibraryQuery {
            search: "woo".into(),
            ..Default::default()
        });
        assert_eq!(by_tag[0].name, "Table");
        assert_eq!(project.prefab_tags_with_counts(), vec![("wood".into(), 1)]);
    }

    #[test]
    fn empty_or_unknown_selection_creates_nothing() {
        let mut project = Project::default();
        assert!(project.create_prefab(&[], None).is_none());
        assert!(project.create_prefab(&[Uuid::new_v4()], None).is_none());
        assert!(
            project
                .instantiate_prefab(Uuid::new_v4(), [0.0; 3])
                .is_empty()
        );
    }
}
