//! Importação de modelos pela porta da interface (`import_model`).
//!
//! Reproduz o caminho que o usuário percorre: ação da interface →
//! `UiIntent::ImportModelFrom` → pipeline de importação → documento.

use petunia_ui_cxxqt::editor::QtEditor;
use serde_json::json;
use std::path::{Path, PathBuf};

/// PNG 1×1 vermelho (válido, gerado uma vez e embutido).
const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
];

const CUBE_OBJ: &str = "\
mtllib cube.mtl
o Cube
v -1 -1 -1
v 1 -1 -1
v 1 1 -1
v -1 1 -1
v -1 -1 1
v 1 -1 1
v 1 1 1
v -1 1 1
vt 0 0
vt 1 0
vt 1 1
vt 0 1
vt 0 0
vt 1 0
vt 1 1
vt 0 1
vn 0 0 1
usemtl Vermelho
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
f 5/5/1 6/6/1 7/7/1
f 5/5/1 7/7/1 8/8/1
f 1/1/1 2/2/1 6/6/1
f 1/1/1 6/6/1 5/5/1
f 2/2/1 3/3/1 7/7/1
f 2/2/1 7/7/1 6/6/1
f 3/3/1 4/4/1 8/8/1
f 3/3/1 8/8/1 7/7/1
f 4/4/1 1/1/1 5/5/1
f 4/4/1 5/5/1 8/8/1
";

const CUBE_MTL: &str = "\
newmtl Vermelho
Ka 1.000 0.000 0.000
Kd 1.000 0.000 0.000
Ks 0.000 0.000 0.000
d 1.0
illum 2
map_Kd cube.png
";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("petunia-import-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("diretório temporário");
    dir
}

fn write(dir: &Path, name: &str, data: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, data).expect("escrever fixture");
    path
}

/// glTF 2.0 com buffer externo e textura PNG externa.
fn write_gltf(dir: &Path) -> PathBuf {
    let mut bin: Vec<u8> = Vec::new();
    for position in [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ] {
        for value in position {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    for uv in [[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
        for value in uv {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in [0u16, 1, 2, 0, 2, 3] {
        bin.extend_from_slice(&index.to_le_bytes());
    }
    write(dir, "mesh.bin", &bin);
    write(dir, "tex.png", PNG_1X1);
    let json = json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "Quad"}],
        "meshes": [{
            "primitives": [{
                "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                "indices": 2,
                "material": 0
            }]
        }],
        "materials": [{
            "name": "Vermelho",
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": 0},
                "metallicFactor": 0.0,
                "roughnessFactor": 0.8
            }
        }],
        "textures": [{"source": 0, "sampler": 0}],
        "images": [{"uri": "tex.png"}],
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
             "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]},
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC2"},
            {"bufferView": 2, "componentType": 5123, "count": 6, "type": "SCALAR"}
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 48},
            {"buffer": 0, "byteOffset": 48, "byteLength": 32},
            {"buffer": 0, "byteOffset": 80, "byteLength": 12}
        ],
        "buffers": [{"uri": "mesh.bin", "byteLength": bin.len()}]
    });
    write(dir, "quad.gltf", json.to_string().as_bytes())
}

fn import(editor: &mut QtEditor, path: &Path) {
    editor
        .request(&json!({"action": "import_model", "path": path.display().to_string()}))
        .expect("a requisição de importação responde");
}

/// GLB com buffer embutido e textura PNG dentro do arquivo.
fn write_glb(dir: &Path) -> PathBuf {
    let mut bin: Vec<u8> = Vec::new();
    for position in [
        [0.0f32, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ] {
        for value in position {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    for uv in [[0.0f32, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
        for value in uv {
            bin.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in [0u16, 1, 2, 0, 2, 3] {
        bin.extend_from_slice(&index.to_le_bytes());
    }
    let image_offset = bin.len();
    bin.extend_from_slice(PNG_1X1);
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let json = json!({
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "Quad"}],
        "meshes": [{
            "primitives": [{
                "attributes": {"POSITION": 0, "TEXCOORD_0": 1},
                "indices": 2,
                "material": 0
            }]
        }],
        "materials": [{
            "name": "Vermelho",
            "pbrMetallicRoughness": {
                "baseColorTexture": {"index": 0},
                "metallicFactor": 0.0,
                "roughnessFactor": 0.8
            }
        }],
        "textures": [{"source": 0, "sampler": 0}],
        "images": [{"bufferView": 3, "mimeType": "image/png"}],
        "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3",
             "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]},
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC2"},
            {"bufferView": 2, "componentType": 5123, "count": 6, "type": "SCALAR"}
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": 48},
            {"buffer": 0, "byteOffset": 48, "byteLength": 32},
            {"buffer": 0, "byteOffset": 80, "byteLength": 12},
            {"buffer": 0, "byteOffset": image_offset, "byteLength": PNG_1X1.len()}
        ],
        "buffers": [{"byteLength": bin.len()}]
    });
    let mut json_bytes = json.to_string().into_bytes();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut glb: Vec<u8> = Vec::with_capacity(total);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2u32.to_le_bytes());
    glb.extend_from_slice(&(total as u32).to_le_bytes());
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes());
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E4942u32.to_le_bytes());
    glb.extend_from_slice(&bin);
    write(dir, "quad.glb", &glb)
}

