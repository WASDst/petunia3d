#include "petunia_workspace_sections.h"
#include <algorithm>
#include <cmath>

#include "../petunia_color_picker.h"
#include "../petunia_icons.h"
#include "../petunia_theme.h"
#include "petunia_i18n.h"

#include <QColor>
#include <QComboBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QIcon>
#include <QLabel>
#include <QPixmap>
#include <QPushButton>
#include <QVBoxLayout>

namespace {
QFormLayout *workspaceForm(QWidget *owner) {
    auto *form = new QFormLayout(owner);
    const auto &theme = PetuniaTheme::instance();
    form->setContentsMargins(theme.space(3), theme.space(2), theme.space(3), theme.space(3));
    form->setHorizontalSpacing(theme.space(2));
    form->setVerticalSpacing(theme.space(2));
    form->setRowWrapPolicy(QFormLayout::WrapAllRows);
    form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    return form;
}

QLabel *caption(QWidget *owner, const QString &text) {
    auto *label = new QLabel(text, owner);
    label->setProperty("role", QStringLiteral("caption"));
    label->setWordWrap(true);
    return label;
}
} // namespace

// ── Área dinâmica ───────────────────────────────────────────────────────────

PetuniaDynamicForm::PetuniaDynamicForm(QWidget *parent) : PetuniaBoundForm(parent) {
    m_form = workspaceForm(this);
}

void PetuniaDynamicForm::beginDynamic() {
    m_fixedRows = m_form->rowCount();
}

void PetuniaDynamicForm::afterSync(const QJsonObject &state) {
    if (m_fixedRows < 0)
        beginDynamic();
    const QString signature = dynamicSignature(state);
    if (signature == m_signature)
        return;
    m_signature = signature;
    // Bindings saem antes dos widgets que eles apontam.
    for (int i = int(m_bindings.size()) - 1; i >= 0; --i)
        if (m_bindings[i].widget->property("petuniaDynamic").toBool())
            m_bindings.removeAt(i);
    while (m_form->rowCount() > m_fixedRows)
        m_form->removeRow(m_fixedRows);
    const auto before = m_bindings.size();
    buildDynamic(state);
    for (auto i = before; i < m_bindings.size(); ++i)
        m_bindings[i].widget->setProperty("petuniaDynamic", true);
    // Valores das linhas novas já no mesmo ciclo (a assinatura não muda: sem laço).
    if (m_bindings.size() > before)
        sync(state);
}

// ── PAINT: Alvo ─────────────────────────────────────────────────────────────

PetuniaPaintTargetForm::PetuniaPaintTargetForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    check(form, tr("Pintar cor de vértice"), "paint_target_vertex", "paint_target");
    check(form, tr("Só onde está selecionado"), "paint_mask_selection", "paint_mask");
    choice(form, tr("Restringir a"), "paint_lock", "set_brush_lock", "lock",
           {"None", "FirstObject", "FirstFace", "SelectedFaces"},
           {tr("Sem restrição"), tr("Primeiro objeto"), tr("Primeira face"), tr("Faces selecionadas")});
    choice(form, tr("Preenchimento atinge"), "paint_fill_scope", "set_fill_scope", "scope",
           {"ConnectedPixels", "Face", "SelectedFaces", "UvIsland", "Object"},
           {tr("Pixels conectados"), tr("Face"), tr("Faces selecionadas"), tr("Ilha UV"), tr("Objeto")});
    number(form, tr("Resolução da textura (px)"), "paint_texture_side", "paint_texture_side", "value", 16, 8192,
           16);
    button(form, tr("Verificar UVs para pintura"), "command", {{"id", "paint.uv_health"}});
}

// ── PAINT: Pincel ───────────────────────────────────────────────────────────

