//! Petunia3D — Undo/Redo via Command Pattern (snapshots).
//!
//! Destructive operations are undoable commands. Snapshots of the affected
//! document are acceptable for V1. History is bounded by a **byte budget**
//! (default 256 MiB), discarding oldest entries first — not a fixed op count.

use std::collections::{HashSet, VecDeque};

/// Pegada de memória de um snapshot: chama `visit(id, bytes)` para cada bloco.
/// Blocos com `id != 0` são compartilháveis (o mesmo `id` em vários snapshots
/// conta uma vez); `id == 0` é memória própria do snapshot.
pub type HistoryFootprint<T> = fn(&T, &mut dyn FnMut(usize, usize));

/// Default history memory budget (ch. 16): 256 MiB per document.
pub const DEFAULT_HISTORY_BUDGET_BYTES: usize = 256 * 1024 * 1024;

/// Snapshot of history memory use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryMetrics {
    pub history_entries: usize,
    pub history_bytes: usize,
    pub largest_entry: usize,
    pub average_entry: usize,
}

#[derive(Debug, Clone)]
struct HistoryEntry<T> {
    label: String,
    value: T,
    bytes: usize,
    version: usize,
}

/// Estado do histórico no instante em que um Save começou.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavePoint {
    version: usize,
    epoch: u64,
}

/// Início de uma transação no histórico: N operações viram **uma** entrada de
/// Undo ([`UndoStack::squash_since`]) ou são descartadas juntas
/// ([`UndoStack::rollback_to`]) — batch atômico de plugin/automação.
#[derive(Debug, Clone)]
pub struct HistoryMark<T> {
    next_version: usize,
    current_version: usize,
    redo: VecDeque<HistoryEntry<T>>,
    redo_bytes: usize,
}

/// Pilha genérica de undo/redo sobre estado clonável com rastreamento determinístico de estado salvo/dirty.
#[derive(Debug)]
pub struct UndoStack<T: Clone> {
    undo: VecDeque<HistoryEntry<T>>,
    redo: VecDeque<HistoryEntry<T>>,
    cap: usize,
    byte_budget: usize,
    undo_bytes: usize,
    redo_bytes: usize,
    clean_version: Option<usize>,
    current_version: usize,
    next_version: usize,
    /// Cresce a cada `mark_dirty` forçado (mudança fora do histórico).
    untracked_epoch: u64,
    /// Com pegada, o orçamento conta blocos compartilhados uma vez só.
    footprint: Option<HistoryFootprint<T>>,
}

