//! Paletas para o seletor de cor da GUI (snapshot) e tradução do pedido `palette`
//! em `PaletteCmd`. Apresentação e roteamento apenas: a mutação é do núcleo.

use petunia_core::{PaletteCmd, PaletteOp};
use petunia_project::Project;
use serde_json::{Value, json};
use uuid::Uuid;

/// Paleta do projeto primeiro (id nulo, sem apagar nem renomear), depois as nomeadas.
pub(crate) fn snapshot(project: &Project) -> Value {
    let mut rows = vec![json!({
        "id": Value::Null,
        "name": "Projeto",
        "builtin": true,
        "colors": project.palette.iter().map(|c| [c[0], c[1], c[2], 1.0]).collect::<Vec<_>>(),
    })];
    rows.extend(project.palettes.iter().map(|palette| {
        json!({
            "id": palette.id.to_string(),
            "name": palette.name,
            "builtin": false,
            "colors": palette.colors,
        })
    }));
    Value::Array(rows)
}

fn palette_id(request: &Value) -> Result<Option<Uuid>, String> {
    match request.get("id") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if text.is_empty() => Ok(None),
        Some(Value::String(text)) => Uuid::parse_str(text)
            .map(Some)
            .map_err(|_| "Identificador de paleta inválido".to_string()),
        Some(_) => Err("Identificador de paleta inválido".into()),
    }
}

fn named_id(request: &Value) -> Result<Uuid, String> {
    palette_id(request)?
        .ok_or_else(|| "A paleta do projeto não pode ser renomeada nem apagada".into())
}

fn color(request: &Value) -> Result<[f32; 4], String> {
    let parts = request["color"].as_array().ok_or("Cor inválida")?;
    if parts.len() != 3 && parts.len() != 4 {
        return Err("Cor inválida".into());
    }
    let mut rgba = [1.0f32; 4];
    for (slot, part) in rgba.iter_mut().zip(parts) {
        let value = part.as_f64().ok_or("Cor inválida")? as f32;
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err("Cor fora de 0..1".into());
        }
        *slot = value;
    }
    Ok(rgba)
}

fn index(request: &Value) -> Result<usize, String> {
    request["index"]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| "Índice de cor inválido".into())
}

fn name(request: &Value) -> Result<String, String> {
    request["name"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Nome inválido".into())
}

/// `{"op": "create"|"rename"|"delete"|"add"|"set"|"remove", ...}`.
pub(crate) fn command_for(request: &Value) -> Result<PaletteCmd, String> {
    let op = match request["op"].as_str().ok_or("Operação de paleta ausente")? {
        "create" => PaletteOp::Create {
            name: name(request)?,
        },
        "rename" => PaletteOp::Rename {
            palette: named_id(request)?,
            name: name(request)?,
        },
        "delete" => PaletteOp::Delete {
            palette: named_id(request)?,
        },
        "add" => PaletteOp::AddColor {
            palette: palette_id(request)?,
            color: color(request)?,
        },
        "set" => PaletteOp::SetColor {
            palette: palette_id(request)?,
            index: index(request)?,
            color: color(request)?,
        },
        "remove" => PaletteOp::RemoveColor {
            palette: palette_id(request)?,
            index: index(request)?,
        },
        other => return Err(format!("Operação de paleta desconhecida: {other}")),
    };
    Ok(PaletteCmd { op })
}