PetuniaBrushForm::PetuniaBrushForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    // Cor do pincel sempre visível (ADR-C16, B1): o painel fica embutido e aplica ao
    // vivo. O alfa da cor É a opacidade do pincel (a linha "Opacidade" saiu: A8).
    m_color = new PetuniaColorPanel(this);
    m_color->setAccessibleName(tr("Cor do pincel"));
    form->addRow(m_color);
    connect(m_color, &PetuniaColorPanel::colorChanged, this, [this](const QColor &c) {
        if (m_syncingColor)
            return;
        send(QStringLiteral("paint_color"), {{"r", c.redF()}, {"g", c.greenF()}, {"b", c.blueF()}});
        if (std::abs(c.alphaF() - m_lastOpacity) > 1e-4) {
            m_lastOpacity = c.alphaF();
            send(QStringLiteral("brush_opacity"), {{"value", c.alphaF()}});
        }
    });
    number(form, tr("Tamanho (px)"), "brush_size", "brush_size", "value", 1, 2048, 1);
    number(form, tr("Dureza"), "brush_hardness", "brush_hardness", "value", 0, 1);
    choice(form, tr("Ponta"), "brush_panel.tip", "set_brush_tip", "tip", {"round", "square", "diamond"},
           {tr("Redonda"), tr("Quadrada"), tr("Losango")});
    choice(form, tr("Mistura"), "brush_panel.blend", "set_brush_blend", "blend",
           {"normal", "multiply", "screen", "add", "darken", "lighten"},
           {tr("Normal"), tr("Multiplicar"), tr("Tela"), tr("Somar"), tr("Escurecer"), tr("Clarear")});
    check(form, tr("Simetria X"), "paint_symmetry_x", "set_paint_symmetry_x", "val");
    check(form, tr("Simetria Y"), "paint_symmetry_y", "set_paint_symmetry_y", "val");
    check(form, tr("Simetria Z"), "paint_symmetry_z", "set_paint_symmetry_z", "val");

    form->addRow(caption(this, tr("Avançado")));
    struct Param { const char *name; const char *label; double min; double max; };
    const Param params[] = {
        {"flow", QT_TR_NOOP("Fluxo"), 0, 1},
        {"spacing", QT_TR_NOOP("Espaçamento"), 0, 1},
        {"smoothing", QT_TR_NOOP("Suavização do traço"), 0, 1},
        {"size_jitter", QT_TR_NOOP("Variação de tamanho"), 0, 1},
        {"opacity_jitter", QT_TR_NOOP("Variação de opacidade"), 0, 1},
        {"scatter", QT_TR_NOOP("Dispersão"), 0, 2},
        {"spray_density", QT_TR_NOOP("Densidade do spray"), 0, 1},
        {"angle", QT_TR_NOOP("Ângulo"), -180, 180},
        {"roundness", QT_TR_NOOP("Arredondamento"), 0, 1},
    };
    for (const auto &p : params)
        number(form, tr(p.label), QStringLiteral("brush_panel.") + QLatin1String(p.name), "set_brush_param", "value",
               p.min, p.max, 0.01, {{"name", QLatin1String(p.name)}});

    form->addRow(caption(this, tr("Presets e paleta")));
    rows(form, tr("Presets do pincel"), "brush_presets", "apply_brush_preset", "index");
    text(form, tr("Salvar preset como"), {}, "save_brush_preset", "name");
    button(form, tr("Remover preset selecionado"), "brush_preset_remove");
    button(form, tr("Paleta mais clara"), "command", {{"id", "paint.palette_lighter"}});
    button(form, tr("Paleta mais escura"), "command", {{"id", "paint.palette_darker"}});
    button(form, tr("Paleta normal"), "command", {{"id", "paint.palette_normal"}});
    fileButton(form, tr("Importar paleta…"), "import_palette", tr("Paletas (*.json *.gpl *.ase)"));
    fileButton(form, tr("Exportar paleta…"), "export_palette", tr("Paleta (*.gpl *.json)"), true);
}

void PetuniaBrushForm::afterSync(const QJsonObject &state) {
    const auto rgb = state["paint_color"].toArray();
    if (rgb.size() < 3)
        return;
    const double opacity = std::clamp(state["brush_opacity"].toDouble(1.0), 0.0, 1.0);
    const QColor color = QColor::fromRgbF(float(rgb[0].toDouble()), float(rgb[1].toDouble()), float(rgb[2].toDouble()),
                                          float(opacity));
    m_lastOpacity = opacity;
    if (m_color->color() == color)
        return;
    m_syncingColor = true;
    m_color->setColor(color);
    m_color->setAccessibleDescription(color.name().toUpper());
    m_syncingColor = false;
}

// ── PAINT: Camada ativa ─────────────────────────────────────────────────────

namespace {
int activeLayerIndex(const QJsonObject &state) {
    const auto layers = state["paint_layers"].toArray();
    for (int i = 0; i < layers.size(); ++i)
        if (layers[i].toObject()["active"].toBool())
            return i;
    return -1;
}
} // namespace