impl<T: Clone> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> UndoStack<T> {
    pub fn new() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            cap: 4096,
            byte_budget: DEFAULT_HISTORY_BUDGET_BYTES,
            undo_bytes: 0,
            redo_bytes: 0,
            clean_version: Some(0),
            current_version: 0,
            next_version: 0,
            untracked_epoch: 0,
            footprint: None,
        }
    }

    /// Mede o histórico pelos blocos únicos dos snapshots (dados
    /// compartilhados entre entradas, como malhas em cópia na escrita, contam
    /// uma vez) em vez da soma dos tamanhos declarados.
    pub fn with_footprint(mut self, footprint: HistoryFootprint<T>) -> Self {
        self.footprint = Some(footprint);
        self.refresh_shared_bytes();
        self
    }

    /// Bytes únicos de uma pilha segundo a pegada.
    fn unique_bytes(footprint: HistoryFootprint<T>, entries: &VecDeque<HistoryEntry<T>>) -> usize {
        let mut seen = HashSet::new();
        let mut total = 0usize;
        for entry in entries {
            footprint(&entry.value, &mut |id, bytes| {
                if id == 0 || seen.insert(id) {
                    total = total.saturating_add(bytes);
                }
            });
        }
        total
    }

    /// Recalcula `undo_bytes`/`redo_bytes` quando há pegada.
    fn refresh_shared_bytes(&mut self) {
        if let Some(footprint) = self.footprint {
            self.undo_bytes = Self::unique_bytes(footprint, &self.undo);
            self.redo_bytes = Self::unique_bytes(footprint, &self.redo);
        }
    }

    pub fn with_budget(byte_budget: usize) -> Self {
        let mut s = Self::new();
        s.byte_budget = byte_budget.max(1);
        s
    }

    pub fn set_byte_budget(&mut self, bytes: usize) {
        self.byte_budget = bytes.max(1);
        self.evict_to_budget();
    }

    /// Salva o estado ATUAL antes de uma mutação (chamar antes de mudar).
    ///
    /// Estimativa superficial (`size_of_val`): existe só para testes com tipos
    /// pequenos. Produção usa [`Self::push_sized`]/[`Self::checkpoint_sized`] com
    /// tamanho profundo do documento.
    #[cfg(test)]
    pub fn checkpoint(&mut self, label: impl Into<String>, current: &T) {
        self.checkpoint_sized(label, current, estimate_bytes(current));
    }

    /// Checkpoint with an explicit payload size (clones `current`).
    /// Prefer [`Self::push_sized`] when the caller already owns a snapshot.
    pub fn checkpoint_sized(&mut self, label: impl Into<String>, current: &T, bytes: usize) {
        self.push_sized(label, current.clone(), bytes);
    }

    /// Registra um snapshot que o chamador já possui, sem clonar de novo.
    pub fn push_sized(&mut self, label: impl Into<String>, snapshot: T, bytes: usize) {
        let bytes = bytes.max(1);
        self.undo.push_back(HistoryEntry {
            label: label.into(),
            value: snapshot,
            bytes,
            version: self.current_version,
        });
        self.undo_bytes = self.undo_bytes.saturating_add(bytes);
        self.next_version = self.next_version.saturating_add(1);
        self.current_version = self.next_version;
        self.redo_bytes = 0;
        self.redo.clear();
        self.refresh_shared_bytes();
        self.evict_to_budget();
        while self.undo.len() > self.cap {
            if let Some(old) = self.undo.pop_front() {
                self.undo_bytes = self.undo_bytes.saturating_sub(old.bytes);
            }
        }
    }

    /// Descarta as entradas mais antigas até caber no orçamento. Sempre mantém
    /// ao menos uma entrada de undo, mesmo que sozinha exceda o orçamento (o
    /// último passo reversível não pode ser perdido). Redo obedece o mesmo teto.
    fn evict_to_budget(&mut self) {
        while self.undo_bytes > self.byte_budget && self.undo.len() > 1 {
            if let Some(old) = self.undo.pop_front() {
                self.undo_bytes = self.undo_bytes.saturating_sub(old.bytes);
                self.refresh_shared_bytes();
            }
        }
        while self.redo_bytes > self.byte_budget && self.redo.len() > 1 {
            if let Some(old) = self.redo.pop_front() {
                self.redo_bytes = self.redo_bytes.saturating_sub(old.bytes);
                self.refresh_shared_bytes();
            }
        }
    }

    pub fn metrics(&self) -> HistoryMetrics {
        let entries: Vec<usize> = self
            .undo
            .iter()
            .map(|e| e.bytes)
            .chain(self.redo.iter().map(|e| e.bytes))
            .collect();
        let history_entries = entries.len();
        let history_bytes = self.undo_bytes.saturating_add(self.redo_bytes);
        let largest_entry = entries.iter().copied().max().unwrap_or(0);
        let average_entry = history_bytes.checked_div(history_entries).unwrap_or(0);
        HistoryMetrics {
            history_entries,
            history_bytes,
            largest_entry,
            average_entry,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|e| e.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.back().map(|e| e.label.as_str())
    }
    pub fn depth(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }

    /// Marca o estado atual do histórico como sincronizado/salvo em disco.
    pub fn mark_clean(&mut self) {
        self.clean_version = Some(self.current_version);
    }

    /// Força o estado a ser marcado como não salvo (dirty).
    pub fn mark_dirty(&mut self) {
        self.clean_version = None;
        self.untracked_epoch = self.untracked_epoch.wrapping_add(1);
    }

    /// Captura o estado a gravar. Um Save que termina depois de novas edições
    /// só pode limpar *aquele* estado, nunca o mais recente.
    pub fn save_point(&self) -> SavePoint {
        SavePoint {
            version: self.current_version,
            epoch: self.untracked_epoch,
        }
    }

    /// Confirma que `point` foi gravado com sucesso. Se o histórico já avançou, o
    /// documento continua dirty; desfazer de volta até `point` volta a limpo.
    /// Mudanças fora do histórico feitas depois do `point` mantêm o dirty.
    pub fn mark_saved(&mut self, point: SavePoint) {
        if point.epoch == self.untracked_epoch {
            self.clean_version = Some(point.version);
        }
    }

    /// Verifica se o estado atual coincide exatamente com o ponto salvo.
    pub fn is_clean(&self) -> bool {
        self.clean_version == Some(self.current_version)
    }

    /// Verifica se há alterações não salvas no histórico.
    pub fn is_dirty(&self) -> bool {
        !self.is_clean()
    }

    /// Desfaz: guarda estado atual no redo, retorna estado anterior.
    #[cfg(test)]
    pub fn undo(&mut self, current: T) -> Option<T> {
        let bytes = estimate_bytes(&current);
        self.undo_sized(current, bytes)
    }

    pub fn undo_sized(&mut self, current: T, bytes: usize) -> Option<T> {
        let prev = self.undo.pop_back()?;
        self.undo_bytes = self.undo_bytes.saturating_sub(prev.bytes);
        let current_bytes = bytes.max(1);
        self.redo.push_back(HistoryEntry {
            label: prev.label,
            value: current,
            bytes: current_bytes,
            version: self.current_version,
        });
        self.current_version = prev.version;
        self.redo_bytes = self.redo_bytes.saturating_add(current_bytes);
        self.refresh_shared_bytes();
        self.evict_to_budget();
        Some(prev.value)
    }

    /// Refaz sem reutilizar uma identidade de estado de outra ramificação.
    #[cfg(test)]
    pub fn redo(&mut self, current: T) -> Option<T> {
        let bytes = estimate_bytes(&current);
        self.redo_sized(current, bytes)
    }

    pub fn redo_sized(&mut self, current: T, bytes: usize) -> Option<T> {
        let next = self.redo.pop_back()?;
        self.redo_bytes = self.redo_bytes.saturating_sub(next.bytes);
        let current_bytes = bytes.max(1);
        self.undo.push_back(HistoryEntry {
            label: next.label,
            value: current,
            bytes: current_bytes,
            version: self.current_version,
        });
        self.current_version = next.version;
        self.undo_bytes = self.undo_bytes.saturating_add(current_bytes);
        self.refresh_shared_bytes();
        self.evict_to_budget();
        Some(next.value)
    }

    /// Marca o início de uma transação.
    pub fn mark(&self) -> HistoryMark<T> {
        HistoryMark {
            next_version: self.next_version,
            current_version: self.current_version,
            redo: self.redo.clone(),
            redo_bytes: self.redo_bytes,
        }
    }

    /// Quantas entradas foram empilhadas desde `mark` (cada push avança
    /// `next_version` em 1; entradas expulsas pelo orçamento não contam).
    fn pushed_since(&self, mark: &HistoryMark<T>) -> usize {
        self.next_version
            .saturating_sub(mark.next_version)
            .min(self.undo.len())
    }

    /// Funde as entradas empilhadas desde `mark` numa só, rotulada `label`: ela
    /// guarda o estado de antes da transação, então um Undo desfaz tudo de uma
    /// vez. Retorna quantas entradas foram fundidas. Se o orçamento já expulsou
    /// a primeira entrada da transação, a mais antiga restante é usada.
    pub fn squash_since(&mut self, mark: &HistoryMark<T>, label: impl Into<String>) -> usize {
        let pushed = self.pushed_since(mark);
        if pushed == 0 {
            return 0;
        }
        let mut removed: Vec<HistoryEntry<T>> =
            (0..pushed).filter_map(|_| self.undo.pop_back()).collect();
        for entry in &removed {
            self.undo_bytes = self.undo_bytes.saturating_sub(entry.bytes);
        }
        // `removed` está da mais nova para a mais antiga.
        if let Some(mut oldest) = removed.pop() {
            oldest.label = label.into();
            self.undo_bytes = self.undo_bytes.saturating_add(oldest.bytes);
            self.undo.push_back(oldest);
        }
        self.refresh_shared_bytes();
        pushed
    }

    /// Descarta as entradas empilhadas desde `mark` e restaura a versão atual e
    /// o redo de antes da transação. Devolve o snapshot de antes da transação
    /// quando ainda está no histórico; o chamador restaura o documento.
    pub fn rollback_to(&mut self, mark: HistoryMark<T>) -> Option<T> {
        let pushed = self.pushed_since(&mark);
        let mut before = None;
        for _ in 0..pushed {
            if let Some(entry) = self.undo.pop_back() {
                self.undo_bytes = self.undo_bytes.saturating_sub(entry.bytes);
                before = Some(entry.value);
            }
        }
        self.current_version = mark.current_version;
        self.redo = mark.redo;
        self.redo_bytes = mark.redo_bytes;
        self.refresh_shared_bytes();
        before
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.redo_bytes = 0;
        self.current_version = 0;
        self.next_version = 0;
        self.clean_version = Some(0);
    }
}

