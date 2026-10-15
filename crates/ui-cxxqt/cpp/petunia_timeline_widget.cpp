#include "petunia_timeline_widget.h"
#include "petunia_icons.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QPainter>
#include <QPainterPath>
#include <QPolygonF>

extern "C" {
    bool petunia_qt_app_get_timeline_info(int *current_frame, int *total_frames, bool *is_playing);
    void petunia_qt_app_set_timeline_frame(int frame);
    bool petunia_qt_app_toggle_playback();
    bool petunia_qt_app_has_keyframe(int asset_index, int frame);
    bool petunia_qt_app_insert_keyframe(int asset_index, int frame);
    bool petunia_qt_app_delete_keyframe(int asset_index, int frame);
    int petunia_qt_app_get_active_asset();
}

// =========================================================================
// PetuniaDopesheetCanvas
// =========================================================================

PetuniaDopesheetCanvas::PetuniaDopesheetCanvas(QWidget *parent)
    : QWidget(parent)
{
    setMinimumHeight(110);
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    setMouseTracking(true);
}

void PetuniaDopesheetCanvas::setCurrentFrame(int frame) {
    if (m_currentFrame != frame) {
        m_currentFrame = qBound(1, frame, m_totalFrames);
        update();
    }
}

void PetuniaDopesheetCanvas::setStartFrame(int start) {
    int s = qBound(1, start, m_totalFrames);
    if (m_startFrame != s) {
        m_startFrame = s;
        if (m_endFrame < m_startFrame) {
            m_endFrame = m_startFrame;
        }
        update();
        emit playbackRangeChanged(m_startFrame, m_endFrame);
    }
}

void PetuniaDopesheetCanvas::setEndFrame(int end) {
    int e = qBound(1, end, m_totalFrames);
    if (m_endFrame != e) {
        m_endFrame = e;
        if (m_startFrame > m_endFrame) {
            m_startFrame = m_endFrame;
        }
        update();
        emit playbackRangeChanged(m_startFrame, m_endFrame);
    }
}

void PetuniaDopesheetCanvas::setPlaybackRange(int start, int end) {
    int s = qBound(1, start, m_totalFrames);
    int e = qBound(s, end, m_totalFrames);
    if (m_startFrame != s || m_endFrame != e) {
        m_startFrame = s;
        m_endFrame = e;
        update();
        emit playbackRangeChanged(m_startFrame, m_endFrame);
    }
}

void PetuniaDopesheetCanvas::setTotalFrames(int total) {
    if (m_totalFrames != total) {
        m_totalFrames = qMax(1, total);
        m_startFrame = qBound(1, m_startFrame, m_totalFrames);
        m_endFrame = qBound(m_startFrame, m_endFrame, m_totalFrames);
        update();
    }
}

void PetuniaDopesheetCanvas::setActiveAsset(int asset) {
    m_activeAsset = asset;
    update();
}

int PetuniaDopesheetCanvas::frameAtX(int x) const {
    int availWidth = width() - m_labelWidth;
    if (availWidth <= 0) return 1;
    float rel = static_cast<float>(x - m_labelWidth) / availWidth;
    int f = 1 + qRound(rel * (m_totalFrames - 1));
    return qBound(1, f, m_totalFrames);
}

int PetuniaDopesheetCanvas::xAtFrame(int frame) const {
    int availWidth = width() - m_labelWidth;
    if (m_totalFrames <= 1) return m_labelWidth;
    float rel = static_cast<float>(frame - 1) / (m_totalFrames - 1);
    return m_labelWidth + qRound(rel * availWidth);
}