PetuniaPaintLayerForm::PetuniaPaintLayerForm(QWidget *parent) : PetuniaDynamicForm(parent) {
    m_title = new QLabel(this);
    m_title->setProperty("role", QStringLiteral("muted"));
    m_title->setWordWrap(true);
    form()->addRow(m_title);
    button(form(), tr("Travar alfa"), "command", {{"id", "paint.alpha_lock"}});
    button(form(), tr("Pixel perfeito"), "command", {{"id", "paint.pixel_perfect"}});
    button(form(), tr("Pontilhado (dithering)"), "command", {{"id", "paint.dithering"}});
    beginDynamic();
}

QString PetuniaPaintLayerForm::dynamicSignature(const QJsonObject &state) const {
    const int index = activeLayerIndex(state);
    QString signature = QString::number(index);
    if (index >= 0) {
        const auto layer = state["paint_layers"].toArray()[index].toObject();
        signature += layer["id"].toString() + layer["kind"].toString() + layer["name"].toString();
    }
    for (const auto &v : state["paint_effect_params"].toArray())
        signature += QLatin1Char('|') + v.toObject()["key"].toString();
    return signature;
}

void PetuniaPaintLayerForm::buildDynamic(const QJsonObject &state) {
    const int index = activeLayerIndex(state);
    if (index < 0) {
        m_title->setText(tr("Nenhuma camada ativa. Crie ou selecione uma camada na Estrutura."));
        return;
    }
    const auto layer = state["paint_layers"].toArray()[index].toObject();
    const QString id = layer["id"].toString();
    m_title->setText(tr("Camada: %1 · %2").arg(layer["name"].toString(), layer["kind_label"].toString()));
    const QString prefix = QStringLiteral("paint_layers.%1.").arg(index);
    number(form(), tr("Opacidade da camada"), prefix + "opacity", "set_paint_layer_opacity", "opacity", 0, 1, 0.01,
           {{"id", id}});
    choice(form(), tr("Modo de mistura"), prefix + "blend", "set_paint_layer_blend", "blend",
           {"0", "1", "2", "3", "4", "5"},
           {tr("Normal"), tr("Multiplicar"), tr("Tela"), tr("Somar"), tr("Escurecer"), tr("Clarear")}, {{"id", id}});
    int param = 0;
    for (const auto &v : state["paint_effect_params"].toArray()) {
        const auto o = v.toObject();
        number(form(), petuniaTr(o["label"].toString()),
               QStringLiteral("paint_effect_params.%1.value").arg(param++), "set_paint_effect_param", "value",
               o["min"].toDouble(), o["max"].toDouble(), 0.01, {{"key", o["key"]}});
    }
}

// ── PAINT: Efeitos ──────────────────────────────────────────────────────────

PetuniaPaintEffectsForm::PetuniaPaintEffectsForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    form->addRow(caption(this, tr("Adiciona uma camada de ajuste não destrutiva acima da camada ativa.")));
    const struct { const char *kind; const char *label; } effects[] = {
        {"HueSaturation", QT_TR_NOOP("Matiz e saturação")},
        {"Levels", QT_TR_NOOP("Níveis")},
        {"BrightnessContrast", QT_TR_NOOP("Brilho e contraste")},
        {"Invert", QT_TR_NOOP("Inverter cores")},
        {"Grain", QT_TR_NOOP("Grão")},
        {"Pixelate", QT_TR_NOOP("Pixelizar")},
        {"Posterize", QT_TR_NOOP("Posterizar")},
    };
    for (const auto &effect : effects)
        button(form, tr(effect.label), "add_paint_effect_layer", {{"kind", QLatin1String(effect.kind)}});
}

// ── PAINT: Projeção e superfície ────────────────────────────────────────────

