//! Petunia3D — Importação, exportação e presets de paletas de cores.
//! Suporta formatos de intercâmbio amplamente adotados:
//! - Hex plain text (#RRGGBB por linha)
//! - GIMP Palette (.gpl), compatível com Blender, Aseprite e GIMP
//! - Presets clássicos de estética retro: Pico-8, Game Boy, C64.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Limites das paletas nomeadas (o arquivo é entrada hostil).
pub const MAX_PALETTES: usize = 64;
pub const MAX_PALETTE_COLORS: usize = 256;
pub const MAX_PALETTE_NAME_CHARS: usize = 64;

/// Paleta nomeada do documento (seletor de cor): cores RGBA em 0..=1.
/// A paleta do projeto (`Project::palette`, RGB) continua existindo à parte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedPalette {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub colors: Vec<[f32; 4]>,
}

impl NamedPalette {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            colors: Vec::new(),
        }
    }
}

/// Cor RGBA válida: finita e dentro de 0..=1 (o que entra numa paleta).
pub fn valid_rgba(color: &[f32; 4]) -> bool {
    color
        .iter()
        .all(|c| c.is_finite() && (0.0..=1.0).contains(c))
}

/// Corrige paletas vindas de arquivo: corta excessos, descarta cores inválidas,
/// conserta nomes vazios ou longos e ids repetidos. Devolve quantas correções fez.
pub fn repair_palettes(palettes: &mut Vec<NamedPalette>) -> usize {
    let mut fixes = 0;
    if palettes.len() > MAX_PALETTES {
        fixes += palettes.len() - MAX_PALETTES;
        palettes.truncate(MAX_PALETTES);
    }
    let mut seen = std::collections::HashSet::new();
    for palette in palettes.iter_mut() {
        if !seen.insert(palette.id) {
            palette.id = Uuid::new_v4();
            fixes += 1;
        }
        let before = palette.colors.len();
        palette.colors.retain(valid_rgba);
        palette.colors.truncate(MAX_PALETTE_COLORS);
        fixes += before - palette.colors.len();
        if palette.name.trim().is_empty() {
            palette.name = "Paleta".into();
            fixes += 1;
        }
        if palette.name.chars().count() > MAX_PALETTE_NAME_CHARS {
            palette.name = palette.name.chars().take(MAX_PALETTE_NAME_CHARS).collect();
            fixes += 1;
        }
    }
    fixes
}

/// Exporta paleta para formato hexadecimal (#RRGGBB por linha).
pub fn export_hex(palette: &[[f32; 3]]) -> String {
    let mut out = String::new();
    for c in palette {
        let r = (c[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (c[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (c[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        out.push_str(&format!("#{:02X}{:02X}{:02X}\n", r, g, b));
    }
    out
}

/// Importa paleta de texto hexadecimal (#RRGGBB por linha).
pub fn import_hex(content: &str) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
        if hex.len() == 6
            && let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            )
        {
            out.push([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        }
    }
    out
}

/// Exporta paleta para formato padrão GIMP Palette (.gpl).
pub fn export_gpl(name: &str, palette: &[[f32; 3]]) -> String {
    let mut out = format!("GIMP Palette\nName: {}\nColumns: 4\n#\n", name);
    for (i, c) in palette.iter().enumerate() {
        let r = (c[0].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (c[1].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (c[2].clamp(0.0, 1.0) * 255.0).round() as u8;
        out.push_str(&format!("{:3} {:3} {:3} Color {}\n", r, g, b, i + 1));
    }
    out
}

/// Importa paleta de arquivo GIMP Palette (.gpl).
pub fn import_gpl(content: &str) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#')
            || trimmed.starts_with("GIMP")
            || trimmed.starts_with("Name:")
            || trimmed.starts_with("Columns:")
            || trimmed.is_empty()
        {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3
            && let (Ok(r), Ok(g), Ok(b)) = (
                parts[0].parse::<u8>(),
                parts[1].parse::<u8>(),
                parts[2].parse::<u8>(),
            )
        {
            out.push([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
        }
    }
    out
}

/// Paleta clássica Pico-8 (16 cores).
pub fn preset_pico8() -> Vec<[f32; 3]> {
    vec![
        [0.0, 0.0, 0.0],
        [0.11, 0.17, 0.33],
        [0.49, 0.15, 0.33],
        [0.0, 0.53, 0.32],
        [0.67, 0.32, 0.21],
        [0.37, 0.34, 0.31],
        [0.76, 0.76, 0.78],
        [1.0, 0.95, 0.91],
        [1.0, 0.0, 0.3],
        [1.0, 0.64, 0.0],
        [1.0, 0.93, 0.15],
        [0.0, 0.89, 0.21],
        [0.16, 0.68, 1.0],
        [0.51, 0.46, 0.61],
        [1.0, 0.47, 0.66],
        [1.0, 0.8, 0.67],
    ]
}

/// Paleta clássica Game Boy (4 tons esverdeados).
pub fn preset_gameboy() -> Vec<[f32; 3]> {
    vec![
        [0.06, 0.22, 0.06],
        [0.19, 0.38, 0.19],
        [0.55, 0.67, 0.06],
        [0.61, 0.73, 0.06],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let colors = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let hex_str = export_hex(&colors);
        assert!(hex_str.contains("#FF0000"));
        assert!(hex_str.contains("#00FF00"));
        assert!(hex_str.contains("#0000FF"));
        let parsed = import_hex(&hex_str);
        assert_eq!(parsed.len(), 3);
        assert!((parsed[0][0] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn gpl_roundtrip() {
        let colors = vec![[1.0, 0.5, 0.2], [0.2, 0.8, 0.4]];
        let gpl_str = export_gpl("TestPalette", &colors);
        assert!(gpl_str.contains("GIMP Palette"));
        let parsed = import_gpl(&gpl_str);
        assert_eq!(parsed.len(), 2);
    }
}
