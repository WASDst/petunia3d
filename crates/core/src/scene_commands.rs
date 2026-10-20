//! Organização da cena: grupos (collections), mover objetos por arrasto e
//! renomear grupos. Toda mudança é um comando transacional com Undo; a UI só
//! emite intenções (Ctrl+G, arrastar e soltar, renomear).

use uuid::Uuid;

use crate::command::{Command, CommandError};
use crate::context::{CommandContext, EditorView};
use petunia_project::ProjectChanges;

/// Prefixo dos nomes automáticos de grupo.
pub const GROUP_NAME_PREFIX: &str = "Grupo";

/// Primeiro nome livre "Grupo N" entre os grupos existentes.
fn unique_group_name(existing: &[String]) -> String {
    for n in 1..=10_000u32 {
        let candidate = format!("{GROUP_NAME_PREFIX} {n}");
        if !existing.iter().any(|name| name == &candidate) {
            return candidate;
        }
    }
    format!("{GROUP_NAME_PREFIX} {}", Uuid::new_v4().simple())
}

/// Ids de objetos que um comando de organização atinge: a seleção de objetos
/// e, na falta dela, o objeto ativo.
fn targeted_assets(editor: &crate::state::EditorSession, active: Option<Uuid>) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = editor.selection.assets.clone();
    if let Some(active) = active
        && !ids.contains(&active)
    {
        ids.push(active);
    }
    ids
}

/// **Agrupar** (Ctrl+G): reúne a seleção num grupo novo da Estrutura.
#[derive(Debug, Clone, Default)]
pub struct GroupSelectionCmd {
    /// Nome do grupo; `None` usa "Grupo N".
    pub name: Option<String>,
}

impl Command for GroupSelectionCmd {
    fn label(&self) -> &'static str {
        "group selection"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active().is_none() && state.selection.assets.is_empty() {
            return Err("Select the objects to group");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let active = state.document.active().map(|asset| asset.id);
        let ids = targeted_assets(state.editor, active);
        if ids.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        let name = match self.name.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => unique_group_name(&state.document.project.collections),
        };
        if !state.document.project.collections.contains(&name) {
            state.document.project.add_collection(&name);
        }
        let mut grouped = 0usize;
        let mut left_groups: Vec<String> = Vec::new();
        for asset in &mut state.document.assets {
            if ids.contains(&asset.id) {
                if let Some(previous) = asset.collection.replace(name.clone())
                    && previous != name
                    && !left_groups.contains(&previous)
                {
                    left_groups.push(previous);
                }
                grouped += 1;
            }
        }
        if grouped == 0 {
            return Err(CommandError::EmptySelection);
        }
        // Os grupos de onde os objetos saíram, se ficaram vazios, deixam de existir.
        for previous in &left_groups {
            state.document.project.prune_collection_if_empty(previous);
        }
        state.set_status(format!("{grouped} objeto(s) agrupados em {name}"));
        Ok(())
    }
}

/// **Desagrupar** (Ctrl+Shift+G): dissolve o grupo, mantendo os objetos.
#[derive(Debug, Clone, Default)]
pub struct UngroupCmd {
    /// Grupo a dissolver; `None` usa o grupo do objeto ativo.
    pub collection: Option<String>,
}

impl Command for UngroupCmd {
    fn label(&self) -> &'static str {
        "ungroup"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let has_group = self.collection.is_some()
            || state
                .document
                .active()
                .is_some_and(|asset| asset.collection.is_some());
        if has_group {
            Ok(())
        } else {
            Err("The active object is not in a group")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let name = self
            .collection
            .clone()
            .or_else(|| {
                state
                    .document
                    .active()
                    .and_then(|asset| asset.collection.clone())
            })
            .ok_or_else(|| CommandError::NoChange("o objeto ativo não está num grupo".into()))?;
        if !state.document.project.collections.contains(&name) {
            return Err(CommandError::StaleReference(name));
        }
        state.document.project.remove_collection(&name);
        state.set_status(format!("{name}: grupo desfeito"));
        Ok(())
    }
}

/// **Soltar** objetos na Estrutura: reordena (antes de `before`, ou no fim) e,
/// opcionalmente, troca o grupo dos objetos movidos. Uma única entrada de Undo.
#[derive(Debug, Clone)]
pub struct MoveAssetsCmd {
    pub ids: Vec<Uuid>,
    /// Objeto que passa a ser o vizinho de baixo; `None` leva ao fim da lista.
    pub before: Option<Uuid>,
    /// `Some(Some(g))` move para o grupo `g`; `Some(None)` tira do grupo;
    /// `None` mantém o grupo de cada objeto.
    pub collection: Option<Option<String>>,
}

