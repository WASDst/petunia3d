//! Resolução segura de caminhos que vêm de dentro de um arquivo externo.
//!
//! Um OBJ pode apontar `mtllib ../outro.mtl` e um glTF pode apontar
//! `../../etc/passwd`: o importador nunca deve ler fora do diretório do
//! arquivo. A regra é a mesma nos dois formatos: caminho relativo, sem `..`,
//! sem raiz e dentro do diretório base (comparação canônica).

use std::path::{Component, Path, PathBuf};

/// Resolve `uri` contra `base_dir` recusando escapes e esquemas remotos.
///
/// Devolve a mensagem de recusa pronta para o usuário (o chamador decide o
/// tipo de erro: aviso na importação de textura, recusa no pipeline).
pub fn resolve_external_path(base_dir: &Path, uri: &str) -> Result<PathBuf, String> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return Err("caminho vazio".to_string());
    }
    // Sem rede no importador: esquemas remotos são recusados, não baixados.
    if trimmed.contains("://") {
        return Err(format!(
            "URI remota ou esquema não suportado (sem rede no import): '{uri}'"
        ));
    }
    let rel_path = Path::new(trimmed);
    if rel_path.is_absolute()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || (trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':')
    {
        return Err(format!("caminho absoluto não permitido: '{uri}'"));
    }
    for component in rel_path.components() {
        match component {
            Component::ParentDir => {
                return Err(format!("path traversal ('..') detectado: '{uri}'"));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!("caminho com raiz/prefixo detectado: '{uri}'"));
            }
            _ => {}
        }
    }
    let resolved = base_dir.join(rel_path);
    // O arquivo precisa existir para a comparação canônica; se não existir,
    // a leitura seguinte vai falhar com uma mensagem melhor.
    if let (Ok(canon_base), Ok(canon_target)) = (base_dir.canonicalize(), resolved.canonicalize())
        && !canon_target.starts_with(&canon_base)
    {
        return Err(format!("caminho escapa do diretório base: '{uri}'"));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_and_absolute_paths_are_refused() {
        let base = std::env::temp_dir();
        assert!(resolve_external_path(&base, "../segredo.png").is_err());
        assert!(resolve_external_path(&base, "/etc/passwd").is_err());
        assert!(resolve_external_path(&base, "C:\\windows\\a.png").is_err());
        assert!(resolve_external_path(&base, "https://exemplo/a.png").is_err());
    }

    #[test]
    fn a_plain_relative_name_is_accepted() {
        let base = std::env::temp_dir();
        let resolved = resolve_external_path(&base, "textura.png").expect("caminho relativo");
        assert!(resolved.ends_with("textura.png"));
    }
}
