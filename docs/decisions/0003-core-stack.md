# 0003 — Núcleo em Go e primeiro adaptador Claude Code

Status: substituído pelo [ADR 0004 — Rust](0004-rust-stack.md), por escolha explícita do mantenedor em 2026-09-24. O conteúdo abaixo preserva a decisão histórica; não orienta a implementação atual.

## Comparação e decisão

| Opção | Vantagem neste projeto | Custo neste projeto |
|---|---|---|
| Go | Binário distribuível, biblioteca padrão para JSON/arquivos/processos e ecossistema TUI | Adaptação de parsers existentes em TS/Python e integração de keychain por SO |
| TypeScript / Node | Proximidade do código de referência do Orca; iteração rápida | Distribuição exige runtime ou empacotamento adicional |
| Python | Parsers e experimentação simples | Distribuição e ambientes do usuário exigem gerenciamento adicional |
| Rust | Executável nativo e controle de recursos | Maior complexidade inicial para uma ferramenta centrada em arquivos e interfaces |

Escolher Go para CLI e núcleo. Usar inicialmente a biblioteca padrão; não introduzir banco, serviço ou framework CLI sem necessidade demonstrada. Bubble Tea é a direção para a TUI da etapa 06, não uma dependência instalada agora. A versão de Go será fixada no bootstrap do módulo e CI da etapa 02; Go não estava no PATH desta máquina nesta pesquisa.

Primeiro alvo de execução: macOS arm64 (ambiente disponível para validar). Linux e Windows são destinos futuros, não plataformas testadas. A compilação cruzada não substitui testes de execução. Dependências de keychain e CGO poderão impor limites que precisam ser validados separadamente.

Primeiro leitor: Claude Code JSONL, inicialmente por caminho explícito. A descoberta por projeto vem na mesma etapa 02, após a leitura determinística. A CLI instalada informa `2.1.281`; nenhum chat real foi aberto ou exportado nesta pesquisa. Compatibilidade por versão ainda não está certificada. Codex é o segundo candidato, etapa 05.

## Fronteiras

- Núcleo de leitura/normalização independente da CLI, TUI, credenciais e rede.
- CLI inicial deve inspecionar um arquivo explícito, reportando eventos e perdas; não executar comandos do histórico.
- Conteúdo normalizado e manifesto versionado são o contrato com futuros adaptadores.
- Autenticação, cotas e integrações MCP ficam fora do leitor.
- Python usado para verificar artefatos de pesquisa não será requisito de execução do produto.

## Referências consultadas

- [Go: GOOS/GOARCH e construção de ferramentas](https://go.dev/doc/install/source).
- [Bubble Tea: framework TUI em Go](https://github.com/charmbracelet/bubbletea).
- [Claude Code: armazenamento de sessões](https://code.claude.com/docs/en/sessions).

A escolha decorre dos objetivos de distribuição e terminal, não de um benchmark de desempenho. Reavaliar em ADR se o protótipo revelar limitações materiais.