#[cfg(test)]
fn estimate_bytes<T>(value: &T) -> usize {
    std::mem::size_of_val(value).max(1)
}

/// Trait genérica para comandos executáveis e transacionais.
pub trait Command<Context, Res = (), Err = String>: Send + Sync {
    /// Rótulo legível para telemetria e pilha de desfazer/refazer (Undo/Redo).
    fn label(&self) -> &'static str;

    /// Executa a operação contra o contexto mutável.
    fn execute(&self, ctx: &mut Context) -> Result<Res, Err>;

    /// Indica se o comando altera o estado persistente e exige captura prévia de checkpoint de Undo.
    fn is_destructive(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squash_turns_a_transaction_into_one_undo_step() {
        let mut stack: UndoStack<u32> = UndoStack::new();
        stack.checkpoint("before", &0);
        let mark = stack.mark();
        stack.checkpoint("a", &1);
        stack.checkpoint("b", &2);
        stack.checkpoint("c", &3);
        assert_eq!(stack.squash_since(&mark, "plugin batch"), 3);
        assert_eq!(stack.depth(), (2, 0));
        assert_eq!(stack.undo_label(), Some("plugin batch"));
        // O único passo restaura o estado de antes da transação.
        assert_eq!(stack.undo(4), Some(1));
    }

    #[test]
    fn rollback_discards_the_transaction_and_restores_redo_and_dirty() {
        let mut stack: UndoStack<u32> = UndoStack::new();
        stack.checkpoint("x", &0);
        stack.mark_clean();
        stack.checkpoint("y", &1);
        assert_eq!(stack.undo(2), Some(1)); // redo agora tem um passo
        let clean_before = stack.is_clean();
        let mark = stack.mark();
        stack.checkpoint("a", &5);
        stack.checkpoint("b", &6);
        assert_eq!(stack.rollback_to(mark), Some(5));
        assert_eq!(stack.depth(), (1, 1));
        assert_eq!(stack.is_clean(), clean_before);
        assert_eq!(stack.redo_label(), Some("y"));
    }

    #[test]
    fn empty_transactions_change_nothing() {
        let mut stack: UndoStack<u32> = UndoStack::new();
        stack.checkpoint("x", &0);
        let mark = stack.mark();
        assert_eq!(stack.squash_since(&mark, "noop"), 0);
        assert_eq!(stack.undo_label(), Some("x"));
        assert_eq!(stack.rollback_to(mark), None);
        assert_eq!(stack.depth(), (1, 0));
    }

    #[test]
    fn footprint_counts_shared_blocks_once() {
        // Cada snapshot: um bloco compartilhado de 100 bytes (id 7) e 10 próprios.
        fn footprint(_: &u8, visit: &mut dyn FnMut(usize, usize)) {
            visit(7, 100);
            visit(0, 10);
        }
        let mut shared: UndoStack<u8> = UndoStack::new().with_footprint(footprint);
        for i in 0..5 {
            shared.push_sized("step", i, 110);
        }
        assert_eq!(shared.metrics().history_bytes, 100 + 5 * 10);
        let mut plain: UndoStack<u8> = UndoStack::new();
        for i in 0..5 {
            plain.push_sized("step", i, 110);
        }
        assert_eq!(plain.metrics().history_bytes, 5 * 110);
        // Com orçamento apertado, a pegada mantém mais entradas.
        shared.set_byte_budget(200);
        plain.set_byte_budget(200);
        assert!(shared.depth().0 > plain.depth().0);
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut st: UndoStack<Vec<i32>> = UndoStack::new();
        let mut cur = vec![1];
        st.checkpoint("add", &cur);
        cur.push(2);
        assert_eq!(st.undo(cur.clone()), Some(vec![1]));
        let cur = vec![1];
        assert_eq!(st.redo(cur), Some(vec![1, 2]));
        assert!(!st.can_redo());
    }

    #[test]
    fn checkpoint_clears_redo() {
        let mut st: UndoStack<i32> = UndoStack::new();
        st.checkpoint("a", &1);
        let _ = st.undo(2);
        assert!(st.can_redo());
        st.checkpoint("b", &1);
        assert!(!st.can_redo());
    }

    struct IncrementCmd(i32);
    impl Command<i32> for IncrementCmd {
        fn label(&self) -> &'static str {
            "increment"
        }
        fn execute(&self, ctx: &mut i32) -> Result<(), String> {
            *ctx += self.0;
            Ok(())
        }
    }