void PetuniaDopesheetCanvas::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    const int rulerHeight = 24;
    const int trackHeight = 20;
    const QStringList tracks = {
        QStringLiteral("◆ Resumo"),
        QStringLiteral("  ◆ Posição"),
        QStringLiteral("  ◆ Rotação"),
        QStringLiteral("  ◆ Escala")
    };

    // 1. Fundo do Canvas
    painter.fillRect(rect(), QColor(22, 22, 22));

    // 2. Régua de Tempo Superior
    QRect rulerRect(m_labelWidth, 0, width() - m_labelWidth, rulerHeight);
    painter.fillRect(rulerRect, QColor(30, 30, 30));

    // Cabeçalho da coluna de faixas
    QRect headerRect(0, 0, m_labelWidth, rulerHeight);
    painter.fillRect(headerRect, QColor(36, 36, 36));
    painter.setPen(QColor(160, 160, 160));
    painter.drawText(headerRect.adjusted(8, 0, 0, 0), Qt::AlignVCenter | Qt::AlignLeft, QStringLiteral("Canais"));

    // Ticks e Números de Frames na Régua
    painter.setPen(QColor(80, 80, 80));
    int step = 10;
    if (m_totalFrames > 500) step = 50;
    else if (m_totalFrames > 200) step = 20;

    for (int f = 1; f <= m_totalFrames; ++f) {
        int x = xAtFrame(f);
        if (f % step == 0 || f == 1 || f == m_totalFrames) {
            painter.setPen(QColor(140, 140, 140));
            painter.drawLine(x, rulerHeight - 8, x, rulerHeight);
            painter.drawText(QRect(x - 16, 0, 32, rulerHeight - 8), Qt::AlignCenter, QString::number(f));
        } else if (f % (step / 2) == 0) {
            painter.setPen(QColor(70, 70, 70));
            painter.drawLine(x, rulerHeight - 4, x, rulerHeight);
        }
    }

    // 3. Desenho das Faixas (Tracks)
    for (int t = 0; t < tracks.size(); ++t) {
        int y = rulerHeight + t * trackHeight;
        QRect rowRect(0, y, width(), trackHeight);
        QColor rowBg = (t % 2 == 0) ? QColor(25, 25, 25) : QColor(20, 20, 20);
        painter.fillRect(rowRect, rowBg);

        // Rótulo da faixa
        QRect labelRect(0, y, m_labelWidth, trackHeight);
        painter.setPen(QColor(180, 180, 180));
        QFont f = painter.font();
        f.setPointSize(8);
        painter.setFont(f);
        painter.drawText(labelRect.adjusted(8, 0, 0, 0), Qt::AlignVCenter | Qt::AlignLeft, tracks[t]);

        // Linha divisória horizontal
        painter.setPen(QColor(40, 40, 40));
        painter.drawLine(0, y + trackHeight, width(), y + trackHeight);

        // Grade vertical sutil
        painter.setPen(QColor(32, 32, 32));
        for (int frame = step; frame <= m_totalFrames; frame += step) {
            int gx = xAtFrame(frame);
            painter.drawLine(gx, y, gx, y + trackHeight);
        }

        // Desenhar Marcadores de Keyframes (◆)
        for (int frame = 1; frame <= m_totalFrames; ++frame) {
            if (petunia_qt_app_has_keyframe(m_activeAsset, frame)) {
                int kx = xAtFrame(frame);
                int ky = y + trackHeight / 2;

                // Desenho do losango do keyframe
                QPolygonF diamond;
                float dSize = (t == 0) ? 5.0f : 4.0f;
                diamond << QPointF(kx, ky - dSize)
                        << QPointF(kx + dSize, ky)
                        << QPointF(kx, ky + dSize)
                        << QPointF(kx - dSize, ky);

                if (frame == m_currentFrame) {
                    painter.setBrush(QColor(255, 179, 0)); // Selecionado / ativo
                    painter.setPen(QPen(QColor(255, 255, 255), 1.0));
                } else {
                    painter.setBrush(QColor(255, 213, 79)); // Amarelo Dopesheet
                    painter.setPen(QPen(QColor(30, 30, 30), 1.0));
                }
                painter.drawPolygon(diamond);
            }
        }
    }

    // Divisória vertical entre coluna de nomes e área de tempo
    painter.setPen(QColor(60, 60, 60));
    painter.drawLine(m_labelWidth, 0, m_labelWidth, height());

    // 3.5 Sombreamento de áreas fora do range ativo (In / Out) e marcadores de régua
    int startX = xAtFrame(m_startFrame);
    int endX = xAtFrame(m_endFrame);
    if (m_startFrame > 1 && startX > m_labelWidth) {
        painter.fillRect(QRect(m_labelWidth, 0, startX - m_labelWidth, height()), QColor(0, 0, 0, 95));
    }
    if (m_endFrame < m_totalFrames && endX < width()) {
        painter.fillRect(QRect(endX, 0, width() - endX, height()), QColor(0, 0, 0, 95));
    }

    // Marcadores visuais de In '[' e Out ']' na régua de tempo
    painter.setPen(QPen(QColor(255, 175, 41), 2.0));
    painter.drawLine(startX, 0, startX, rulerHeight);
    painter.drawLine(startX, 0, startX + 4, 0);
    painter.drawLine(startX, rulerHeight - 1, startX + 4, rulerHeight - 1);

    painter.drawLine(endX, 0, endX, rulerHeight);
    painter.drawLine(endX - 4, 0, endX, 0);
    painter.drawLine(endX - 4, rulerHeight - 1, endX, rulerHeight - 1);
    painter.drawLine(startX, rulerHeight - 1, endX, rulerHeight - 1);

    // 4. Linha de Reprodução / Scrubber (Vermelha / Laranja)
    int scrubX = xAtFrame(m_currentFrame);
    painter.setPen(QPen(QColor(233, 106, 0), 2.0));
    painter.drawLine(scrubX, 0, scrubX, height());

    // Marcador superior do scrubber
    QPolygonF scrubHead;
    scrubHead << QPointF(scrubX - 5, 0)
              << QPointF(scrubX + 5, 0)
              << QPointF(scrubX + 5, rulerHeight - 6)
              << QPointF(scrubX, rulerHeight)
              << QPointF(scrubX - 5, rulerHeight - 6);
    painter.setBrush(QColor(233, 106, 0));
    painter.setPen(Qt::NoPen);
    painter.drawPolygon(scrubHead);
}