PetuniaPaintProjectionForm::PetuniaPaintProjectionForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    choice(form, tr("Projeção do pincel"), "paint_projection", "set_brush_projection", "projection",
           {"Surface", "ScreenSpace"}, {tr("Na superfície"), tr("Da tela")});

    form->addRow(caption(this, tr("Traçados e estêncil")));
    fileButton(form, tr("Importar SVG…"), "import_svg", tr("SVG (*.svg)"));
    const struct { const char *id; const char *label; } commands[] = {
        {"paint.svg_resolution_512", QT_TR_NOOP("SVG em 512 px")},
        {"paint.svg_resolution_1024", QT_TR_NOOP("SVG em 1024 px")},
        {"paint.path_paint", QT_TR_NOOP("Pintar ao longo do caminho")},
        {"paint.path_ribbon", QT_TR_NOOP("Faixa ao longo do caminho")},
        {"paint.path_stamps", QT_TR_NOOP("Carimbos ao longo do caminho")},
        {"paint.stencil", QT_TR_NOOP("Usar como estêncil")},
        {"paint.stencil_luma", QT_TR_NOOP("Estêncil por luminância")},
        {"paint.stencil_place", QT_TR_NOOP("Posicionar estêncil")},
        {"paint.project_decal", QT_TR_NOOP("Projetar como decalque")},
        {"paint.projection_mirror", QT_TR_NOOP("Espelhar projeção")},
        {"paint.projection_rotate", QT_TR_NOOP("Girar projeção")},
        {"paint.projection_grow", QT_TR_NOOP("Aumentar projeção")},
        {"paint.projection_shrink", QT_TR_NOOP("Reduzir projeção")},
        {"paint.apply_surface", QT_TR_NOOP("Aplicar na superfície")},
        {"paint.cancel_surface", QT_TR_NOOP("Cancelar projeção")},
    };
    for (const auto &c : commands)
        button(form, tr(c.label), "command", {{"id", QLatin1String(c.id)}});

    form->addRow(caption(this, tr("Imagens de referência")));
    button(form, tr("Projetar referência na textura"), "project_from_reference");
    button(form, tr("Bake da referência"), "bake_reference");
    // Escolha local (não é estado do documento): qual imagem os passos abaixo ajustam.
    auto *axis = new QComboBox(this);
    axis->setAccessibleName(tr("Imagem de referência ajustada"));
    const QStringList axes{"front", "back", "left", "right", "top", "bottom"};
    const QStringList axisLabels{tr("Frente"), tr("Trás"), tr("Esquerda"), tr("Direita"), tr("Topo"), tr("Base")};
    for (int i = 0; i < axes.size(); ++i)
        axis->addItem(axisLabels[i], axes[i]);
    connect(axis, &QComboBox::currentIndexChanged, this,
            [this, axis] { m_referenceAxis = axis->currentData().toString(); });
    form->addRow(tr("Imagem"), axis);
    const struct { const char *label; const char *param; double delta; double min; double max; double base; } steps[] = {
        {QT_TR_NOOP("Mover imagem"), "offset", 0.1, -50.0, 50.0, 0.0},
        {QT_TR_NOOP("Escalonar imagem"), "size", 0.1, 0.1, 50.0, 1.0},
        {QT_TR_NOOP("Rotacionar imagem"), "rotation", 5.0, -180.0, 180.0, 0.0},
    };
    for (const auto &step : steps) {
        auto *row = new QWidget(this);
        auto *layout = new QHBoxLayout(row);
        layout->setContentsMargins(0, 0, 0, 0);
        for (const double sign : {-1.0, 1.0}) {
            const QString text = sign < 0 ? QStringLiteral("−") : QStringLiteral("+");
            auto *stepButton = new QPushButton(text, row);
            stepButton->setAccessibleName(tr(step.label) + (sign < 0 ? tr(" (diminuir)") : tr(" (aumentar)")));
            stepButton->setToolTip(stepButton->accessibleName());
            const QString param = QLatin1String(step.param);
            connect(stepButton, &QPushButton::clicked, this, [this, param, sign, step] {
                const auto slot = state()[QStringLiteral("reference_") + m_referenceAxis].toObject();
                const double current = slot.value(param).toDouble(step.base);
                send(QStringLiteral("set_reference_param"),
                     {{"axis_str", m_referenceAxis}, {"param", param},
                      {"val", qBound(step.min, current + sign * step.delta, step.max)}});
            });
            layout->addWidget(stepButton);
        }
        form->addRow(tr(step.label), row);
    }
}

// ── PAINT: Decalque ─────────────────────────────────────────────────────────

