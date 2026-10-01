//! Camada C-ABI / FFI para Integração com Frontends Externos (Gauntlet G10).
//!
//! Permite que frontends externos (C++, C#, Python, Go, etc.) controlem o núcleo
//! do Petunia3D com segurança de memória, sem acoplamento a UI e com contratos estáveis.

#![allow(clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::cell::RefCell;
use std::ffi::{CStr, c_char};
use std::path::Path;
use std::ptr;

use petunia_core::{
    AddPrimitiveCmd, AppState, ClearSelectionCmd, CommandError, DeleteSelectionCmd,
    DuplicateSelectionCmd, ExtrudeSelectedCmd, PrimitiveKind, ProjectService, RedoCmd,
    ScaleSelectionCmd, SelectAllCmd, SubdivideSelectionCmd, UndoCmd,
};
use uuid::Uuid;

// Códigos de erro canônicos da C-ABI
pub const PETUNIA_OK: i32 = 0;
pub const PETUNIA_ERR_NULL_PTR: i32 = -1;
pub const PETUNIA_ERR_INVALID_UTF8: i32 = -2;
pub const PETUNIA_ERR_OPERATION_FAILED: i32 = -3;
pub const PETUNIA_ERR_BUFFER_TOO_SMALL: i32 = -4;
pub const PETUNIA_ERR_NOT_FOUND: i32 = -5;
pub const PETUNIA_ERR_UNKNOWN_PRIMITIVE: i32 = -6;
pub const PETUNIA_ERR_UNKNOWN_COMMAND: i32 = -7;

fn command_error_code(err: &CommandError) -> i32 {
    match err {
        CommandError::UnknownPrimitive(_) => PETUNIA_ERR_UNKNOWN_PRIMITIVE,
        CommandError::UnknownCommand(_) => PETUNIA_ERR_UNKNOWN_COMMAND,
        CommandError::NoActiveAsset | CommandError::InvalidAssetIndex(_) => PETUNIA_ERR_NOT_FOUND,
        _ => PETUNIA_ERR_OPERATION_FAILED,
    }
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

fn set_last_error(msg: impl Into<String>) {
    LAST_ERROR.with(|err| {
        *err.borrow_mut() = msg.into();
    });
}

/// Contexto opaco de sessão do Petunia3D.
pub struct PetuniaContext {
    pub state: AppState,
}

// -----------------------------------------------------------------------------
// Helpers Internos Seguros
// -----------------------------------------------------------------------------

unsafe fn c_str_to_str<'a>(ptr: *const c_char) -> Result<&'a str, i32> {
    if ptr.is_null() {
        set_last_error("Ponteiro de string nulo recebido.");
        return Err(PETUNIA_ERR_NULL_PTR);
    }
    CStr::from_ptr(ptr).to_str().map_err(|_| {
        set_last_error("String fornecida contém sequência UTF-8 inválida.");
        PETUNIA_ERR_INVALID_UTF8
    })
}

unsafe fn write_to_c_buffer(src: &str, out_buf: *mut c_char, out_len: usize) -> i32 {
    if out_buf.is_null() {
        set_last_error("Buffer de saída nulo.");
        return PETUNIA_ERR_NULL_PTR;
    }
    if out_len == 0 {
        set_last_error("Comprimento do buffer de saída é zero.");
        return PETUNIA_ERR_BUFFER_TOO_SMALL;
    }

    let bytes = src.as_bytes();
    if bytes.len() + 1 > out_len {
        set_last_error(format!(
            "Buffer insuficiente: necessário {}, fornecido {}",
            bytes.len() + 1,
            out_len
        ));
        return PETUNIA_ERR_BUFFER_TOO_SMALL;
    }

    ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, bytes.len());
    *out_buf.add(bytes.len()) = 0; // null terminator
    PETUNIA_OK
}

// -----------------------------------------------------------------------------
// Lifecycle do Contexto
// -----------------------------------------------------------------------------

/// Cria uma nova instância de contexto do Petunia3D com um novo projeto.
/// `lang` pode ser nulo (padrão: "en").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_context_create(lang: *const c_char) -> *mut PetuniaContext {
    let language = if lang.is_null() {
        "en"
    } else {
        c_str_to_str(lang).unwrap_or("en")
    };

    let mut state = AppState::new(language);
    ProjectService::new_project(&mut state);

    let ctx = Box::new(PetuniaContext { state });
    Box::into_raw(ctx)
}

/// Destrói uma instância de contexto do Petunia3D liberando todos os recursos.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_context_destroy(ctx: *mut PetuniaContext) {
    if !ctx.is_null() {
        drop(Box::from_raw(ctx));
    }
}

// -----------------------------------------------------------------------------
// Gerenciamento de Projetos e I/O
// -----------------------------------------------------------------------------

/// Inicializa um novo projeto limpo no contexto.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_new_project(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    ProjectService::new_project(&mut ctx.state);
    PETUNIA_OK
}

