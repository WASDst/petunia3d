# Icon Packs — Petunia3D (`assets/icons`)

Este diretório contém os pacotes de ícones disponíveis para a interface gráfica do Petunia3D.

## Estrutura de um Pacote de Ícones

Cada pacote de ícones reside em seu próprio subdiretório e contém:
- `manifest.toml`: Metadados do pacote (identificador, nome amigável, autor, versão, licença).
- `icons.toml`: Mapeamento de identificadores de ícones (`IconId`) para fontes vetoriais/raster ou glifos canônicos.

### Pacotes Nativos Disponíveis

Apenas diretórios com `manifest.toml` são registrados (P3D-087/088).
Arte de ícones não registrada fica em `assets/legacy-icon-art/`.

1. **`petunia/`**: Pacote canônico oficial (Petunia Custom Icons), primeira arte do produto para conceitos 3D sem equivalente genérico (Extrude, Inset, Round Edge, Loop Cut, shading, pivot, entre outros).
2. **`phosphor/`**: Pacote vetorial baseado nos ícones Phosphor.
3. **`tabler/`**: Pacote vetorial baseado na suíte Tabler Icons.
4. **`iconoir/`**: Pacote minimalista baseado na biblioteca Iconoir.
5. **`lucide/`**: Pacote limpo e moderno baseado em Lucide Icons.
6. **`petunia-outline/` e `petunia-filled/`**: Família original em SVG com IDs semânticos idênticos nas duas variantes. Consulte a [galeria, catálogo, gerador e matriz de cobertura](petunia-dual/README.md). Os assets estão prontos para um resolver de `IconId`; o shell Slint ainda referencia SVGs estáticos.

## Criação de Pacotes Customizados

Para adicionar um novo pacote de ícones, crie uma pasta em `assets/icons/<nome-do-pacote>` contendo:

```toml
# manifest.toml
id = "meu-pacote"
name = "Meu Pacote Customizado"
version = "1.0.0"
author = "Comunidade"
license = "MIT"
```

O Petunia3D detecta dinamicamente pacotes presentes nesta pasta e os disponibiliza no modal de Configurações (`⚙ Config -> Ícones`).