void PetuniaDopesheetCanvas::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        m_isScrubbing = true;
        int frame = frameAtX(event->pos().x());
        setCurrentFrame(frame);
        emit frameChanged(frame);
    }
}

void PetuniaDopesheetCanvas::mouseMoveEvent(QMouseEvent *event) {
    if (m_isScrubbing) {
        int frame = frameAtX(event->pos().x());
        setCurrentFrame(frame);
        emit frameChanged(frame);
    }
}

void PetuniaDopesheetCanvas::mouseDoubleClickEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && event->pos().y() > 24) {
        int frame = frameAtX(event->pos().x());
        emit keyframeToggled(frame);
    }
}

// =========================================================================
// PetuniaTimelineWidget
// =========================================================================

PetuniaTimelineWidget::PetuniaTimelineWidget(QWidget *parent)
    : QWidget(parent)
    , m_playbackTimer(new QTimer(this))
{
    setupUi();

    connect(m_playbackTimer, &QTimer::timeout, this, &PetuniaTimelineWidget::onTimerTick);
    m_playbackTimer->setInterval(1000 / m_fps);

    syncFromApp();
}

PetuniaTimelineWidget::~PetuniaTimelineWidget() {
    if (m_playbackTimer) {
        m_playbackTimer->stop();
    }
}

