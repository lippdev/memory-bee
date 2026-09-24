# Estado de execução

Última atualização: 2026-09-24. Este arquivo é o ponto de retomada entre agentes. Não substitui a inspeção do Git nem os critérios do [roadmap](../ROADMAP.md).

## Situação atual

- Existe documentação de produto, workflow e um HTML de planejamento.
- Não existe CLI, TUI, leitor de sessões ou integração de contas implementada.
- Nome escolhido: Memory Pier (`memory-pier`). Núcleo em Rust, primeiro alvo macOS arm64 e leitor Claude Code escolhidos. Licença pendente.
- Não há comandos de build, lint ou testes de aplicação definidos.
- Remoto: [lippdev/memory-pier](https://github.com/lippdev/memory-pier), público, após autorização do mantenedor.
- Documentação inicial integrada na `main` pelo PR #1.

## Próxima tarefa de produto

**Etapa 02 — Leitor Claude Code.** Criar projeto Cargo com toolchain Rust fixada, CLI mínima de inspeção de arquivo e parser de somente leitura, usando as [fixtures sintéticas](../testdata/claude/README.md). Adicionar casos de ferramentas, ramos, compaction e erros; depois implementar descoberta por projeto. Configurar CI para testes do leitor, sem anunciar exportação funcional antes da etapa 03.

Referências: [decisão técnica atual](decisions/0004-rust-stack.md), [pesquisa de integrações](research/integrations.md), [contrato v1](specs/bundle-v1.md) e [cenário M1](research/m1-scenario.md).

O repositório de consulta de consumo continua aguardando envio do mantenedor; não bloqueia M1. Verificar e preparar a toolchain Rust antes de implementar.

## Status do roadmap

| Etapa | Prioridade | Status | Evidência / pendência |
|---|---|---|---|
| 01 Pesquisa e contrato | P0 | Concluído | ADR 0003 substituído pelo ADR 0004, matriz de investigação, contrato v1, fixtures e cenário M1; compatibilidade real do leitor ainda não certificada. |
| 02 Leitor de sessão | P0 | Pendente | Depende de 01. |
| 03 Exportação revisável | P0 | Pendente | Depende de 02. |
| 04 Estado do código | P0 | Pendente | Depende de 03; fecha M1. |
| 05 Troca de agente | P1 | Pendente | Depende de 03–04. |
| 06 Dashboard terminal | P1 | Pendente | Depende de 05. HTML existente é planejamento, não implementação. |
| 07 Uso e alertas | P1 | Pendente | Depende de 01 e 06; falta referência de consumo. |
| 08 Perfis de conta | P1 | Pendente | Depende de 05–06 e prova de isolamento. |
| 09 Captura contínua | P2 | Pendente | Depende de 02–06. |
| 10 Novos adaptadores | P2 | Pendente | Depende dos contratos e testes relevantes. |
| 11 Distribuição | P2 | Pendente | Pode acompanhar M1 para alpha; licença em aberto. |

## Registro de entregas documentais

### Bootstrap e plano visual

- `50d75f8`: escopo inicial e workflow em Markdown.
- `5632542`: HTML com prioridades, dependências e critérios de aceite.
- Verificações registradas: links locais, diff e sintaxe JavaScript do HTML. Aparência não validada em navegador.
- Revisão: autorrevisão; nenhuma revisão independente registrada.

### Guia operacional em Markdown

- Escopo: converter as 11 etapas do HTML para `ROADMAP.md`, conectar os pontos de entrada e registrar o estado de retomada.
- Resultado: regras mantidas em `CONTRIBUTING.md`, leitura inicial em `AGENTS.md` e roadmap canônico em Markdown; nenhuma funcionalidade de produto implementada.
- Validação: conferência dos 11 títulos, prioridades, dependências, entregas e critérios contra o HTML; links locais e `git diff --check`.
- Revisão: autorrevisão documental. Sem testes de aplicação, pois não há aplicação nesta entrega.
- Próximo passo: avaliação técnica descrita acima, quando o mantenedor solicitar continuidade do produto.

## Como manter este arquivo

Ao começar, registrar a entrega ativa e seus critérios. Ao encerrar, atualizar a linha da etapa e acrescentar um registro curto com resultado, verificações realmente executadas, tipo de revisão e pendências. Referenciar PR/commit quando existir, sem inventar identificadores. Não manter planos pessoais paralelos que o próximo agente não consiga consultar.

## Publicação inicial — Memory Pier

- Repositório privado criado em 2026-09-24 na conta `lippdev`.
- Nome atualizado no README, roadmap, instruções e HTML; decisão em `docs/decisions/0002-product-name.md`.
- Revisão: autorrevisão documental. Validação: links locais e `git diff --check`.
- Nenhuma funcionalidade de produto ou release de aplicação nesta entrega.

## Etapa 01 — Fundação técnica

- Decisão histórica (substituída pelo ADR 0004): Go para núcleo, macOS arm64 primeiro, Claude Code como primeiro leitor; TUI futura com Bubble Tea.
- Contrato v1 em Markdown e JSON Schema, exemplo somente contexto e três fixtures sintéticas.
- Validação: schema e exemplo, hashes, rejeição de versão inválida, fixtures válidas/truncadas/vazias, links locais e diff.
- Revisão: autorrevisão. Sem leitura de chats reais, sem chamadas a modelos, sem exportador ou teste fim a fim ainda.
- A versão instalada do Claude Code é 2.1.281; isso não certifica compatibilidade de parsing.
- Próximo passo: etapa 02, descrita acima.

## Escolha de Rust

- Mantenedor escolheu Rust explicitamente; ADR 0004 substitui a decisão de Go sem apagar seu histórico.
- README, AGENTS, roadmap e HTML alinhados. Contrato e fixtures preservados.
- Validação: links locais, sintaxe JS do HTML e git diff --check. Autorrevisão documental.
- Nenhum build ou teste Rust executado: projeto Cargo ainda será criado na etapa 02.
