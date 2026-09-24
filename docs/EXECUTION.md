# Estado de execução

Última atualização: 2026-09-24. Este arquivo é o ponto de retomada entre agentes. Não substitui a inspeção do Git nem os critérios do [roadmap](../ROADMAP.md).

## Situação atual

- Existe documentação de produto, workflow e um HTML de planejamento.
- CLI de inspeção e leitor Claude Code por arquivo explícito implementados; TUI, exportação e contas pendentes.
- Nome escolhido: Memory Pier (`memory-pier`). Núcleo em Rust, primeiro alvo macOS arm64 e leitor Claude Code escolhidos. Licença pendente.
- Cargo, toolchain Rust 1.98.1 e checks canônicos definidos no README; CI em macOS/Linux.
- Remoto: [lippdev/memory-pier](https://github.com/lippdev/memory-pier), público, após autorização do mantenedor.
- Documentação inicial integrada na `main` pelo PR #1.

## Próxima tarefa de produto

**Etapa 02 — Descoberta e seleção de sessões por projeto.** Usar o leitor existente
para pesquisar uma raiz configurável, conferir metadados do projeto e listar
sessões sem depender apenas do nome codificado do diretório. Cobrir erros de
acesso e subagentes com fixtures sintéticas. Definir seleção explícita de ramo
antes de exportar ou afirmar continuidade; certificar compatibilidade com uma
amostra controlada sem versionar conversas reais. Não iniciar etapa 03 ainda.

Referências: [decisão técnica atual](decisions/0004-rust-stack.md), [pesquisa de integrações](research/integrations.md), [contrato v1](specs/bundle-v1.md) e [cenário M1](research/m1-scenario.md).

O repositório de consulta de consumo continua aguardando envio do mantenedor; não bloqueia M1. Toolchain já preparada e fixada no projeto.

## Recorte entregue — bootstrap e inspeção por arquivo

Escopo autorizado: projeto Cargo, toolchain fixada, biblioteca de leitura Claude Code,
CLI de inspeção por caminho explícito, fixtures e CI. Aceite: preservar ordem e
proveniência; recuperar prefixos/linhas válidas com diagnósticos; distinguir ferramentas,
ramos e checkpoints; impor limites; não escrever na origem nem chamar modelos.
Descoberta por projeto permanece para a próxima entrega da etapa 02.

## Status do roadmap

| Etapa | Prioridade | Status | Evidência / pendência |
|---|---|---|---|
| 01 Pesquisa e contrato | P0 | Concluído | ADR 0003 substituído pelo ADR 0004, matriz de investigação, contrato v1, fixtures e cenário M1; compatibilidade real do leitor ainda não certificada. |
| 02 Leitor de sessão | P0 | Em andamento | Leitor por arquivo, CLI e testes implementados; descoberta, seleção de ramo e compatibilidade real pendentes. |
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

## Etapa 02 — Bootstrap Cargo e inspeção por arquivo

- Entrega concluída neste recorte: biblioteca Rust, CLI `memory-pier inspect`,
  toolchain 1.98.1, lockfile e workflow CI em macOS/Linux. Etapa 02 permanece em andamento.
- Leitura limitada ao tamanho observado na abertura, somente leitura, sem modelos.
  Proveniência por linha/bloco, UUID/parent e ordem física; ferramentas separadas;
  checkpoints identificados; ramos/subagentes sinalizados para seleção posterior.
- Fixtures sintéticas ampliadas e erros diagnosticados: truncamento, UTF-8 inválido,
  tipos desconhecidos, conteúdo omitido, metadados inconsistentes e limites.
- Validação local em macOS arm64: `cargo fmt --all -- --check`,
  `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`
  (18 testes), `cargo build --locked`, links Markdown, sintaxe JS do roadmap e
  `git diff --check`. Execuções e resultados remotos de CI vinculados ao
  [PR #4](https://github.com/lippdev/memory-pier/pull/4).
- Revisão: autorrevisão de diff, critérios e falhas; corrigidos diagnóstico de
  metadados inválidos, conteúdo vazio e preservação do indicador de erro de ferramentas.
  Sem revisão independente.
- Limitações: compatibilidade real não certificada; sem descoberta, seleção de ramo,
  exportação ou TUI. Snapshot não atômico, limites de 64 MiB/arquivo e 1 MiB/linha.
  Uso, saída e diagnósticos em [contrato do leitor](specs/claude-reader.md).
- Próximo passo: descoberta por projeto e seleção explícita, conforme tarefa acima.