void PetuniaTimelineWidget::setupUi() {
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(6, 4, 6, 4);
    mainLayout->setSpacing(4);

    setStyleSheet(QStringLiteral(
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 3px 8px; font-weight: bold; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
        "QPushButton:checked { background-color: #e53935; color: #ffffff; }"
        "QSpinBox, QComboBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 2px 6px; }"
    ));

    // 1. Barra de Transporte Superior
    auto *transportRow = new QHBoxLayout();
    transportRow->setContentsMargins(0, 0, 0, 0);
    transportRow->setSpacing(4);

    m_firstBtn = new QPushButton(PetuniaIcons::get("jump_start"), QString(), this);
    m_firstBtn->setToolTip(QStringLiteral("Ir para Primeiro Frame (Shift+Esquerda)"));
    connect(m_firstBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::jumpToFirst);
    transportRow->addWidget(m_firstBtn);

    m_prevKeyBtn = new QPushButton(QStringLiteral("◀◆"), this);
    m_prevKeyBtn->setToolTip(QStringLiteral("Keyframe Anterior"));
    connect(m_prevKeyBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::prevKeyframe);
    transportRow->addWidget(m_prevKeyBtn);

    m_playBtn = new QPushButton(PetuniaIcons::get("play"), QString(), this);
    m_playBtn->setFixedWidth(36);
    m_playBtn->setToolTip(QStringLiteral("Reproduzir / Pausar (Espaço)"));
    connect(m_playBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::togglePlayback);
    transportRow->addWidget(m_playBtn);

    m_nextKeyBtn = new QPushButton(QStringLiteral("◆▶"), this);
    m_nextKeyBtn->setToolTip(QStringLiteral("Próximo Keyframe"));
    connect(m_nextKeyBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::nextKeyframe);
    transportRow->addWidget(m_nextKeyBtn);

    m_endBtn = new QPushButton(PetuniaIcons::get("jump_end"), QString(), this);
    m_endBtn->setToolTip(QStringLiteral("Ir para Último Frame (Shift+Direita)"));
    connect(m_endBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::jumpToEnd);
    transportRow->addWidget(m_endBtn);

    transportRow->addSpacing(8);

    // Auto-Key
    m_autoKeyBtn = new QPushButton(PetuniaIcons::get("keyframe"), QStringLiteral(" Auto-Key"), this);
    m_autoKeyBtn->setCheckable(true);
    m_autoKeyBtn->setToolTip(QStringLiteral("Gravação Automática de Keyframes ao Transformar"));
    transportRow->addWidget(m_autoKeyBtn);

    // Inserir / Deletar Keyframes Manuais
    m_addKeyBtn = new QPushButton(PetuniaIcons::get("keyframe"), QStringLiteral(" +Key"), this);
    m_addKeyBtn->setToolTip(QStringLiteral("Inserir Keyframe no Frame Atual (I)"));
    connect(m_addKeyBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::insertKeyframeAtCurrent);
    transportRow->addWidget(m_addKeyBtn);

    m_delKeyBtn = new QPushButton(PetuniaIcons::get("delete"), QStringLiteral(" -Key"), this);
    m_delKeyBtn->setToolTip(QStringLiteral("Deletar Keyframe no Frame Atual (Alt+I)"));
    connect(m_delKeyBtn, &QPushButton::clicked, this, &PetuniaTimelineWidget::deleteKeyframeAtCurrent);
    transportRow->addWidget(m_delKeyBtn);

    transportRow->addSpacing(8);

    // Indicadores Numéricos de Range e Frame Atual (In, Playhead, Out)
    auto *inLabel = new QLabel(QStringLiteral("In:"), this);
    inLabel->setStyleSheet(QStringLiteral("color: #888888; font-size: 11px;"));
    transportRow->addWidget(inLabel);

    m_startSpin = new QSpinBox(this);
    m_startSpin->setToolTip(QStringLiteral("Frame Inicial da Reprodução (In Point)"));
    m_startSpin->setRange(1, 250);
    m_startSpin->setValue(1);
    m_startSpin->setFixedWidth(56);
    connect(m_startSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int s) {
        if (m_endSpin && s > m_endSpin->value()) {
            m_endSpin->blockSignals(true);
            m_endSpin->setValue(s);
            m_endSpin->blockSignals(false);
        }
        if (m_canvas) m_canvas->setStartFrame(s);
    });
    transportRow->addWidget(m_startSpin);

    auto *curLabel = new QLabel(QStringLiteral("Frame:"), this);
    curLabel->setStyleSheet(QStringLiteral("color: #b58cff; font-weight: bold; font-size: 11px;"));
    transportRow->addWidget(curLabel);

    m_frameSpin = new QSpinBox(this);
    m_frameSpin->setToolTip(QStringLiteral("Frame Atual (Playhead)"));
    m_frameSpin->setRange(1, 250);
    m_frameSpin->setValue(1);
    m_frameSpin->setFixedWidth(62);
    connect(m_frameSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, &PetuniaTimelineWidget::setFrame);
    transportRow->addWidget(m_frameSpin);

    auto *outLabel = new QLabel(QStringLiteral("Out:"), this);
    outLabel->setStyleSheet(QStringLiteral("color: #888888; font-size: 11px;"));
    transportRow->addWidget(outLabel);

    m_endSpin = new QSpinBox(this);
    m_endSpin->setToolTip(QStringLiteral("Frame Final da Reprodução (Out Point)"));
    m_endSpin->setRange(1, 250);
    m_endSpin->setValue(250);
    m_endSpin->setFixedWidth(56);
    connect(m_endSpin, QOverload<int>::of(&QSpinBox::valueChanged), this, [this](int e) {
        if (m_startSpin && e < m_startSpin->value()) {
            m_startSpin->blockSignals(true);
            m_startSpin->setValue(e);
            m_startSpin->blockSignals(false);
        }
        if (m_canvas) m_canvas->setEndFrame(e);
    });
    transportRow->addWidget(m_endSpin);

    m_totalFramesLabel = new QLabel(QStringLiteral("/ 250"), this);
    m_totalFramesLabel->setStyleSheet(QStringLiteral("color: #888888; font-weight: bold;"));
    transportRow->addWidget(m_totalFramesLabel);

    transportRow->addStretch();

    // Taxa de Quadros (FPS)
    transportRow->addWidget(new QLabel(QStringLiteral("FPS:"), this));
    m_fpsCombo = new QComboBox(this);
    m_fpsCombo->addItem(QStringLiteral("24 fps"), 24);
    m_fpsCombo->addItem(QStringLiteral("30 fps"), 30);
    m_fpsCombo->addItem(QStringLiteral("60 fps"), 60);
    m_fpsCombo->setCurrentIndex(1); // 30 fps
    connect(m_fpsCombo, QOverload<int>::of(&QComboBox::currentIndexChanged), this, [this](int idx) {
        m_fps = m_fpsCombo->itemData(idx).toInt();
        if (m_playbackTimer) m_playbackTimer->setInterval(1000 / m_fps);
    });
    transportRow->addWidget(m_fpsCombo);

    mainLayout->addLayout(transportRow);

    // 2. Canvas Visual do Dopesheet
    m_canvas = new PetuniaDopesheetCanvas(this);
    connect(m_canvas, &PetuniaDopesheetCanvas::frameChanged, this, [this](int f) {
        m_frameSpin->blockSignals(true);
        m_frameSpin->setValue(f);
        m_frameSpin->blockSignals(false);
        petunia_qt_app_set_timeline_frame(f);
        emit frameChanged(f);
    });
    connect(m_canvas, &PetuniaDopesheetCanvas::playbackRangeChanged, this, [this](int s, int e) {
        if (m_startSpin && m_startSpin->value() != s) {
            m_startSpin->blockSignals(true);
            m_startSpin->setValue(s);
            m_startSpin->blockSignals(false);
        }
        if (m_endSpin && m_endSpin->value() != e) {
            m_endSpin->blockSignals(true);
            m_endSpin->setValue(e);
            m_endSpin->blockSignals(false);
        }
        emit playbackRangeChanged(s, e);
    });
    connect(m_canvas, &PetuniaDopesheetCanvas::keyframeToggled, this, [this](int f) {
        int asset = petunia_qt_app_get_active_asset();
        if (petunia_qt_app_has_keyframe(asset, f)) {
            petunia_qt_app_delete_keyframe(asset, f);
            emit keyframeDeleted(asset, f);
        } else {
            petunia_qt_app_insert_keyframe(asset, f);
            emit keyframeInserted(asset, f);
        }
        m_canvas->update();
    });
    mainLayout->addWidget(m_canvas, 1);
}

