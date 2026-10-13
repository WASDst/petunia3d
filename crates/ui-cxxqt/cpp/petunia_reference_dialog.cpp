#include "petunia_reference_dialog.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>
#include <QFileDialog>
#include <QDialogButtonBox>
#include <QFileInfo>

PetuniaReferenceDialog::PetuniaReferenceDialog(QWidget *parent)
    : QDialog(parent)
{
    setupUi();
}

void PetuniaReferenceDialog::setupUi() {
    setWindowTitle(QStringLiteral("Gerenciador de Imagens de Referência"));
    resize(580, 420);
    setStyleSheet(QStringLiteral(
        "QDialog { background-color: #1e1e1e; color: #eeeeee; }"
        "QGroupBox { color: #b5b5b5; font-weight: bold; border: 1px solid #404040; border-radius: 4px; margin-top: 10px; padding-top: 10px; }"
        "QListWidget { background-color: #202020; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; selection-background-color: #b58cff; selection-color: #121212; }"
        "QComboBox, QDoubleSpinBox { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 4px; }"
        "QPushButton { background-color: #292929; color: #eeeeee; border: 1px solid #404040; border-radius: 4px; padding: 6px 14px; font-weight: bold; }"
        "QPushButton:hover { background-color: #383838; border-color: #b58cff; }"
        "QPushButton:default { background-color: #b58cff; color: #121212; }"
    ));

    auto *mainLayout = new QHBoxLayout(this);
    mainLayout->setContentsMargins(12, 12, 12, 12);

    // Coluna esquerda: Lista de referências
    auto *leftCol = new QVBoxLayout();
    m_imageList = new QListWidget(this);
    leftCol->addWidget(m_imageList);

    auto *btnRow = new QHBoxLayout();
    m_addBtn = new QPushButton(QStringLiteral("Adicionar..."), this);
    connect(m_addBtn, &QPushButton::clicked, this, &PetuniaReferenceDialog::addReferenceImage);
    btnRow->addWidget(m_addBtn);

    m_removeBtn = new QPushButton(QStringLiteral("Remover"), this);
    connect(m_removeBtn, &QPushButton::clicked, this, &PetuniaReferenceDialog::removeReferenceImage);
    btnRow->addWidget(m_removeBtn);
    leftCol->addLayout(btnRow);

    mainLayout->addLayout(leftCol, 1);

    // Coluna direita: Propriedades da referência selecionada
    auto *rightCol = new QVBoxLayout();
    auto *propsGroup = new QGroupBox(QStringLiteral("Propriedades do Plano"), this);
    auto *form = new QFormLayout(propsGroup);
    form->setContentsMargins(12, 12, 12, 12);
    form->setSpacing(10);

    m_planeCombo = new QComboBox(propsGroup);
    m_planeCombo->addItem(QStringLiteral("Frente (Plano XZ)"));
    m_planeCombo->addItem(QStringLiteral("Superior (Plano XY)"));
    m_planeCombo->addItem(QStringLiteral("Direita (Plano YZ)"));
    form->addRow(QStringLiteral("Alinhamento:"), m_planeCombo);

    m_opacitySlider = new QSlider(Qt::Horizontal, propsGroup);
    m_opacitySlider->setRange(10, 100);
    m_opacitySlider->setValue(50);
    m_opacityLabel = new QLabel(QStringLiteral("50%"), propsGroup);
    auto *opacityRow = new QHBoxLayout();
    opacityRow->addWidget(m_opacitySlider);
    opacityRow->addWidget(m_opacityLabel);
    connect(m_opacitySlider, &QSlider::valueChanged, this, &PetuniaReferenceDialog::updateImageOpacity);
    form->addRow(QStringLiteral("Opacidade:"), opacityRow);

    m_scaleSpin = new QDoubleSpinBox(propsGroup);
    m_scaleSpin->setRange(0.01, 100.0);
    m_scaleSpin->setValue(1.0);
    m_scaleSpin->setSingleStep(0.1);
    m_scaleSpin->setSuffix(QStringLiteral("x"));
    form->addRow(QStringLiteral("Escala:"), m_scaleSpin);

    m_offsetX = new QDoubleSpinBox(propsGroup);
    m_offsetX->setRange(-1000.0, 1000.0);
    m_offsetX->setSingleStep(0.1);
    m_offsetX->setSuffix(QStringLiteral(" m"));
    form->addRow(QStringLiteral("Deslocamento X:"), m_offsetX);

    m_offsetY = new QDoubleSpinBox(propsGroup);
    m_offsetY->setRange(-1000.0, 1000.0);
    m_offsetY->setSingleStep(0.1);
    m_offsetY->setSuffix(QStringLiteral(" m"));
    form->addRow(QStringLiteral("Deslocamento Y:"), m_offsetY);

    rightCol->addWidget(propsGroup);
    rightCol->addStretch();

    auto *buttonBox = new QDialogButtonBox(QDialogButtonBox::Close, this);
    connect(buttonBox, &QDialogButtonBox::rejected, this, &QDialog::accept);
    rightCol->addWidget(buttonBox);

    mainLayout->addLayout(rightCol, 1);
}

void PetuniaReferenceDialog::addReferenceImage() {
    QString file = QFileDialog::getOpenFileName(
        this,
        QStringLiteral("Selecionar Imagem de Referência"),
        QString(),
        QStringLiteral("Imagens (*.png *.jpg *.jpeg *.webp *.bmp);;Todos os Arquivos (*)")
    );
    if (!file.isEmpty()) {
        QFileInfo fi(file);
        m_imageList->addItem(fi.fileName());
        m_imageList->setCurrentRow(m_imageList->count() - 1);
    }
}

void PetuniaReferenceDialog::removeReferenceImage() {
    int row = m_imageList->currentRow();
    if (row >= 0) {
        delete m_imageList->takeItem(row);
    }
}

void PetuniaReferenceDialog::updateImageOpacity(int val) {
    m_opacityLabel->setText(QStringLiteral("%1%").arg(val));
}