const TWO_OBJECTS_OBJ: &str = "\
o Primeiro
v 0 0 0
v 1 0 0
v 1 1 0
f 1 2 3
o Segundo
v 0 0 1
v 1 0 1
v 1 1 1
f 4 5 6
";

#[test]
fn obj_import_brings_the_mesh_into_the_document() {
    let dir = scratch("obj");
    let path = write(&dir, "cube.obj", CUBE_OBJ.as_bytes());
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    import(&mut editor, &path);
    let assets = &editor.state.project.assets;
    assert_eq!(
        assets.len(),
        before + 1,
        "o OBJ deve virar um asset: status {:?}",
        editor.state.ui.status
    );
    assert_eq!(assets[assets.len() - 1].mesh.verts.len(), 8);
    assert_eq!(assets[assets.len() - 1].mesh.faces.len(), 12);
}

#[test]
fn obj_import_keeps_the_material_and_its_texture() {
    let dir = scratch("obj-mtl");
    write(&dir, "cube.mtl", CUBE_MTL.as_bytes());
    write(&dir, "cube.png", PNG_1X1);
    let path = write(&dir, "cube.obj", CUBE_OBJ.as_bytes());
    let mut editor = QtEditor::default();
    let materials_before = editor.state.project.project.materials.len();
    import(&mut editor, &path);
    let materials = &editor.state.project.project.materials;
    assert_eq!(
        materials.len(),
        materials_before + 1,
        "o material do .mtl deve ser importado: status {:?}",
        editor.state.ui.status
    );
    let imported = materials.last().expect("material importado");
    assert!(
        imported.albedo_texture.is_some(),
        "a textura do .mtl deve chegar ao material"
    );
    let asset = editor.state.project.assets.last().expect("asset");
    assert!(
        asset.texture.is_some(),
        "a textura deve espelhar no asset para a viewport"
    );
}

#[test]
fn gltf_import_brings_mesh_material_and_texture() {
    let dir = scratch("gltf");
    let path = write_gltf(&dir);
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    let materials_before = editor.state.project.project.materials.len();
    import(&mut editor, &path);
    let assets = &editor.state.project.assets;
    assert_eq!(
        assets.len(),
        before + 1,
        "o glTF deve virar um asset: status {:?}",
        editor.state.ui.status
    );
    let materials = &editor.state.project.project.materials;
    assert_eq!(
        materials.len(),
        materials_before + 1,
        "o material do glTF deve ser importado"
    );
    assert!(
        materials[materials.len() - 1].albedo_texture.is_some(),
        "a textura do glTF deve ser decodificada"
    );
    assert!(
        assets[assets.len() - 1].texture.is_some(),
        "a textura deve espelhar no asset para a viewport"
    );
}

#[test]
fn glb_import_brings_mesh_material_and_embedded_texture() {
    let dir = scratch("glb");
    let path = write_glb(&dir);
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    let materials_before = editor.state.project.project.materials.len();
    import(&mut editor, &path);
    let assets = &editor.state.project.assets;
    assert_eq!(
        assets.len(),
        before + 1,
        "o GLB deve virar um asset: status {:?}",
        editor.state.ui.status
    );
    let materials = &editor.state.project.project.materials;
    assert_eq!(materials.len(), materials_before + 1);
    assert!(
        materials[materials.len() - 1].albedo_texture.is_some(),
        "a textura embutida do GLB deve ser decodificada"
    );
    assert!(assets[assets.len() - 1].texture.is_some());
}

#[test]
fn an_obj_with_two_objects_brings_two_assets() {
    let dir = scratch("obj-two");
    let path = write(&dir, "dois.obj", TWO_OBJECTS_OBJ.as_bytes());
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    import(&mut editor, &path);
    assert_eq!(
        editor.state.project.assets.len(),
        before + 2,
        "cada objeto do OBJ deve virar um asset: status {:?}",
        editor.state.ui.status
    );
}

#[test]
fn the_palette_command_asks_the_host_to_open_the_dialog() {
    let mut editor = QtEditor::default();
    let before = editor.snapshot()["import_request"].as_u64().unwrap_or(0);
    editor
        .execute_command(&petunia_core::ImportObjCmd)
        .expect("o comando de importação roda");
    let after = editor.snapshot()["import_request"].as_u64().unwrap_or(0);
    assert!(
        after > before,
        "o comando precisa pedir a importação ao host (o host abre o diálogo)"
    );
    assert_eq!(
        editor.snapshot()["import_request_filter"].as_str(),
        Some("obj")
    );
}

#[test]
fn a_file_that_is_not_a_model_is_refused_without_crashing() {
    let dir = scratch("bad");
    let path = write(&dir, "nota-model.obj", b"isto nao e um obj valido\n");
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    import(&mut editor, &path);
    assert_eq!(
        editor.state.project.assets.len(),
        before,
        "nada deve ser importado de um arquivo inválido"
    );
    assert!(
        !editor.state.ui.status.is_empty(),
        "a recusa precisa aparecer no status"
    );
}

#[test]
fn the_import_can_be_undone() {
    let dir = scratch("undo");
    let path = write(&dir, "cube.obj", CUBE_OBJ.as_bytes());
    let mut editor = QtEditor::default();
    let before = editor.state.project.assets.len();
    import(&mut editor, &path);
    assert_eq!(editor.state.project.assets.len(), before + 1);
    assert!(editor.undo(), "a importação precisa ser desfeita");
    assert_eq!(editor.state.project.assets.len(), before);
}