impl Command for MoveAssetsCmd {
    fn label(&self) -> &'static str {
        "move assets"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if self.ids.is_empty() {
            return Err("Nothing to move");
        }
        if state.editor.isolate_active {
            return Err("Leave isolation before reordering the scene");
        }
        if let Some(Some(group)) = &self.collection
            && !state.document.project.collections.contains(group)
        {
            return Err("The target group no longer exists");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if let Some(before) = self.before
            && self.ids.contains(&before)
        {
            return Err(CommandError::NoChange(
                "soltar um objeto sobre ele mesmo".into(),
            ));
        }
        let active_id = state.document.active().map(|asset| asset.id);
        let before_order: Vec<Uuid> = state.document.assets.iter().map(|a| a.id).collect();
        let before_groups: Vec<Option<String>> = state
            .document
            .assets
            .iter()
            .map(|a| a.collection.clone())
            .collect();

        let assets = std::mem::take(&mut state.document.assets);
        let (mut moved, mut rest): (Vec<_>, Vec<_>) = assets
            .into_iter()
            .partition(|asset| self.ids.contains(&asset.id));
        if moved.is_empty() {
            state.document.assets = rest;
            return Err(CommandError::StaleReference("objetos ausentes".into()));
        }
        // Grupos de onde os objetos saem: se ficarem vazios, deixam de existir.
        let mut left_groups: Vec<String> = Vec::new();
        for asset in &moved {
            if let Some(group) = &asset.collection
                && !left_groups.contains(group)
            {
                left_groups.push(group.clone());
            }
        }
        if let Some(collection) = &self.collection {
            for asset in &mut moved {
                asset.collection.clone_from(collection);
            }
        }
        let position = self
            .before
            .and_then(|before| rest.iter().position(|asset| asset.id == before))
            .unwrap_or(rest.len());
        rest.splice(position..position, moved);
        state.document.assets = rest;
        if let Some(id) = active_id
            && let Some(index) = state.document.find(id)
        {
            state.document.active = index;
        }

        let after_order: Vec<Uuid> = state.document.assets.iter().map(|a| a.id).collect();
        let after_groups: Vec<Option<String>> = state
            .document
            .assets
            .iter()
            .map(|a| a.collection.clone())
            .collect();
        if after_order == before_order && after_groups == before_groups {
            return Err(CommandError::NoChange("a ordem já era essa".into()));
        }
        for group in &left_groups {
            state.document.project.prune_collection_if_empty(group);
        }
        state.set_status("Estrutura reorganizada".to_string());
        Ok(())
    }
}

/// Renomeia um grupo da Estrutura, mantendo os objetos nele.
#[derive(Debug, Clone)]
pub struct RenameGroupCmd {
    pub from: String,
    pub to: String,
}

impl Command for RenameGroupCmd {
    fn label(&self) -> &'static str {
        "rename group"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let to = self.to.trim();
        if to.is_empty() {
            return Err("The group name cannot be empty");
        }
        if to != self.from
            && state
                .document
                .project
                .collections
                .iter()
                .any(|name| name == to)
        {
            return Err("A group with this name already exists");
        }
        if !state
            .document
            .project
            .collections
            .iter()
            .any(|name| name == &self.from)
        {
            return Err("The group no longer exists");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let to = self.to.trim().to_string();
        if to == self.from {
            return Err(CommandError::NoChange("o nome é o mesmo".into()));
        }
        if !state.document.project.rename_collection(&self.from, &to) {
            return Err(CommandError::StaleReference(self.from.clone()));
        }
        state.set_status(format!("Grupo renomeado para {to}"));
        Ok(())
    }
}

/// Cor de seleção de um grupo da Estrutura. O objeto com cor própria vence a do
/// grupo, a menos que `force` esteja ligado (ADR-C16, item 1).
#[derive(Debug, Clone)]
pub struct SetGroupColorCmd {
    pub name: String,
    /// `None` remove a cor do grupo.
    pub color: Option<[u8; 3]>,
    /// A cor do grupo vence a cor própria de cada objeto. Ignorado sem cor.
    pub force: bool,
}

impl Command for SetGroupColorCmd {
    fn label(&self) -> &'static str {
        "set group color"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state
            .document
            .project
            .collections
            .iter()
            .any(|name| name == &self.name)
        {
            Ok(())
        } else {
            Err("The group no longer exists")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !state
            .document
            .project
            .set_collection_color(&self.name, self.color, self.force)
        {
            return if state
                .document
                .project
                .collections
                .iter()
                .any(|name| name == &self.name)
            {
                Err(CommandError::NoChange("a cor do grupo é a mesma".into()))
            } else {
                Err(CommandError::StaleReference(self.name.clone()))
            };
        }
        state.set_status(format!("Cor de seleção de {} atualizada", self.name));
        Ok(())
    }
}