/// Carrega um arquivo `.petunia` no contexto.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_load_project(
    ctx: *mut PetuniaContext,
    path: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &mut *ctx;
    match ProjectService::load_project(&mut ctx.state, Path::new(path_str)) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao carregar projeto: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Salva o projeto atual no formato `.petunia`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_save_project(
    ctx: *mut PetuniaContext,
    path: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &mut *ctx;
    match ProjectService::save_project(&mut ctx.state, Path::new(path_str)) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao salvar projeto: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Importa uma malha Wavefront OBJ como novo asset no projeto.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_import_obj(ctx: *mut PetuniaContext, path: *const c_char) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &mut *ctx;
    match ProjectService::import_obj(&mut ctx.state, Path::new(path_str)) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao importar OBJ: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Importa malha glTF/GLB (com materiais e texturas) como novos assets.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_import_gltf(ctx: *mut PetuniaContext, path: *const c_char) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &mut *ctx;
    match ProjectService::import_file_pipeline(
        &mut ctx.state,
        Path::new(path_str),
        &petunia_project::pipeline::ImportOptions::default(),
    ) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao importar glTF/GLB: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Exporta o asset ativo para Wavefront OBJ.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_export_obj(ctx: *mut PetuniaContext, path: *const c_char) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &*ctx;
    let active_idx = ctx.state.project.active;
    match ProjectService::export_obj(&ctx.state, active_idx, Path::new(path_str)) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao exportar OBJ: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Exporta todos os assets visíveis da cena para o formato binário GLB (glTF 2.0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_export_glb(ctx: *mut PetuniaContext, path: *const c_char) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let path_str = match c_str_to_str(path) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &*ctx;
    let all_indices: Vec<usize> = (0..ctx.state.project.assets.len()).collect();
    match ProjectService::export_glb(&ctx.state, &all_indices, Path::new(path_str)) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao exportar GLB: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

// -----------------------------------------------------------------------------
// Primitivas e Despacho de Ferramentas / Comandos
// -----------------------------------------------------------------------------

/// Insere uma nova primitiva na cena ("Cube", "Plane", "Sphere", "Cylinder", "Cylinder8", "Capsule", "Cone").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_add_primitive(
    ctx: *mut PetuniaContext,
    kind: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let kind_str = match c_str_to_str(kind) {
        Ok(s) => s,
        Err(err) => return err,
    };

    let ctx = &mut *ctx;
    let kind = match PrimitiveKind::parse(kind_str) {
        Ok(k) => k,
        Err(e) => {
            set_last_error(e.to_string());
            return command_error_code(&e);
        }
    };
    match ctx.state.dispatch(&AddPrimitiveCmd {
        kind,
        name: Some(kind_str.to_string()),
        at_cursor: true,
    }) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao adicionar primitiva: {e}"));
            command_error_code(&e)
        }
    }
}

/// Desfaz a última ação executada.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_undo(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&UndoCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao desfazer: {e}"));
            command_error_code(&e)
        }
    }
}

/// Refaz a última ação desfeita.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_redo(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&RedoCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao refazer: {e}"));
            command_error_code(&e)
        }
    }
}

/// Seleciona todos os elementos do ativo atual.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_select_all(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    if ctx.state.selection_domain() == petunia_core::SelectionDomain::Object {
        ctx.state
            .set_selection_domain(petunia_core::SelectionDomain::Face);
    }
    match ctx.state.dispatch(&SelectAllCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao executar SelectAll: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Limpa toda a seleção atual.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_clear_selection(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&ClearSelectionCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao executar ClearSelection: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Remove os elementos selecionados.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_delete_selection(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&DeleteSelectionCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao executar DeleteSelection: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Duplica a seleção ou o objeto atual.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_duplicate_selection(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&DuplicateSelectionCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao executar DuplicateSelection: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Extrude os elementos selecionados com a distância especificada.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_extrude_selection(ctx: *mut PetuniaContext, distance: f32) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&ExtrudeSelectedCmd { dist: distance }) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao extrudar: {e}"));
            command_error_code(&e)
        }
    }
}

/// Subdivide os elementos selecionados.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_subdivide_selection(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&SubdivideSelectionCmd) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao subdividir: {e}"));
            command_error_code(&e)
        }
    }
}

/// Aplica escala uniforme aos elementos selecionados.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_scale_selection(ctx: *mut PetuniaContext, scale: f32) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &mut *ctx;
    match ctx.state.dispatch(&ScaleSelectionCmd { factor: scale }) {
        Ok(_) => PETUNIA_OK,
        Err(e) => {
            set_last_error(format!("Erro ao escalar: {e}"));
            command_error_code(&e)
        }
    }
}

// -----------------------------------------------------------------------------
// Consultas da Cena e Seleção (Queries & DTOs)
// -----------------------------------------------------------------------------

/// Retorna o número total de assets no projeto.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_get_asset_count(ctx: *mut PetuniaContext) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    ctx.state.project.assets.len() as i32
}

