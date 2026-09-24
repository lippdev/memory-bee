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
sintéticas. Exportador e TUI pendentes; compatibilidade real não certificada.

```sh
cargo run --locked -- inspect testdata/claude/basic.jsonl
cargo run --locked -- sessions --root testdata/claude-projects --project /synthetic/project
cargo run --locked -- inspect testdata/claude-projects/arbitrary/session.jsonl --leaf a1
```

A saída é um relatório JSON local com eventos, proveniência e diagnósticos.
Veja [uso, limites e códigos de saída](docs/specs/claude-reader.md).

## Desenvolvimento

A toolchain Rust 1.98.1 está fixada em `rust-toolchain.toml`; instale Rust via rustup.

```sh
cargo build --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```


- Comece por [AGENTS.md](AGENTS.md), mesmo usando uma ferramenta que não carregue esse arquivo automaticamente.
- [Roadmap canônico em Markdown](ROADMAP.md)
- [Estado de execução e próxima tarefa](docs/EXECUTION.md)
- [Plano visual: prioridades e ordem de implementação](docs/roadmap/index.html)
- [Fluxo de contribuição](CONTRIBUTING.md)
- [Instruções para agentes](AGENTS.md)
- [Decisões de arquitetura e processo](docs/decisions/0001-development-workflow.md)

O Orca é uma referência de pesquisa para captura de contexto e leitores de sessões: <https://github.com/stablyai/orca>. Nenhum código dele foi incorporado nesta etapa.