    #[test]
    fn saving_an_older_revision_keeps_the_newer_edit_dirty() {
        let mut st = UndoStack::<i32>::new();
        st.checkpoint("edit 1", &1);
        let point = st.save_point();
        // Edição enquanto o Save ainda gravava.
        st.checkpoint("edit 2", &2);
        st.mark_saved(point);
        assert!(st.is_dirty(), "o estado mais novo não foi gravado");
        // Voltar ao estado gravado deixa o documento limpo.
        assert_eq!(st.undo(3), Some(2));
        assert!(st.is_clean());
    }

    #[test]
    fn untracked_changes_after_the_save_point_stay_dirty() {
        let mut st = UndoStack::<i32>::new();
        st.checkpoint("edit", &1);
        let point = st.save_point();
        st.mark_dirty();
        st.mark_saved(point);
        assert!(st.is_dirty());
    }

    #[test]
    fn test_undo_stack_dirty_state_tracking() {
        let mut st: UndoStack<i32> = UndoStack::new();
        // Initial state is clean (at saved/start version 0)
        assert!(st.is_clean());
        assert!(!st.is_dirty());

        // Mutation 1 -> dirty
        let mut cur = 0;
        st.checkpoint("step 1", &cur);
        cur = 1;
        assert!(st.is_dirty());

        // Mark saved (clean)
        st.mark_clean();
        assert!(st.is_clean());
        assert!(!st.is_dirty());

        // Mutation 2 -> dirty
        st.checkpoint("step 2", &cur);
        cur = 2;
        assert!(st.is_dirty());

        // Undo -> back to step 1 which was marked clean!
        let prev = st.undo(cur).unwrap();
        assert_eq!(prev, 1);
        assert!(
            st.is_clean(),
            "Undoing back to the saved state must be clean"
        );

        // Redo -> forward to step 2 which is dirty!
        let next = st.redo(prev).unwrap();
        assert_eq!(next, 2);
        assert!(st.is_dirty(), "Redoing to an unsaved state must be dirty");

        // Force dirty
        st.mark_clean();
        assert!(st.is_clean());
        st.mark_dirty();
        assert!(st.is_dirty());
    }