int PetuniaTimelineWidget::currentFrame() const {
    return m_frameSpin ? m_frameSpin->value() : 1;
}

int PetuniaTimelineWidget::startFrame() const {
    return m_canvas ? m_canvas->startFrame() : 1;
}

int PetuniaTimelineWidget::endFrame() const {
    return m_canvas ? m_canvas->endFrame() : 250;
}

bool PetuniaTimelineWidget::isAutoKeyEnabled() const {
    return m_autoKeyBtn && m_autoKeyBtn->isChecked();
}

void PetuniaTimelineWidget::setFrame(int frame) {
    if (m_canvas) m_canvas->setCurrentFrame(frame);
    if (m_frameSpin && m_frameSpin->value() != frame) {
        m_frameSpin->blockSignals(true);
        m_frameSpin->setValue(frame);
        m_frameSpin->blockSignals(false);
    }
    petunia_qt_app_set_timeline_frame(frame);
    emit frameChanged(frame);
}

void PetuniaTimelineWidget::setStartFrame(int start) {
    if (m_canvas) m_canvas->setStartFrame(start);
    if (m_startSpin && m_startSpin->value() != start) {
        m_startSpin->blockSignals(true);
        m_startSpin->setValue(start);
        m_startSpin->blockSignals(false);
    }
}

void PetuniaTimelineWidget::setEndFrame(int end) {
    if (m_canvas) m_canvas->setEndFrame(end);
    if (m_endSpin && m_endSpin->value() != end) {
        m_endSpin->blockSignals(true);
        m_endSpin->setValue(end);
        m_endSpin->blockSignals(false);
    }
}

