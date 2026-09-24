# 0004 — Núcleo em Rust

Status: aceito pelo mantenedor em 2026-09-24. Substitui a escolha de linguagem e direção de TUI do [ADR 0003](0003-core-stack.md).

## Contexto

Go foi escolhido inicialmente pela simplicidade de distribuição e implementação da CLI. Após discutir alternativas, o mantenedor escolheu Rust. Nenhum código de aplicação havia sido implementado, portanto não há migração de runtime ou dados.

## Decisão

Usar Rust para o núcleo e a CLI. Preparar projeto Cargo, fixar toolchain e dependências e versionar Cargo.lock na etapa 02. A biblioteca de TUI será avaliada na etapa 06; Bubble Tea deixa de ser a direção escolhida. Não adicionar framework de interface, runtime assíncrono ou banco antes de uma necessidade concreta.

Modelar explicitamente estados de leitura e erros: resultados parciais devem carregar diagnósticos; ausência de dados não deve parecer sucesso ou consumo zero. Tipos ajudam a expressar essas regras, mas não substituem validação de arquivos, testes ou revisão. Não há benchmark que demonstre vantagem de desempenho neste projeto. O custo aceito é maior complexidade inicial da implementação.

## O que permanece

- Primeiro alvo: macOS arm64; outros sistemas precisam de validação própria.
- Primeiro leitor: Claude Code JSONL, de somente leitura, com arquivo explícito antes de descoberta.
- Núcleo independente de TUI, credenciais, rede e serviços.
- Contrato do pacote v1, schema, fixtures e cenário M1 continuam válidos.
- Compatibilidade com registros reais ainda não certificada; nenhuma CLI funcional nesta entrega.

## Próxima entrega

Bootstrap Cargo e leitor testado contra fixtures normais, parciais, vazias e demais casos descritos na pesquisa de integrações. Documentar comandos reais de build, formatação, lint e testes quando existirem; não declarar checks executados antes da implementação.