PetuniaDecalForm::PetuniaDecalForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    form->addRow(caption(this, tr("Ativo quando a camada selecionada é um decalque.")));
    fileButton(form, tr("Importar imagem do decalque…"), "import_decal", tr("Imagem (*.png *.jpg *.jpeg *.webp *.bmp)"));
    choice(form, tr("Arrastar na vista"), "decal_gizmo_mode", "decal_gizmo_mode", "id",
           {"auto", "move", "rotate", "scale"},
           {tr("Automático (alças)"), tr("Mover"), tr("Rotacionar"), tr("Escalonar")});
    const struct { const char *field; const char *label; double min; double max; } fields[] = {
        {"center_u", QT_TR_NOOP("Centro U"), -10, 10},
        {"center_v", QT_TR_NOOP("Centro V"), -10, 10},
        {"scale_u", QT_TR_NOOP("Escala U"), -10, 10},
        {"scale_v", QT_TR_NOOP("Escala V"), -10, 10},
        {"rotation_deg", QT_TR_NOOP("Rotação (°)"), -360, 360},
    };
    for (const auto &f : fields)
        number(form, tr(f.label), QStringLiteral("decal_") + QLatin1String(f.field), "decal_transform_field", "value",
               f.min, f.max, 0.01, {{"field", QLatin1String(f.field)}});
    button(form, tr("Aplicar decalque na textura (bake)"), "bake_active_decal", {}, "active_layer_is_decal");
    disabledReason(tr("Selecione uma camada de decalque na Estrutura"));

    form->addRow(caption(this, tr("Variantes e animação")));
    fileButton(form, tr("Adicionar variante…"), "decal_variant_add", tr("Imagem (*.png *.jpg *.jpeg *.webp *.bmp)"));
    rows(form, tr("Variantes"), "decal_variants", "decal_variant", "index");
    button(form, tr("Remover variante"), "decal_variant_remove");
    number(form, tr("Quadro"), "decal_frame", "decal_frame", "value", 0, 100000, 1);
    number(form, tr("Duração (quadros)"), "decal_track_length", "decal_length", "value", 1, 100000, 1);
    number(form, tr("Quadros por segundo"), "decal_track_fps", "decal_fps", "value", 1, 240, 1);
    button(form, tr("Inserir key do decalque"), "decal_key_add");
    button(form, tr("Remover key do decalque"), "decal_key_remove");
    button(form, tr("Tocar/pausar decalque"), "decal_play");
    fileButton(form, tr("Exportar quadros…"), "decal_export", tr("PNG (*.png)"), true);
}

// ── PAINT: Canvas ───────────────────────────────────────────────────────────

PetuniaPaintCanvasForm::PetuniaPaintCanvasForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    check(form, tr("Canvas principal com 3D em miniatura"), "paint_pip", "toggle_paint_pip", {});
    check(form, tr("Grade de pixels"), "paint_pixel_grid", "paint_grid", {});
    check(form, tr("UVs sobre a pintura"), "paint_show_uv_overlay", "paint_uv_overlay", {});
    number(form, tr("Zoom do canvas"), "paint_canvas_zoom", "paint_canvas_zoom", "value", 1, 16, 1);
}

// ── UV ──────────────────────────────────────────────────────────────────────

PetuniaUvSelectionForm::PetuniaUvSelectionForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    choice(form, tr("Selecionar"), "uv_selection_mode", "uv_selection_mode", "id",
           {"vertex", "edge", "face", "island"}, {tr("Vértices"), tr("Arestas"), tr("Faces"), tr("Ilhas")});
    check(form, tr("Sincronizar com a seleção 3D"), "uv_selection_sync", "uv_selection_sync");
    m_stats = new QLabel(this);
    m_stats->setProperty("role", QStringLiteral("muted"));
    m_stats->setWordWrap(true);
    m_stats->setTextInteractionFlags(Qt::TextSelectableByMouse | Qt::TextSelectableByKeyboard);
    m_stats->setAccessibleName(tr("Resumo dos UVs"));
    form->addRow(m_stats);
}

void PetuniaUvSelectionForm::afterSync(const QJsonObject &state) {
    const auto editor = state["uv_editor"].toObject();
    const QString text = tr("%1 ilhas · %2 faces · %3 selecionados · %4 fixados")
                             .arg(editor["island_count"].toVariant().toString(),
                                  editor["face_count"].toVariant().toString(),
                                  editor["uv_selected_count"].toVariant().toString(),
                                  editor["pinned_count"].toVariant().toString());
    if (m_stats->text() != text)
        m_stats->setText(text);
}

PetuniaUvUnwrapForm::PetuniaUvUnwrapForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    button(form, tr("Desdobrar automaticamente"), "command", {{"id", "uv.unwrap_auto"}});
    button(form, tr("Projetar da vista"), "command", {{"id", "uv.project_view"}});
    button(form, tr("Projeção planar"), "command", {{"id", "uv.project_planar"}});
    button(form, tr("Projeção em cubo"), "command", {{"id", "uv.project_cube"}});
    button(form, tr("Projetar da referência"), "project_from_reference");
}

