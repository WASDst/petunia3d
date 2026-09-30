//! Matemática de cena compartilhada pelos backends GL e wgpu:
//! grid, quads de referência e constantes de luz. Uma fonte só.

/// Direção da luz + ambiente (lambert simples).
pub const LIGHT_DIR: [f32; 3] = [0.5, 0.9, 0.6];
pub const LIGHT_AMBIENT: f32 = 0.45;
pub const LIGHT_DIFFUSE: f32 = 0.65;

/// Luz de estúdio presa à câmera (Plasticity, Cinema 4D): por cima do ombro
/// esquerdo de quem olha, para as faces de frente ficarem claras e as
/// laterais darem leitura de volume em qualquer órbita.
pub fn studio_light_for_camera(camera: &petunia_core::Camera) -> [f32; 3] {
    let direction = -camera.forward() * 0.75 + camera.up() * 0.55 - camera.right() * 0.35;
    let direction = direction.normalize_or_zero();
    if direction.length_squared() < 0.5 {
        LIGHT_DIR
    } else {
        direction.to_array()
    }
}

/// Cor de seleção (laranja Blender-like).
pub const SELECT_COLOR: [f32; 3] = [1.0, 0.55, 0.15];
/// Cor de aresta selecionada (overlay).
pub const SELECT_EDGE_COLOR: [f32; 3] = [1.0, 0.35, 0.1];

/// Segmento de linha com cor: (a, b, cor).
pub type ColoredLine = ([f32; 3], [f32; 3], [f32; 3]);

/// Linhas dos eixos mundiais cartesianos (X=vermelho, Y=verde, Z=azul).
pub fn world_axes_lines(extent: f32) -> Vec<ColoredLine> {
    let axis_x = AXIS_X;
    let axis_y = AXIS_Y;
    let axis_z = AXIS_Z;
    vec![
        ([-extent, 0.0, 0.0], [extent, 0.0, 0.0], axis_x),
        ([0.0, -extent, 0.0], [0.0, extent, 0.0], axis_y),
        ([0.0, 0.0, -extent], [0.0, 0.0, extent], axis_z),
    ]
}

/// Grid estilo Blender no plano XZ com suporte a tamanho, subdivisões, opacidade e guia isométrico.
pub fn grid_lines_custom(
    size: f32,
    spacing: f32,
    opacity: f32,
    show_iso: bool,
    iso_angle_deg: f32,
) -> Vec<ColoredLine> {
    let mut v = Vec::new();
    let extent = size.max(1.0);
    let step = spacing.clamp(0.1, extent);
    let op = opacity.clamp(0.05, 1.0);

    // Hierarquia do grid: cada 10ª linha é "major" e recebe mais contraste. É
    // isso que dá noção de escala sem o grid competir com o objeto.
    let minor = [0.16 * op, 0.17 * op, 0.20 * op];
    let major = [0.28 * op, 0.30 * op, 0.34 * op];

    let steps = (extent / step).ceil() as i32;
    for i in -steps..=steps {
        let f = i as f32 * step;
        if f.abs() > extent + 1e-4 {
            continue;
        }
        let color = if i % 10 == 0 { major } else { minor };
        v.push(([f, 0.0, -extent], [f, 0.0, extent], color));
        v.push(([-extent, 0.0, f], [extent, 0.0, f], color));
    }

    if show_iso {
        let iso_color = [0.18 * op * 2.5, 0.42 * op * 2.5, 0.52 * op * 2.5];
        let angle_rad = iso_angle_deg.to_radians();
        let tan_a = angle_rad.tan().abs().max(0.1);
        let iso_step = step * 2.0;
        let iso_steps = (extent * 2.0 / iso_step).ceil() as i32;
        for i in -iso_steps..=iso_steps {
            let offset = i as f32 * iso_step;
            let z0 = -extent * tan_a + offset;
            let z1 = extent * tan_a + offset;
            if (z0 >= -extent && z0 <= extent) || (z1 >= -extent && z1 <= extent) {
                let cz0 = z0.clamp(-extent, extent);
                let cx0 = (cz0 - offset) / tan_a;
                let cz1 = z1.clamp(-extent, extent);
                let cx1 = (cz1 - offset) / tan_a;
                v.push(([cx0, 0.0, cz0], [cx1, 0.0, cz1], iso_color));
            }

            let z0_neg = extent * tan_a + offset;
            let z1_neg = -extent * tan_a + offset;
            if (z0_neg >= -extent && z0_neg <= extent) || (z1_neg >= -extent && z1_neg <= extent) {
                let cz0 = z0_neg.clamp(-extent, extent);
                let cx0 = -(cz0 - offset) / tan_a;
                let cz1 = z1_neg.clamp(-extent, extent);
                let cx1 = -(cz1 - offset) / tan_a;
                v.push(([cx0, 0.0, cz0], [cx1, 0.0, cz1], iso_color));
            }
        }
    }

    // Eixos por último, com peso próprio: são a referência mais forte do grid
    // e precisam vencer as linhas sem dominar o objeto.
    let axis_extent = extent;
    v.push((
        [-axis_extent, 0.0, 0.0],
        [axis_extent, 0.0, 0.0],
        [AXIS_X[0] * 0.62, AXIS_X[1] * 0.62, AXIS_X[2] * 0.62],
    ));
    v.push((
        [0.0, 0.0, -axis_extent],
        [0.0, 0.0, axis_extent],
        [AXIS_Z[0] * 0.62, AXIS_Z[1] * 0.62, AXIS_Z[2] * 0.62],
    ));

    v
}

