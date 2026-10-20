# 07 — Acessibilidade, cognição espacial e interação profissional

**Status:** princípios aprovados + contratos sugeridos para implementação. Público: iniciantes e especialistas; Windows/Linux; pessoas com diferentes capacidades de percepção espacial, visão, mobilidade e atenção.

## 7.1 Filosofia

**Não reduzir capacidades; reduzir a exposição simultânea de complexidade.** Profissionais permanecem rápidos via comandos/atalhos; iniciantes recebem feedback contextual, informações descritas em linguagem natural, previews não destrutivas e pistas espaciais. O sistema nunca assume que corpo humanoide/animal deve seguir proporção “correta”.

## 7.2 Visão e cor

- Sinais essenciais redundantes: cor + espessura de linha, forma, textura/hachura ou texto.
- X/Y/Z com convenção RGB como default, mas labels visíveis sob demanda e paletas alternativas.
- Handles/límites selecionados devem ter contraste adequado sobre fundos claros/escuros/Clay/MatCap/textura; metas baseadas em WCAG 2.2 (não afirmar conformidade web de app Qt sem auditoria específica).
- Slider para espessura de contorno, tamanho de alça, escala de UI e texto, cavity, AO, grid, motion; configurações específicas para reduções de estímulo visual.
- Mapas de espessura/depth/weights sempre com legenda numérica e gradientes perceptualmente ordenados ou faixas identificadas; não depender de red-green único.
- Foco visual evidente para teclado, independentemente da seleção geométrica.

## 7.3 Movimento, teclado, caneta e entrada numérica

- Nenhuma operação essencial só por drag: clique em handle + valor + Enter, fields tabuláveis, comandos via palette, ajustes incrementais.
- Drag protegido SEL-004; input device normalized no editor, respeitando DPIs, stylus pressure/tilt quando relevante sem exigir caneta.
- Área real de hit test suficientemente grande sem colisão com vizinhas; alternativa de navegar por alças ativas por teclado.
- Cancela (`Esc`) e Undo em qualquer preview, com mensagem que indique que documento não mudou.
- Gestos multietapas sem timeouts surpreendentes; keymap configurável e busca de ferramentas por nome/objetivo.
- Assistência espacial e popup companion não devem roubar foco de campos numéricos.

## 7.4 Modelo de informação para leitores de tela

Viewport GL/QPainter é principalmente visual. Criar modelo semântico paralelo exposto ao Qt Accessibility (`QAccessible` e propriedade de widgets apropriada), usando `AccessibleSelectionModel`, `AccessibleMeasurementModel`, `AccessibleInspectionModel`, `AccessibleRigModel`.

O modelo deve incluir: entidade, tipo, IDs/nomes, eixo/frame, medida/unidade/qualidade, status da operação, erro, descrição do botão e disponibilidade. Anúncios durante drag devem ser **debitados/limitados**; não falar a cada frame. Valores dinâmicos com consulta on-demand e, quando adequado, live region discreta.

Exemplos:

- “Braço esquerdo selecionado; objeto; Move pelo eixo local X; alteração mais 3 centímetros; prévia, Enter para aplicar, Esc para cancelar”.
- “Espessura aproximada 4,2 milímetros; raycast direcional; não implica espessura mínima”.
- “Companion View lateral sugerida; pressione atalho configurado para abrir”.

## 7.5 Perfis e persistência

Presets `Essential`, `Assisted`, `Analytical`, `Organic` (pode ser preset específico, não novo sistema de modo):

| Perfil | Comportamento |
|---|---|
| Essential | Clay/iluminação legível + gizmo, sem cotas fixas automáticas |
| Assisted | Volume Frame discreto, feedback de operação e sugestões não intrusivas |
| Analytical | Medidas e seções ampliadas sob controle do usuário |
| Organic | Clay Form opcional, simetria visual, shape profile, silhueta e companions |

Sempre permitir alterações individuais e persistência de preferências manuais. Mudar uma ferramenta não redefine shading escolhido. Auto mode explicitamente ligado é exceção deliberada, não default.

## 7.6 Overlay Layout Manager

Ordena prioridades sem cobrir a região manipulada: gizmo/handle ativo > seleção > rótulo numérico principal > indicação de snap relevante > avisos > cotas secundárias > elementos técnicos decorativos. Anchors 3D projetados no viewport e offsets em screen space; rótulos com histerese no posicionamento para não “tremer”. Cotas interativas possuem hit ID separado; puramente informativas são click-through.

Ao caber pouco conteúdo (mobile/small window/high zoom), compactar em status bar/painel semântico. Nunca cortar valor numericamente essencial apenas para preservar aparência; oferecer expand.

## 7.7 Testes com participantes

Não alegar que recurso “ajuda iniciantes” até testar. Comparar viewport sem assistência / com Volume+Depth / com Companion+Section. Tarefas representativas: alinhar braço em profundidade, escalar torso à medida certa, formar quatro patas, cortar seção, corrigir weights e pose. Medir conclusão, erro dimensional, tempo, órbitas de câmera, retries/Undo, conforto e carga subjetiva. Segmentar por experiência técnica e preferências de feedback quando possível; evitar concluir “melhor para todos” com amostra pequena.

## 7.8 Heurísticas de não intrusão

- Companion View **sugerida**, nunca autoabertura sem opt-in (VSP-004).
- Repetir sugestões apenas após mudança real no contexto e cooldown funcional, sem sugerir na mesma operação continuamente.
- Não orbitar ou dar zoom automático no meio do drag.
- Modo de análise explícito pode exibir mais overlays, mas ao sair restaura escolha de aparência anterior.
- Geometry Truth e Inspect Section não desativam material/rig de forma persistente.
- Informação numérica oculta não é excluída: sempre acessível no inspector, por comando ou por pin.