PetuniaUvSeamsForm::PetuniaUvSeamsForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    button(form, tr("Marcar ou remover costura na seleção"), "toggle_selected_uv_seams");
    button(form, tr("Limpar todas as costuras"), "clear_all_uv_seams");
    button(form, tr("Fixar ou soltar pins na seleção"), "toggle_selected_uv_pins");
    button(form, tr("Limpar todos os pins"), "clear_all_uv_pins");
    button(form, tr("Separar ilha"), "command", {{"id", "uv.split"}});
}

PetuniaUvPackForm::PetuniaUvPackForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    button(form, tr("Empacotar ilhas"), "command", {{"id", "uv.pack_islands"}});
    button(form, tr("Relaxar"), "command", {{"id", "uv.relax"}});
    button(form, tr("Costurar (stitch)"), "command", {{"id", "uv.stitch"}});
    button(form, tr("Igualar densidade de texels"), "uv_equalize_texel_density");
}

PetuniaUvTransformForm::PetuniaUvTransformForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    form->addRow(caption(this, tr("Valores relativos aplicados à seleção UV.")));
    number(form, tr("Mover em U"), {}, "uv_move_selected", "du", -1000, 1000, 0.01, {{"dv", 0.}});
    number(form, tr("Mover em V"), {}, "uv_move_selected", "dv", -1000, 1000, 0.01, {{"du", 0.}});
    number(form, tr("Escalar por"), {}, "uv_scale_selected", "factor", 0.001, 1000, 0.01);
    number(form, tr("Girar (°)"), {}, "uv_rotate_selected", "degrees", -360, 360, 1);
}

PetuniaUvDisplayForm::PetuniaUvDisplayForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    check(form, tr("Xadrez de verificação"), "show_uv_checker", "uv_checker", {});
    check(form, tr("Mostrar textura"), "uv_show_texture", "uv_texture", {});
}

// ── ANIMATE ─────────────────────────────────────────────────────────────────

PetuniaCreatureForm::PetuniaCreatureForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    rows(form, tr("Adicionar criatura"), "animate.creature_presets", "animate", "arg", {{"id", "add-creature"}});
    rows(form, tr("Criaturas do projeto"), "animate.creatures", "animate", "arg", {{"id", "select-creature"}});
    button(form, tr("Ajustar e ligar ao modelo"), "animate", {{"id", "fit-to-model"}}, "animate.can_fit");
    disabledReason(tr("Adicione uma criatura e selecione o modelo que ela vai animar"));
    check(form, tr("Mostrar ossos"), "animate.show_bones", "animate", {}, {{"id", "toggle-bones"}});
}

PetuniaMotionForm::PetuniaMotionForm(QWidget *parent) : PetuniaDynamicForm(parent) {
    auto *f = form();
    rows(f, tr("Adicionar Motion"), "animate.catalog", "animate", "arg", {{"id", "add-motion"}});
    rows(f, tr("Motions do projeto"), "animate.motions", "animate", "arg", {{"id", "select-motion"}});
    rows(f, tr("Estilo"), "animate.styles", "animate", "arg", {{"id", "set-style"}});
    check(f, tr("Quadros em degraus (stepped)"), "animate.stepped", "animate", "value", {{"id", "set-stepped"}});
    check(f, tr("Movimento da raiz"), "animate.root_motion", "animate", "value", {{"id", "set-root-motion"}});
    const QString needsMotion = tr("Adicione ou selecione um Motion primeiro");
    button(f, tr("Duplicar Motion"), "animate", {{"id", "duplicate-motion"}}, "animate.has_motion");
    disabledReason(needsMotion);
    button(f, tr("Remover Motion"), "animate", {{"id", "remove-motion"}}, "animate.has_motion");
    disabledReason(needsMotion);
    button(f, tr("Bake e manter o Motion"), "animate", {{"id", "apply-now"}, {"arg", "keep"}}, "animate.has_motion");
    disabledReason(needsMotion);
    button(f, tr("Bake e substituir o Motion"), "animate", {{"id", "apply-now"}, {"arg", "replace"}},
           "animate.has_motion");
    disabledReason(needsMotion);
    check(f, tr("Mostrar parâmetros avançados"), "animate.show_advanced", "animate", {}, {{"id", "toggle-advanced"}});
    f->addRow(caption(this, tr("Parâmetros do Motion")));
    beginDynamic();
}