/// Obtém os totais de vértices e faces da cena completa.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_get_scene_summary(
    ctx: *mut PetuniaContext,
    out_total_verts: *mut u32,
    out_total_faces: *mut u32,
) -> i32 {
    if ctx.is_null() || out_total_verts.is_null() || out_total_faces.is_null() {
        set_last_error("Ponteiro de parâmetro nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    let (verts, faces) = ctx.state.project.totals();
    *out_total_verts = verts as u32;
    *out_total_faces = faces as u32;
    PETUNIA_OK
}

/// Copia o nome do asset ativo para o buffer C fornecido.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_get_active_asset_name(
    ctx: *mut PetuniaContext,
    buffer: *mut c_char,
    buffer_len: usize,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    if let Some(active) = ctx.state.project.active() {
        write_to_c_buffer(&active.name, buffer, buffer_len)
    } else {
        set_last_error("Nenhum asset ativo na cena");
        PETUNIA_ERR_NOT_FOUND
    }
}

/// Obtém as contagens de vértices e faces do asset ativo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_get_active_asset_stats(
    ctx: *mut PetuniaContext,
    out_verts: *mut u32,
    out_faces: *mut u32,
) -> i32 {
    if ctx.is_null() || out_verts.is_null() || out_faces.is_null() {
        set_last_error("Ponteiro de parâmetro nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    if let Some(active) = ctx.state.project.active() {
        *out_verts = active.mesh.vert_count() as u32;
        *out_faces = active.mesh.faces.len() as u32;
        PETUNIA_OK
    } else {
        set_last_error("Nenhum asset ativo na cena");
        PETUNIA_ERR_NOT_FOUND
    }
}

/// Ativa um asset da cena através de seu identificador UUID estável (em formato de string).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_set_active_asset_by_id(
    ctx: *mut PetuniaContext,
    uuid_str: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let id_s = match c_str_to_str(uuid_str) {
        Ok(s) => s,
        Err(err) => return err,
    };
    let uuid = match Uuid::parse_str(id_s) {
        Ok(u) => u,
        Err(_) => {
            set_last_error(format!("Formato de UUID inválido: {id_s}"));
            return PETUNIA_ERR_OPERATION_FAILED;
        }
    };

    let ctx = &mut *ctx;
    if ctx.state.set_active_asset_by_id(uuid) {
        PETUNIA_OK
    } else {
        set_last_error(format!("Asset com UUID {uuid} não encontrado."));
        PETUNIA_ERR_NOT_FOUND
    }
}

/// Deleta um asset da cena através de seu identificador UUID estável.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_delete_asset_by_id(
    ctx: *mut PetuniaContext,
    uuid_str: *const c_char,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let id_s = match c_str_to_str(uuid_str) {
        Ok(s) => s,
        Err(err) => return err,
    };
    let uuid = match Uuid::parse_str(id_s) {
        Ok(u) => u,
        Err(_) => {
            set_last_error(format!("Formato de UUID inválido: {id_s}"));
            return PETUNIA_ERR_OPERATION_FAILED;
        }
    };

    let ctx = &mut *ctx;
    if ctx.state.delete_asset_by_id(uuid) {
        PETUNIA_OK
    } else {
        set_last_error(format!("Asset com UUID {uuid} não encontrado."));
        PETUNIA_ERR_NOT_FOUND
    }
}

/// Serializa a hierarquia da cena (`SceneHierarchyDto`) em JSON e preenche o buffer C fornecido.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_query_scene_hierarchy_json(
    ctx: *mut PetuniaContext,
    buffer: *mut c_char,
    buffer_len: usize,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    let hierarchy = ctx.state.query_scene_hierarchy();
    match serde_json::to_string(&hierarchy) {
        Ok(json) => write_to_c_buffer(&json, buffer, buffer_len),
        Err(e) => {
            set_last_error(format!("Falha na serialização JSON: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Serializa os detalhes da seleção (`SelectionDetailsDto`) em JSON e preenche o buffer C fornecido.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_query_selection_details_json(
    ctx: *mut PetuniaContext,
    buffer: *mut c_char,
    buffer_len: usize,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    let details = ctx.state.query_selection_details();
    match serde_json::to_string(&details) {
        Ok(json) => write_to_c_buffer(&json, buffer, buffer_len),
        Err(e) => {
            set_last_error(format!("Falha na serialização JSON: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

/// Serializa o status da ferramenta atual (`ToolStatusDto`) em JSON e preenche o buffer C fornecido.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_query_tool_status_json(
    ctx: *mut PetuniaContext,
    buffer: *mut c_char,
    buffer_len: usize,
) -> i32 {
    if ctx.is_null() {
        set_last_error("Contexto nulo");
        return PETUNIA_ERR_NULL_PTR;
    }
    let ctx = &*ctx;
    let status = ctx.state.query_tool_status();
    match serde_json::to_string(&status) {
        Ok(json) => write_to_c_buffer(&json, buffer, buffer_len),
        Err(e) => {
            set_last_error(format!("Falha na serialização JSON: {e}"));
            PETUNIA_ERR_OPERATION_FAILED
        }
    }
}

// -----------------------------------------------------------------------------
// Diagnóstico de Erros
// -----------------------------------------------------------------------------

/// Copia a última mensagem de erro registrada para o buffer C fornecido.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_last_error_message(buffer: *mut c_char, buffer_len: usize) -> i32 {
    LAST_ERROR.with(|err| {
        let msg = err.borrow();
        write_to_c_buffer(&msg, buffer, buffer_len)
    })
}
