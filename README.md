# Memory Pier

Continue um trabalho de programação em outro agente ou entregue o contexto a outra pessoa, sem depender de uma nova resposta da IA de origem.

## Proposta

Memory Pier será uma ferramenta de terminal independente de IDE. Seu núcleo lê registros disponíveis de sessões, organiza um pacote local e prepara a retomada. Skills e plugins poderão facilitar seu uso, mas não serão necessários para exportar.

A visão inclui uma dashboard em terminal para sessões, uso disponível e perfis de contas, conforme suporte de cada agente. O núcleo de exportação vem primeiro; cotas e autenticação dependem de pesquisa técnica.

O destinatário deve conseguir usar o pacote sem instalar Memory Pier: um Markdown legível será a porta de entrada, acompanhado do histórico selecionado e das referências necessárias.

## Primeiro marco

Exportar uma sessão de um agente para um pacote que outro desenvolvedor consiga usar no mesmo projeto. O núcleo será em Rust, com primeiro leitor para Claude Code e validação inicial em macOS arm64. Veja a [decisão técnica](docs/decisions/0004-rust-stack.md).

O pacote deve identificar o pedido original, preservar a proveniência das mensagens, referenciar repositório/branch/commit e informar omissões. Alterações locais selecionadas poderão acompanhar o contexto; a exportação não deve publicar código automaticamente.

A exportação básica funciona sem chamadas a modelos. Sínteses semânticas dependem de checkpoints existentes ou de uma etapa opcional com IA. Abrir uma sessão nova não renova cotas de uso do provedor.

## Estado

Etapa 01 concluída. Etapa 02 em andamento: projeto Cargo e primeiro leitor Claude
Code com descoberta por projeto e seleção explícita de ramo, testados com fixtures
sintéticas. Etapa 03 implementada para exportação somente de contexto, com prévia,
exclusões e manifesto v1. Validação manual e compatibilidade real pendentes; TUI
continua no roadmap. Etapa 04 iniciada: `export --project <pasta>` registra referência Git local; `--include-path <arquivo>` inclui alterações textuais selecionadas em pacote v2. `verify` confere pacotes e `apply --check` / `--write` verifica ou aplica explicitamente em checkout limpo com base exata.

```sh
cargo run --locked -- inspect testdata/claude/basic.jsonl
cargo run --locked -- sessions --root testdata/claude-projects --project /synthetic/project
cargo run --locked -- inspect testdata/claude-projects/arbitrary/session.jsonl --leaf a1
cargo run --locked -- export testdata/claude/basic.jsonl --preview
```

A saída é um relatório JSON local com eventos, proveniência e diagnósticos.
Veja [inspeção e descoberta](docs/specs/claude-reader.md),
[exportação e limites](docs/specs/context-export.md), [recebimento e aplicação](docs/specs/receive-apply.md) e o
[roteiro para testar um por um](docs/MANUAL_TESTS.md). Para gravar um pacote,
use `export <arquivo> --output <pasta-nova>`; a pasta pai deve existir.

## Desenvolvimento

A toolchain Rust 1.98.1 está fixada em `rust-toolchain.toml`; instale Rust via rustup.

```sh
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

A validação adicional de pacotes usa Python apenas no desenvolvimento/CI:

```sh
python3 -m venv /tmp/memory-pier-validation
/tmp/memory-pier-validation/bin/pip install -r scripts/requirements-validation.txt
/tmp/memory-pier-validation/bin/python scripts/check_bundle.py
```

- Comece por [AGENTS.md](AGENTS.md), mesmo usando uma ferramenta que não carregue esse arquivo automaticamente.
- [Roadmap canônico em Markdown](ROADMAP.md)
- [Estado de execução e próxima tarefa](docs/EXECUTION.md)
- [Plano visual: prioridades e ordem de implementação](docs/roadmap/index.html)
- [Fluxo de contribuição](CONTRIBUTING.md)
- [Instruções para agentes](AGENTS.md)
- [Decisões de arquitetura e processo](docs/decisions/0001-development-workflow.md)

O Orca é uma referência de pesquisa para captura de contexto e leitores de sessões: <https://github.com/stablyai/orca>. Nenhum código dele foi incorporado nesta etapa.