    #[test]
    fn command_trait_executes_and_identifies() {
        let cmd = IncrementCmd(5);
        assert_eq!(cmd.label(), "increment");
        assert!(cmd.is_destructive());
        let mut val = 10;
        assert!(cmd.execute(&mut val).is_ok());
        assert_eq!(val, 15);
    }

    #[test]
    fn byte_budget_evicts_oldest_not_by_count() {
        let mut st: UndoStack<Vec<u8>> = UndoStack::with_budget(64);
        for i in 0..8 {
            let cur = vec![i; 20];
            st.checkpoint_sized("step", &cur, 20);
        }
        let metrics = st.metrics();
        assert!(metrics.history_bytes <= 64);
        assert!(metrics.history_entries <= 4);
        assert!(metrics.largest_entry <= 20);
    }

    #[test]
    fn push_sized_takes_ownership_without_extra_clone() {
        let mut st: UndoStack<Vec<u8>> = UndoStack::with_budget(1024);
        st.push_sized("a", vec![1, 2, 3], 3);
        assert_eq!(st.depth(), (1, 0));
        assert_eq!(st.metrics().history_bytes, 3);
        assert_eq!(st.undo_sized(vec![9], 1), Some(vec![1, 2, 3]));
    }

    #[test]
    fn redo_stack_obeys_the_byte_budget() {
        let mut st: UndoStack<Vec<u8>> = UndoStack::with_budget(64);
        for i in 0..8u8 {
            st.push_sized("step", vec![i; 32], 32);
        }
        // Desfazer tudo empilha estados de 32 B no redo; o teto de 64 B vale também aqui.
        for _ in 0..8 {
            let _ = st.undo_sized(vec![0; 32], 32);
        }
        assert!(st.metrics().history_bytes <= 64 + 32, "{:?}", st.metrics());
        assert!(st.can_redo());
    }
}