QString PetuniaMotionForm::dynamicSignature(const QJsonObject &state) const {
    QString signature;
    for (const auto &v : state["animate"].toObject()["params"].toArray()) {
        const auto o = v.toObject();
        signature += o["key"].toString() + o["label"].toString() + o["choices"].toVariant().toString() + QLatin1Char('|');
    }
    return signature;
}

void PetuniaMotionForm::buildDynamic(const QJsonObject &state) {
    int index = 0;
    for (const auto &v : state["animate"].toObject()["params"].toArray()) {
        const auto o = v.toObject();
        const QString key = o["key"].toString();
        const QString prefix = QStringLiteral("animate.params.%1.").arg(index++);
        const auto choices = o["choices"].toArray();
        if (!choices.isEmpty()) {
            QStringList ids, labels;
            for (int i = 0; i < choices.size(); ++i) {
                ids << QString::number(i);
                labels << petuniaTr(choices[i].toString());
            }
            choice(form(), petuniaTr(o["label"].toString()), prefix + "choice_index", "animate", "value", ids, labels,
                   {{"id", "param-set"}, {"arg", key}});
        } else {
            number(form(), petuniaTr(o["label"].toString()), prefix + "value", "animate", "value",
                   o["minimum"].toDouble(), o["maximum"].toDouble(), 0.01, {{"id", "param-set"}, {"arg", key}});
        }
        if (!m_bindings.isEmpty())
            m_bindings.last().widget->setToolTip(o["tip"].toString());
    }
}

PetuniaPoseForm::PetuniaPoseForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    form->addRow(caption(this, tr("Selecione um osso na Estrutura ou na vista.")));
    check(form, tr("Cinemática inversa (IK)"), "animate.ik_mode", "animate", "value", {{"id", "set-ik-mode"}});
    button(form, tr("Resetar pose do osso"), "animate", {{"id", "reset-bone"}}, "animate.can_keyframe");
    disabledReason(tr("Selecione um osso de uma criatura primeiro"));
    xyz(form, tr("Posição"), "bone_translation", "bone_transform", "value", {{"kind", "translation"}});
    xyz(form, tr("Rotação"), "bone_rotation", "bone_transform", "value", {{"kind", "rotation"}});
    xyz(form, tr("Escala"), "bone_scale", "bone_transform", "value", {{"kind", "scale"}});
}

PetuniaAnimationLayersForm::PetuniaAnimationLayersForm(QWidget *parent) : PetuniaDynamicForm(parent) {
    button(form(), tr("Nova camada de animação"), "animate", {{"id", "add-motion-layer"}});
    button(form(), tr("Nova camada de ajuste"), "animate", {{"id", "add-tweak-layer"}});
    beginDynamic();
}

QString PetuniaAnimationLayersForm::dynamicSignature(const QJsonObject &state) const {
    QString signature;
    for (const auto &v : state["animate"].toObject()["layers"].toArray())
        signature += v.toObject()["id"].toString() + v.toObject()["name"].toString() + QLatin1Char('|');
    return signature;
}

void PetuniaAnimationLayersForm::buildDynamic(const QJsonObject &state) {
    int index = 0;
    for (const auto &v : state["animate"].toObject()["layers"].toArray()) {
        const auto o = v.toObject();
        const QString id = o["id"].toString();
        const QString prefix = QStringLiteral("animate.layers.%1.").arg(index++);
        check(form(), o["name"].toString(), prefix + "enabled", "animate", {}, {{"id", "toggle-layer-enabled"}, {"arg", id}});
        number(form(), tr("Peso de %1").arg(o["name"].toString()), prefix + "weight", "animate", "value", 0, 1, 0.01,
               {{"id", "set-layer-weight"}, {"arg", id}});
        check(form(), tr("Aditiva"), prefix + "is_additive", "animate", "value",
              {{"id", "set-layer-blend-mode"}, {"arg", id}});
        button(form(), tr("Remover %1").arg(o["name"].toString()), "animate", {{"id", "remove-motion-layer"}, {"arg", id}});
    }
}