void PetuniaTimelineWidget::setPlaybackRange(int start, int end) {
    if (m_canvas) m_canvas->setPlaybackRange(start, end);
    if (m_startSpin) {
        m_startSpin->blockSignals(true);
        m_startSpin->setValue(start);
        m_startSpin->blockSignals(false);
    }
    if (m_endSpin) {
        m_endSpin->blockSignals(true);
        m_endSpin->setValue(end);
        m_endSpin->blockSignals(false);
    }
    emit playbackRangeChanged(start, end);
}

void PetuniaTimelineWidget::togglePlayback() {
    if (m_isPlaying) {
        pause();
    } else {
        play();
    }
}

void PetuniaTimelineWidget::play() {
    m_isPlaying = true;
    if (m_playBtn) m_playBtn->setText(QStringLiteral("⏸"));
    if (m_playbackTimer) m_playbackTimer->start();
    emit playbackToggled(true);
}

void PetuniaTimelineWidget::pause() {
    m_isPlaying = false;
    if (m_playBtn) m_playBtn->setText(QStringLiteral("▶"));
    if (m_playbackTimer) m_playbackTimer->stop();
    emit playbackToggled(false);
}

void PetuniaTimelineWidget::jumpToFirst() {
    setFrame(startFrame());
}

void PetuniaTimelineWidget::jumpToEnd() {
    setFrame(endFrame());
}

void PetuniaTimelineWidget::prevKeyframe() {
    int cur = currentFrame();
    int asset = petunia_qt_app_get_active_asset();
    int start = startFrame();
    for (int f = cur - 1; f >= start; --f) {
        if (petunia_qt_app_has_keyframe(asset, f)) {
            setFrame(f);
            return;
        }
    }
    setFrame(start);
}

void PetuniaTimelineWidget::nextKeyframe() {
    int cur = currentFrame();
    int asset = petunia_qt_app_get_active_asset();
    int end = endFrame();
    for (int f = cur + 1; f <= end; ++f) {
        if (petunia_qt_app_has_keyframe(asset, f)) {
            setFrame(f);
            return;
        }
    }
    setFrame(end);
}

void PetuniaTimelineWidget::insertKeyframeAtCurrent() {
    int cur = currentFrame();
    int asset = petunia_qt_app_get_active_asset();
    if (petunia_qt_app_insert_keyframe(asset, cur)) {
        if (m_canvas) m_canvas->update();
        emit keyframeInserted(asset, cur);
    }
}

void PetuniaTimelineWidget::deleteKeyframeAtCurrent() {
    int cur = currentFrame();
    int asset = petunia_qt_app_get_active_asset();
    if (petunia_qt_app_delete_keyframe(asset, cur)) {
        if (m_canvas) m_canvas->update();
        emit keyframeDeleted(asset, cur);
    }
}

void PetuniaTimelineWidget::onTimerTick() {
    int start = startFrame();
    int end = endFrame();
    int next = currentFrame() + 1;
    if (next > end) {
        next = start; // Loop contínuo dentro da janela ativa
    }
    setFrame(next);
}

void PetuniaTimelineWidget::syncFromApp() {
    int cur = 1, total = 250;
    bool playing = false;
    if (petunia_qt_app_get_timeline_info(&cur, &total, &playing)) {
        if (m_canvas) {
            m_canvas->setTotalFrames(total);
            m_canvas->setActiveAsset(petunia_qt_app_get_active_asset());
            m_canvas->setCurrentFrame(cur);
        }
        if (m_frameSpin) {
            m_frameSpin->blockSignals(true);
            m_frameSpin->setRange(1, total);
            m_frameSpin->setValue(cur);
            m_frameSpin->blockSignals(false);
        }
        if (m_startSpin) {
            m_startSpin->blockSignals(true);
            m_startSpin->setRange(1, total);
            m_startSpin->blockSignals(false);
        }
        if (m_endSpin) {
            m_endSpin->blockSignals(true);
            m_endSpin->setRange(1, total);
            m_endSpin->blockSignals(false);
        }
        if (m_totalFramesLabel) {
            m_totalFramesLabel->setText(QStringLiteral("/ %1").arg(total));
        }
    }
}