/// Cores canônicas dos eixos (X vermelho, Y verde, Z azul).
pub const AXIS_X: [f32; 3] = [0.88, 0.24, 0.26];
pub const AXIS_Y: [f32; 3] = [0.38, 0.79, 0.20];
pub const AXIS_Z: [f32; 3] = [0.19, 0.51, 0.96];

/// Grid estilo Blender no plano XZ (semi-eixo 20).
pub fn grid_lines() -> Vec<ColoredLine> {
    grid_lines_custom(20.0, 1.0, 0.4, false, 30.0)
}

/// Eixo do plano de referência.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefPlane {
    Front,
    Back,
    Left,
    Right,
    Side,
    Top,
    Bottom,
}

/// Quad da imagem de referência (4 cantos) no mundo com rotação opcional em graus.
/// `aspect = width / height`.
pub fn ref_quad(plane: RefPlane, offset: f32, size: f32, aspect: f32) -> [[f32; 3]; 4] {
    ref_quad_with_rot(plane, offset, size, aspect, 0.0)
}

/// Quad da imagem de referência com rotação no plano em graus.
pub fn ref_quad_with_rot(
    plane: RefPlane,
    offset: f32,
    size: f32,
    aspect: f32,
    rotation_deg: f32,
) -> [[f32; 3]; 4] {
    let h = size * 0.5;
    let w = h * aspect;
    let rad = rotation_deg.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();
    let rot2 = |x: f32, y: f32| (x * cos_r - y * sin_r, x * sin_r + y * cos_r);

    let (p0, p1, p2, p3) = (rot2(-w, -h), rot2(w, -h), rot2(w, h), rot2(-w, h));

    match plane {
        RefPlane::Front => [
            [p0.0, p0.1, offset],
            [p1.0, p1.1, offset],
            [p2.0, p2.1, offset],
            [p3.0, p3.1, offset],
        ],
        RefPlane::Back => [
            [-p0.0, p0.1, -offset],
            [-p1.0, p1.1, -offset],
            [-p2.0, p2.1, -offset],
            [-p3.0, p3.1, -offset],
        ],
        RefPlane::Right | RefPlane::Side => [
            [offset, p0.1, -p0.0],
            [offset, p1.1, -p1.0],
            [offset, p2.1, -p2.0],
            [offset, p3.1, -p3.0],
        ],
        RefPlane::Left => [
            [-offset, p0.1, p0.0],
            [-offset, p1.1, p1.0],
            [-offset, p2.1, p2.0],
            [-offset, p3.1, p3.0],
        ],
        RefPlane::Top => [
            [p0.0, offset, -p0.1],
            [p1.0, offset, -p1.1],
            [p2.0, offset, -p2.1],
            [p3.0, offset, -p3.1],
        ],
        RefPlane::Bottom => [
            [p0.0, -offset, p0.1],
            [p1.0, -offset, p1.1],
            [p2.0, -offset, p2.1],
            [p3.0, -offset, p3.1],
        ],
    }
}

/// UVs padrão do quad (origem em cima, como RGBA de `image`).
pub const QUAD_UVS_TOP_LEFT: [[f32; 2]; 4] = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

/// UVs com V flipado (origem GL embaixo).
pub const QUAD_UVS_GL: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

#[cfg(test)]
mod grid_hierarchy_tests {
    use super::*;

    fn is_axis(line: &ColoredLine) -> bool {
        let (a, b, c) = line;
        // Um eixo tem uma componente zerada nos dois extremos e cor saturada.
        (a[0] == 0.0 && b[0] == 0.0)
            || (a[2] == 0.0 && b[2] == 0.0) && c.iter().any(|channel| *channel > 0.2)
    }

    #[test]
    fn the_grid_has_minor_and_major_levels() {
        let lines = grid_lines_custom(20.0, 1.0, 0.5, false, 30.0);
        let brightness: Vec<f32> = lines.iter().map(|(_, _, c)| c[0] + c[1] + c[2]).collect();
        let darkest = brightness.iter().cloned().fold(f32::MAX, f32::min);
        let brightest = brightness.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            brightest > darkest * 1.4,
            "linhas major precisam se destacar das minor: {darkest} vs {brightest}"
        );
    }

    #[test]
    fn the_grid_never_outshines_the_selection_colour() {
        // A hierarquia exigida: seleção muito mais forte que grid.
        let lines = grid_lines_custom(20.0, 1.0, 1.0, false, 30.0);
        for (_, _, color) in &lines {
            let luminance = color[0] + color[1] + color[2];
            assert!(
                luminance < 1.4,
                "nenhuma linha de grid pode chegar perto da seleção: {color:?}"
            );
        }
        let select = crate::scene::SELECT_COLOR;
        let select_luminance = select[0] + select[1] + select[2];
        assert!(select_luminance > 1.5, "seleção precisa ser quente e forte");
    }

    #[test]
    fn the_axis_lines_are_present_and_toned_down() {
        let lines = grid_lines_custom(20.0, 1.0, 0.4, false, 30.0);
        let axes: Vec<&ColoredLine> = lines.iter().filter(|line| is_axis(line)).collect();
        assert!(
            axes.len() >= 2,
            "X e Z precisam existir, veio {}",
            axes.len()
        );
        for (_, _, color) in axes {
            let luminance = color[0] + color[1] + color[2];
            assert!(luminance < 1.6, "eixo não pode dominar o objeto: {color:?}");
        }
    }
}
