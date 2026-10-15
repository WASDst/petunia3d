#pragma once

#include <QWidget>
#include <QPushButton>
#include <QSpinBox>
#include <QComboBox>
#include <QTimer>
#include <QLabel>
#include <QPaintEvent>
#include <QMouseEvent>

class PetuniaDopesheetCanvas : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaDopesheetCanvas(QWidget *parent = nullptr);
    ~PetuniaDopesheetCanvas() override = default;

    int currentFrame() const { return m_currentFrame; }
    int startFrame() const { return m_startFrame; }
    int endFrame() const { return m_endFrame; }
    int totalFrames() const { return m_totalFrames; }
    void setCurrentFrame(int frame);
    void setStartFrame(int start);
    void setEndFrame(int end);
    void setPlaybackRange(int start, int end);
    void setTotalFrames(int total);
    void setActiveAsset(int asset);

signals:
    void frameChanged(int frame);
    void keyframeToggled(int frame);
    void playbackRangeChanged(int start, int end);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;

private:
    int frameAtX(int x) const;
    int xAtFrame(int frame) const;

    int m_currentFrame{1};
    int m_startFrame{1};
    int m_endFrame{250};
    int m_totalFrames{250};
    int m_activeAsset{0};
    int m_labelWidth{130};
    bool m_isScrubbing{false};
};

class PetuniaTimelineWidget : public QWidget {
    Q_OBJECT

public:
    explicit PetuniaTimelineWidget(QWidget *parent = nullptr);
    ~PetuniaTimelineWidget() override;

    int currentFrame() const;
    int startFrame() const;
    int endFrame() const;
    bool isPlaying() const { return m_isPlaying; }
    bool isAutoKeyEnabled() const;

public slots:
    void setFrame(int frame);
    void setStartFrame(int start);
    void setEndFrame(int end);
    void setPlaybackRange(int start, int end);
    void togglePlayback();
    void play();
    void pause();
    void jumpToFirst();
    void jumpToEnd();
    void prevKeyframe();
    void nextKeyframe();
    void insertKeyframeAtCurrent();
    void deleteKeyframeAtCurrent();
    void syncFromApp();

signals:
    void frameChanged(int frame);
    void playbackRangeChanged(int start, int end);
    void playbackToggled(bool isPlaying);
    void keyframeInserted(int asset, int frame);
    void keyframeDeleted(int asset, int frame);

private slots:
    void onTimerTick();

private:
    void setupUi();

    PetuniaDopesheetCanvas *m_canvas{nullptr};

    // Botões de Transporte
    QPushButton *m_firstBtn{nullptr};
    QPushButton *m_prevKeyBtn{nullptr};
    QPushButton *m_playBtn{nullptr};
    QPushButton *m_nextKeyBtn{nullptr};
    QPushButton *m_endBtn{nullptr};
    QPushButton *m_autoKeyBtn{nullptr};
    QPushButton *m_addKeyBtn{nullptr};
    QPushButton *m_delKeyBtn{nullptr};

    QSpinBox *m_startSpin{nullptr};
    QSpinBox *m_frameSpin{nullptr};
    QSpinBox *m_endSpin{nullptr};
    QLabel *m_totalFramesLabel{nullptr};
    QComboBox *m_fpsCombo{nullptr};

    QTimer *m_playbackTimer{nullptr};
    bool m_isPlaying{false};
    int m_fps{30};
};
