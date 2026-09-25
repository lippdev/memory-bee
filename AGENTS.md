# Instruções para agentes

Este é o ponto de entrada para qualquer agente. O projeto é uma CLI com dashboard em terminal para portabilidade de contexto entre agentes e pessoas; não criar uma IDE como requisito de uso.

## Leitura inicial e fonte de verdade

1. [README.md](README.md): visão e estado geral do produto.
2. [CONTRIBUTING.md](CONTRIBUTING.md): workflow, autonomia, Git, revisão e validação.
3. [ROADMAP.md](ROADMAP.md): prioridades, dependências e critérios de aceite.
4. [docs/EXECUTION.md](docs/EXECUTION.md): status, evidências e próximo passo.
5. [Decisões](docs/decisions/0001-development-workflow.md): ler os ADRs relevantes à tarefa.

Não presumir acesso às conversas anteriores. O Markdown é a referência operacional; o HTML é uma apresentação complementar. Se sua ferramenta não carregar AGENTS.md automaticamente, ler esses arquivos explicitamente. Uma instrução atual do mantenedor pode mudar o plano: registrar a mudança nos documentos afetados, sem tratar propostas antigas como decisões definitivas.

## Retomada e encerramento

- Antes de editar, conferir branch, alterações locais, histórico recente e remoto. O estado do Git deve ser observado, não inferido deste documento.
- Executar a tarefa solicitada; para um pedido aberto de continuação, usar o próximo passo em `docs/EXECUTION.md`, respeitando dependências do roadmap.
- Dividir etapas grandes em entregas revisáveis. Escolhas rotineiras dentro do escopo não exigem nova aprovação; decisões de arquitetura precisam de justificativa registrada.
- A cada implementação, atualizar `docs/MANUAL_TESTS.md` com comandos, resultados esperados e pendências para o mantenedor testar depois; não marcar ensaios manuais como executados por terem testes automatizados.
- Ao encerrar trabalho de produto, atualizar status, evidências, limitações e próximo passo em `docs/EXECUTION.md`. Preservar o registro de trabalhos anteriores.
- Não marcar funcionalidades como prontas por haver apenas planejamento, mockup ou documentação. Não iniciar outras fases quando o pedido é apenas revisar ou documentar o plano.

## Forma de trabalhar

- Atue com as etapas de uma equipe: planejamento, implementação, revisão e validação, com registros proporcionais à mudança.
- Preserve autoria real. Não invente integrantes, aprovações ou revisões independentes. Identifique autorrevisões.
- Há autonomia para commits, push, PRs, merge e releases dentro do escopo acordado, respeitando verificações, permissões da plataforma e trabalho preexistente.
- Após o commit inicial de documentação, use branches `codex/<assunto>` e PRs quando houver remoto. Integre por squash; não reescreva branches compartilhadas.
- Registre decisões duradouras em `docs/decisions/`. Não escolha uma stack ou amplie o escopo sem apresentar a justificativa.
- Relate o que foi alterado, como foi validado e limitações reais. Não declare CI, testes, PRs ou publicação como concluídos sem evidência.

## Invariantes do produto

- Exportação básica sem chamadas a modelos, inclusive quando a origem não consegue mais responder.
- Pacote legível por pessoas e agentes sem exigir instalação do Memory Bee no destino.
- Distinguir registros extraídos de sínteses ou inferências; informar conteúdo omitido ou indisponível.
- Referenciar o estado do código e tratar alterações locais explicitamente. Não publicar código como efeito implícito de exportar contexto.
- Não versionar dados reais de conversas ou segredos. Usar fixtures sintéticas.

## Validação atual

Stack: Rust 1.98.1 fixada, projeto Cargo com biblioteca e CLI de inspeção.
Comandos canônicos: `cargo build --locked`, `cargo fmt --all -- --check`,
`cargo clippy --locked --all-targets -- -D warnings` e `cargo test --locked`.
CI executa esses checks em macOS e Linux, além de `python scripts/check_bundle.py`
(com dependências de `scripts/requirements-validation.txt`) para schema e hashes. O leitor usa fixtures sintéticas;
descoberta por projeto e seleção de ramo implementadas, compatibilidade real pendente.
Exportação de contexto v1 com referência Git opcional (`--project`) e código selecionado v2 (`--include-path`) implementadas; `verify` confere pacotes e `apply --check/--write` exige checkout limpo/base exata; roteiro manual em `docs/MANUAL_TESTS.md`.
Mantenedor adiou ensaios manuais e autorizou continuar implementações; registrar
pendências sem usá-las como bloqueio automático (ADR 0006).
Contrato de inspeção em `docs/specs/claude-reader.md`; contrato do pacote em
`docs/specs/bundle-v1.md` e `docs/specs/bundle-v2.md`, com schemas em `schemas/` e exemplos em `examples/`.
Para documentação, verificar links relativos e executar `git diff --check`.

Segundo leitor: `inspect-codex <rollout.jsonl>` experimental, apenas arquivo explícito;
contrato em `docs/specs/codex-reader.md`. Exportação Codex via `export-codex --preview/--output`, com exclusões e Git/código
selecionado opcionais, implementada. Descoberta via `sessions-codex --root <pasta>
--project <projeto>` implementada com limites e metadados; lançamento pendente.
Contrato em `docs/specs/codex-discovery.md` (ADR 0012).
Contrato de exportação em `docs/specs/codex-export.md` (ADR 0011).
Preparação de retomada via `prepare-resume <pacote> --target claude|codex --project
<projeto> [--worktree <pasta>] (--preview | --output <arquivo>)` implementada;
`--launch <confirmação>` inicia o agente só quando o token da prévia confere
(ADR 0014). Contrato em `docs/specs/resume-preparation.md` (ADR 0013).
