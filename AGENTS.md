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
- Pacote legível por pessoas e agentes sem exigir instalação do Memory Pier no destino.
- Distinguir registros extraídos de sínteses ou inferências; informar conteúdo omitido ou indisponível.
- Referenciar o estado do código e tratar alterações locais explicitamente. Não publicar código como efeito implícito de exportar contexto.
- Não versionar dados reais de conversas ou segredos. Usar fixtures sintéticas.

## Validação atual

Stack escolhida: Go; ainda não há módulo, build ou testes de aplicação. O contrato inicial está em `docs/specs/bundle-v1.md`, com schema em `schemas/` e exemplos sintéticos em `examples/` e `testdata/`. Para mudanças documentais, verificar links relativos e executar `git diff --check`. Atualize esta seção quando os comandos reais do projeto existirem.