PetuniaTimelineForm::PetuniaTimelineForm(QWidget *parent) : PetuniaBoundForm(parent) {
    auto *form = workspaceForm(this);
    number(form, tr("Posição no ciclo"), "animate.progress", "animate", "value", 0, 1, 0.01, {{"id", "seek"}});
    choice(form, tr("Interpolação do keyframe"), "animate.interpolation", "animate", "arg", {"step", "linear", "cubic"},
           {tr("Degrau"), tr("Linear"), tr("Cúbica")}, {{"id", "set-keyframe-interp"}});
    button(form, tr("Limpar trilha do osso"), "animate", {{"id", "clear-bone-track"}}, "animate.can_keyframe");
    disabledReason(tr("Selecione um osso de uma criatura primeiro"));
    check(form, tr("Editor de curvas"), "animate.curve_mode", "animate", "value", {{"id", "set-curve-mode"}});
    choice(form, tr("Canal da curva"), "animate.curve_channel", "animate", "arg",
           {"trans_x", "trans_y", "trans_z", "rot_x", "rot_y", "rot_z", "scale"},
           {tr("Posição X"), tr("Posição Y"), tr("Posição Z"), tr("Rotação X"), tr("Rotação Y"), tr("Rotação Z"),
            tr("Escala")},
           {{"id", "set-curve-channel"}});
    check(form, tr("Onion skin"), "animate.onion_skin_enabled", "animate", "value", {{"id", "toggle-onion-skin"}});
    check(form, tr("Rastros de movimento"), "animate.motion_trails_enabled", "animate", "value",
          {{"id", "toggle-motion-trails"}});
}

// ── Página da linha do tempo ────────────────────────────────────────────────

PetuniaTimelinePage::PetuniaTimelinePage(PetuniaShellController *controller, QWidget *canvas, QWidget *parent)
    : QWidget(parent), m_controller(controller) {
    const auto &theme = PetuniaTheme::instance();
    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);
    auto *bar = new QHBoxLayout();
    bar->setContentsMargins(theme.space(2), theme.space(1), theme.space(2), theme.space(1));
    bar->setSpacing(theme.space(1));
    auto transport = [this, bar](const QString &icon, const QString &tip, const QJsonObject &args) {
        auto *button = new QPushButton(PetuniaIcons::get(icon), QString(), this);
        button->setProperty("role", QStringLiteral("quiet"));
        button->setFixedSize(PetuniaTheme::instance().minHitTarget(), PetuniaTheme::instance().minHitTarget());
        button->setToolTip(tip);
        button->setAccessibleName(tip);
        connect(button, &QPushButton::clicked, this,
                [this, args] { m_controller->request(QStringLiteral("animate"), args); });
        bar->addWidget(button);
        return button;
    };
    transport(QStringLiteral("jump_start"), tr("Primeiro quadro"), {{"id", "jump-start"}});
    transport(QStringLiteral("step_backward"), tr("Quadro anterior"), {{"id", "step-frame"}, {"value", -1}});
    m_play = transport(QStringLiteral("play"), tr("Tocar ou pausar (Espaço)"), {{"id", "toggle-play"}});
    transport(QStringLiteral("step_forward"), tr("Próximo quadro"), {{"id", "step-frame"}, {"value", 1}});
    transport(QStringLiteral("jump_end"), tr("Último quadro"), {{"id", "jump-end"}});
    m_key = transport(QStringLiteral("keyframe"), tr("Inserir keyframe no osso"), {{"id", "insert-keyframe"}});
    transport(QStringLiteral("trash"), tr("Remover keyframe"), {{"id", "delete-keyframe"}, {"value", -1}});
    m_frame = new QLabel(this);
    m_frame->setProperty("role", QStringLiteral("muted"));
    m_frame->setAccessibleName(tr("Quadro atual"));
    bar->addSpacing(theme.space(2));
    bar->addWidget(m_frame);
    bar->addStretch(1);
    root->addLayout(bar);
    if (canvas) {
        canvas->setParent(this);
        root->addWidget(canvas, 1);
        canvas->show();
    }
}

void PetuniaTimelinePage::sync(const QJsonObject &state) {
    const auto animate = state["animate"].toObject();
    const bool playing = animate["playing"].toBool();
    m_play->setIcon(PetuniaIcons::get(playing ? QStringLiteral("pause") : QStringLiteral("play")));
    m_play->setAccessibleDescription(playing ? tr("Tocando") : tr("Parado"));
    const bool canKey = animate["can_keyframe"].toBool();
    m_key->setEnabled(canKey);
    m_key->setToolTip(canKey ? tr("Inserir keyframe no osso")
                             : tr("Inserir keyframe no osso — selecione um osso de uma criatura primeiro"));
    const QString frame = tr("Quadro %1 de %2")
                              .arg(animate["current_frame"].toInt())
                              .arg(animate["total_frames"].toInt());
    if (m_frame->text() != frame)
        m_frame->setText(frame);
}
