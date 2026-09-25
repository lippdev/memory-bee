# Estado de execução

Última atualização: 2026-09-25. Este arquivo é o ponto de retomada entre agentes. Não substitui a inspeção do Git nem os critérios do [roadmap](../ROADMAP.md).

## Situação atual

- Existe documentação de produto, workflow e um HTML de planejamento.
- CLI de inspeção, descoberta, seleção e exportação somente contexto implementadas; referência Git opcional implementada; código selecionado v2 implementado; verify/apply explícito implementados; inspeção/exportação Codex por arquivo e descoberta por projeto sob raiz explícita implementadas, experimentais; preparação de retomada Claude/Codex e lançamento confirmado por token implementados, experimentais; dashboard de terminal (ratatui) com lista, detalhe, exportação de contexto/exclusões, aplicação e prompt/lançamento confirmado implementados; workspace unificado com demo explícita e primeiro modo Claude local sobre CLI oficial/PTY, experimental (ADR 0018); cena animada, exportação nativa pela TUI e perfis reais pendentes.
- Nome decidido: **Memory Bee** (`memory-bee`), com mascote abelha e colmeia dos projetos, conforme o [ADR 0015](decisions/0015-memory-bee-identity.md); crate, binário, pacotes e documentos renomeados para `memory-bee` (registros anteriores mantêm o nome da época). Núcleo em Rust, primeiro alvo macOS arm64 e leitor Claude Code escolhidos. Licença pendente.
- Cargo, toolchain Rust 1.98.1 e checks canônicos definidos no README; CI em macOS/Linux.
- Remoto atual: [lippdev/memory-bee](https://github.com/lippdev/memory-bee), público.
  Em 2026-09-25 o push confirmou redirecionamento da URL local antiga
  `lippdev/memory-pier`; links históricos foram preservados.
- Documentação inicial integrada na `main` pelo PR #1.

## Auditoria e rastreamento de issues — 2026-09-25

- Inventário do roadmap, contratos, roteiro manual, código e CI convertido em
  [23 issues #25–#47](https://github.com/lippdev/memory-bee/issues?q=is%3Aissue+is%3Aopen):
  2 P0, 17 P1 e 4 P2. Não havia issues preexistentes. Labels de prioridade,
  área e validação separam implementação de prova humana.
- [#25](https://github.com/lippdev/memory-bee/issues/25) e
  [#26](https://github.com/lippdev/memory-bee/issues/26) rastreiam a prova real
  do leitor Claude e do M1. A sequência Claude primeiro está em
  [#29](https://github.com/lippdev/memory-bee/issues/29) (prompts ocultos),
  [#30](https://github.com/lippdev/memory-bee/issues/30) (permissões reais) e
  [#31](https://github.com/lippdev/memory-bee/issues/31) (histórico/exportação).
  [#47](https://github.com/lippdev/memory-bee/issues/47) reúne o aceite do
  lançamento conjunto Claude/Codex e liga as dependências principais.
- A [CI da `main` no commit `c7f432b`](https://github.com/lippdev/memory-bee/actions/runs/36177917324)
  passou em macOS e Linux. A auditoria não executou ensaios humanos nem validou
  agentes reais adicionais; criar issues não altera o status das etapas abaixo.
  Autorrevisão do inventário; sem revisão independente.
- Próximo recorte de implementação: #29, seguido da prova de permissões #30;
  os ensaios P0 #25–#26 podem avançar em paralelo quando o mantenedor os fizer.

## Revisão automática de PRs — 2026-09-25

- Pullfrog já estava ativo para PRs novos e novos commits; configuração inicial
  tinha `prompts.review` vazio, aprovação automática e auto-merge desligados.
- Instruções específicas de revisão foram versionadas em
  [`.github/pullfrog/review.md`](../.github/pullfrog/review.md). O procedimento
  de sincronização está no [guia de contribuição](../CONTRIBUTING.md), e o
  ensaio humano permanece pendente no item 53 do [roteiro](MANUAL_TESTS.md).
- `npx --yes pullfrog@latest config set prompts.review --file
  .github/pullfrog/review.md --repo lippdev/memory-bee` retornou `updated`; `get`
  devolveu o texto completo. Validar o primeiro parecer no PR e a revisão
  incremental em PR futuro; isso não altera o status das etapas do produto.
  Autorrevisão documental; sem revisão independente registrada nesta entrega.

## Revisão de sequência — orientação do mantenedor

Em 2026-09-24, o mantenedor pediu continuidade das implementações e deixou os
ensaios manuais para depois, um por um. A validação controlada de compatibilidade
da etapa 02 permanece pendente e deixa de bloquear a implementação da etapa 03.
Isso não certifica suporte real nem dispensa testes automatizados e CI.

Recorte autorizado nesta entrega: exportador somente contexto, com prévia, exclusão de linhas,
HANDOFF/manifesto/histórico v1, hashes, diagnósticos e detecção básica de possíveis
segredos. Aceite: pacote legível sem instalar o produto, proveniência preservada,
sem modelos, sem sobrescrever destino e sem inferir estado do código. Exportação
de patches e consulta ao Git ficam para a etapa 04.

## Próxima tarefa de produto

**Claude invisível na TUI Bee — próximo recorte de produto.** O mantenedor
rejeitou a exibição da tela nativa do Claude. `workspace --claude` agora hospeda
a CLI interativa em PTY oculto e usa hooks para mostrar texto, ações e permissões
na Bee. Uma resposta real curta foi recebida na UI; negação/aprovação e retomada
foram testadas com CLI falsa. Próximo: ensaio humano de ferramentas/permissões
reais, tratamento de prompts nativos sem hook e histórico/exportação revisável.
O modo ainda guarda só IDs; perfis reais e Codex integrado seguem pendentes.
O lançamento da experiência completa ainda exige os dois agentes (ADR 0018).

**Refinamento visual.** Referência HTML e [plano visual](specs/workspace-visual.md)
concluídos; renderizador Rust, abelha por ação e animações de exportação/troca
permanecem pendentes. O HTML apresenta o plano visual anterior à prioridade
Claude primeiro; este registro e o roadmap Markdown prevalecem.

A renomeação no código está feita (seção "Renomeação — memory-bee"). O GitHub
já informa `lippdev/memory-bee`; a URL local antiga redireciona. Não foi feita
renomeação remota nesta retomada.

Compatibilidade real de Claude Code e ensaios de leitura/retomada do pacote estão
pendentes com o mantenedor em [docs/MANUAL_TESTS.md](MANUAL_TESTS.md), sem bloquear
esta próxima implementação, conforme a revisão de sequência acima.

Referências: [decisão técnica atual](decisions/0004-rust-stack.md), [pesquisa de integrações](research/integrations.md), [contrato v1](specs/bundle-v1.md) e [cenário M1](research/m1-scenario.md).

O repositório de consulta de consumo continua aguardando envio do mantenedor; não bloqueia M1. Toolchain já preparada e fixada no projeto.

## Recorte entregue — bootstrap e inspeção por arquivo

Escopo autorizado: projeto Cargo, toolchain fixada, biblioteca de leitura Claude Code,
CLI de inspeção por caminho explícito, fixtures e CI. Aceite: preservar ordem e
proveniência; recuperar prefixos/linhas válidas com diagnósticos; distinguir ferramentas,
ramos e checkpoints; impor limites; não escrever na origem nem chamar modelos.
Descoberta por projeto permanece para a próxima entrega da etapa 02.

## Recorte entregue — descoberta e seleção

Escopo: listar JSONL sob raiz explícita por `cwd` observado; manter subagentes
separados; diagnosticar arquivos não classificáveis, erros e limites. Selecionar
ponta por UUID com cadeia de parentesco verificável, preservando diagnósticos e
registrando exclusões. Aceite: fixtures sintéticas de projetos homônimos,
subagentes, ramos válidos/ambíguos, falhas e limites; CLI e checks canônicos.
Orientação daquele recorte: compatibilidade real seria validada antes do exportador;
substituída pela revisão de sequência do mantenedor registrada acima.

## Status do roadmap

| Etapa | Prioridade | Status | Evidência / pendência |
|---|---|---|---|
| 01 Pesquisa e contrato | P0 | Concluído | ADR 0003 substituído pelo ADR 0004, matriz de investigação, contrato v1, fixtures e cenário M1; compatibilidade real do leitor ainda não certificada. |
| 02 Leitor de sessão | P0 | Em andamento | Leitor, descoberta e seleção de ramo testados com fixtures; validação controlada de compatibilidade real pendente. |
| 03 Exportação revisável | P0 | Em andamento | Exportador somente contexto com prévia, exclusões e testes implementado; ensaios manuais de pacote/retomada pendentes. |
| 04 Estado do código | P0 | Em andamento | Referência Git, código selecionado, verify e apply explícito implementados; ensaios manuais/M1 pendentes. |
| 05 Troca de agente | P1 | Em andamento | Inspeção/exportação Codex, descoberta, preparação de retomada e lançamento confirmado implementados, experimentais; validação real com os agentes pendente (itens 38–40). |
| 06 Dashboard terminal | P1 | Em andamento | Leitura e ações confirmadas implementadas; cena, exportação de código pela TUI e ensaios humanos pendentes. |
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

## Etapa 02 — Descoberta e seleção de ramo

- Entrega concluída neste recorte: `sessions --root ... --project ...` lista arquivos
  por `cwd` observado; `inspect ... --leaf <uuid>` seleciona ancestrais de uma ponta
  verificável. Subagentes permanecem separados. ADR 0005 registra as decisões.
- Busca limitada e somente leitura; erros, metadados ausentes/conflitantes, symlinks
  e limites geram diagnósticos. Seleção mantém perdas originais e conta exclusões;
  recusa parentesco ausente/duplicado/futuro e mistura de sessão/agente.
- Validação local macOS arm64: 34 testes (`cargo test --locked`), Clippy sem warnings,
  build, formatação, links Markdown, sintaxe JS do roadmap e `git diff --check`.
  Demonstração de descoberta e seleção executada na árvore sintética versionada.
  CI remoto e integração vinculados ao [PR #5](https://github.com/lippdev/memory-pier/pull/5).
- Revisão: autorrevisão de limites, identidade, omissões, caminhos e falhas. Corrigida
  consulta quadrática de diagnósticos por registro e sinalização de arquivos
  parciais fora do projeto pesquisado; testes ajustados para caminhos
  canonicalizados do macOS e ordenação por componentes. Sem revisão independente.
- Limitações: formato real não certificado, comparação lexical de projetos,
  layout de busca delimitado e árvore sem snapshot atômico. Seleção exige cadeia
  verificável e pode recusar sessões compactadas. Nenhuma conversa pessoal lida.
- Próximo passo: validação controlada de compatibilidade descrita acima; etapa 02
  permanece em andamento e exportação continua pendente.

## Etapa 03 — Exportador somente contexto

- Implementação concluída neste recorte: `export --preview` ou `--output`, escolha
  de ramo, exclusões por linha, HANDOFF legível, history JSONL e manifesto v1 com
  SHA-256. Conteúdo extraído e checkpoints permanecem distintos, sem síntese/modelos.
- Perdas/seleção/exclusões registradas; trechos iniciais limitados sem truncar
  histórico retido. Estado Git explicitamente desconhecido. Caminhos cwd e da
  origem não são copiados automaticamente como metadados.
- Detecção básica de possíveis segredos em texto e metadados, com localização sem
  ecoar valor nos achados; gravação bloqueada se houver suspeitas. Revisão humana
  sempre pendente. Destino exclusivo, permissões privadas no Unix e manifesto por
  último; em erro comum, limpeza só de arquivos criados pela operação.
- Validação local em macOS arm64: 49 testes Rust, fmt, Clippy e build com lockfile;
  schema v1 e hashes independentes em Python para quatro pacotes sintéticos
  (normal, filtrado, truncado e ferramentas). Links Markdown, sintaxe JS e diff
  conferidos. CI passa a executar a mesma validação de schema.
- Revisão: autorrevisão de privacidade, proveniência, falhas e contrato. Verificada
  recusa de destinos existentes/symlink, limpeza parcial preservando arquivo alheio,
  referências parentais após exclusão e cercas Markdown para texto histórico.
  Sem revisão independente e sem execução dos ensaios pessoais do mantenedor.
- Dependências justificadas no ADR 0006: sha2, regex e jsonschema apenas para checks
  de desenvolvimento/CI. Compatibilidade real, revisão manual de pacotes, referências
  Git e patches continuam pendentes. O marco M1 não está concluído.
- Próximo passo de implementação: referência Git, descrita acima. Roteiro manual
  separado permite testar as funcionalidades uma por uma posteriormente.

### Retomada e fechamento da exportação

- Sessão anterior deixou o commit `0579a7f` publicado na branch, sem PR.
  Retomada conferiu árvore limpa, implementação, contrato e testes; nenhuma
  alteração de comportamento foi necessária.
- Reexecutados com sucesso: 49 testes Rust, build, fmt, Clippy, validação Python
  de quatro pacotes (schema, proveniência, exclusões e hashes), links Markdown
  e `git diff --check`. Revisão feita pelo próprio agente, sem revisão independente.
- CI remoto e integração vinculados ao [PR #6](https://github.com/lippdev/memory-pier/pull/6).
  Ensaios manuais continuam pendentes; próxima entrega permanece referência Git.

## Etapa 04 — Referência Git explícita

- Recorte: `export --project <pasta>` observa origin, branch, commit e dirty,
  registrando estado e avisos no manifesto v1 e HANDOFF. Sem opção, não consulta Git.
  ADR 0007 documenta CLI Git local, privacidade, limites e semântica.
- Não publica nem inclui código. URLs sensíveis/locais são omitidas; branch/remoto
  passam pela detecção existente. Erros deixam campos desconhecidos e saída parcial;
  detached HEAD e repositório sem commit têm representações distintas.
- Validação local: 58 testes Rust, build, fmt, Clippy; Python valida cinco pacotes
  contra schema v1 e hashes independentes, incluindo base Git sintética. Casos
  incluem índice corrompido, Git ausente, conflitos, worktree e ambiente GIT_*.
- Autorrevisão de falhas, privacidade e compatibilidade do contrato; sem revisão
  independente. Índice e arquivo fonte preservados no teste de observação limpa.
- Limites: sem snapshot atômico, timeout ou verificação remota; apenas origin;
  arquivos ignorados não contam em dirty. Ensaios manuais continuam pendentes.
- Próximo passo: alterações selecionadas e aplicação explícita, conforme acima.
- CI remoto e integração vinculados ao [PR #7](https://github.com/lippdev/memory-pier/pull/7).
  Links Markdown, sintaxe JavaScript do roadmap e `git diff --check` também passaram.

## Etapa 04 — Exportação de alterações selecionadas

- Recorte: `--include-path` repetível, com projeto/base explícitos. Patches de
  arquivos da base e arquivos novos mapeados em files/. Manifesto v2 separado,
  mantendo v1 para contexto. Modos e SHA-256 antes/depois; nenhum código aplicado.
- Leitura de blobs/disco sem filtros Git, seleção literal, limites, exclusão de
  tipos não suportados e detector de segredos incluindo texto removido. Omissões
  geram saída parcial, sem afirmar captura completa do estado local.
- Validação local: 69 testes Rust, build, fmt, Clippy e sete pacotes verificados
  em Python (schemas v1/v2, metadados e hashes). Reconstrução sintética com git apply
  em clone separado confere bytes, exclusões, nomes com espaço, CRLF, falta de
  newline, arquivos vazios e modo executável; origem/índice preservados.
- Autorrevisão: desabilitados filtros de status que poderiam executar configuração
  local; testes cobrem filtros na raiz e submódulos. Submódulos não são percorridos; estado interno é explicitamente não verificado. Sem revisão independente.
- Limitações: nomes ASCII restritos, 64 seletores, 1 MiB por versão, 8 MiB total;
  sem snapshot atômico, timeout, binários, submódulos, filtros ou importador.
  Staging não é preservado; captura é disco contra base. ADR 0008 detalha decisões.
- Pedido persistente do mantenedor registrado em AGENTS: atualizar sempre roteiro
  manual. Itens 10–16 adicionados com preparação e expectativas; numeração anterior
  corrigida. Ensaios manuais continuam pendentes; M1 não concluído.
- Próximo recorte: recebimento e aplicação explícita, descrito acima.
- Verificações documentais: links locais, JS do roadmap, numeração e sintaxe shell
  dos exemplos do roteiro manual, além de `git diff --check`, aprovados.

### Publicação pendente — exportação selecionada

- Implementação e validação local concluídas no commit `cca4b6e`, branch
  `codex/selected-changes`. Integração ainda pendente: três tentativas de push HTTPS
  em 2026-09-24 receberam `remote: Internal Server Error` do GitHub. A consulta à
  API confirmou permissão de push; a branch não apareceu no remoto.
- Alternativa SSH não utilizada com sucesso: `Host key verification failed`.
  Nenhuma configuração de confiança SSH foi alterada.
- PR não criado e CI remoto não executado para esta entrega. Não declarar merge.
  Próxima ação operacional: publicar a branch, abrir PR, conferir CI macOS/Linux
  e integrar por squash antes de iniciar a próxima implementação.

### Retomada da publicação

- Após novo pedido do mantenedor, push HTTPS concluído com sucesso. Os erros
  anteriores ficaram resolvidos, sem alteração de configuração SSH.
- Publicação e verificações remotas vinculadas ao [PR #8](https://github.com/lippdev/memory-pier/pull/8).
  Nenhuma mudança de implementação nesta retomada; roteiro manual 10–16 preservado.

## Etapa 04 — Recebimento e aplicação explícita

- Recorte: verify de pacotes v1/v2 suportados; apply --check sem escrita e --write
  explícito em checkout limpo/HEAD exato. Verificações de campos, hashes, layout,
  caminhos, mapeamentos, base e conflitos; rejeição de symlinks/colisões/extensões.
- Patch decodificado e reconstituído canonicamente; não executa Git apply recebido,
  hooks, filtros ou histórico. Arquivos de resultado verificados; índice/commit
  preservados. Contagens de omissões/avisos continuam nos relatórios.
- Aplicação mantém originais em staging privado e tenta rollback em erro comum.
  Interrupção abrupta pode exigir recuperação manual via recovery.json/old-N;
  sem atomicidade multiarquivo ou garantia contra escritores concorrentes.
- Validação local: 81 testes Rust, build, fmt, Clippy; roundtrip, adulteração mesmo com hash recalculado,
  versões/campos/limites, colisões ignoradas, base divergente, mudança após check,
  symlinks, modos/vazios e não execução de filtros. Falha injetada valida rollback
  e preservação de índice. Python confere schemas/hashes e verify/check/write.
- Revisão: autorrevisão; sem revisão independente. ADR 0009 documenta limites:
  perfil estrito de patch, Unix/hard links, checkout limpo, sem submódulos,
  permissões privadas e recuperação não automática após interrupção abrupta.
- Roteiro manual ampliado com itens 17–22 e expectativas. Ensaios humanos continuam
  pendentes; etapa 04 e M1 não declarados concluídos por testes automatizados.
- Próximo passo: segundo leitor com contrato/fixtures, conforme tarefa acima.
- Links Markdown, sintaxe JS do roadmap, numeração 1–22 e sintaxe shell do roteiro
  manual conferidos, além de `git diff --check`.
- CI remoto e integração vinculados ao [PR #9](https://github.com/lippdev/memory-pier/pull/9).

## Etapa 05 — Inspeção Codex explícita

- Comando inspect-codex e relatório independente da árvore Claude. Mensagens,
  papéis, ferramentas, blocos, IDs, fase e sessão/turno conhecidos preservados.
  Checkpoints identificados; event_msg/reasoning/tipos futuros/compactações geram
  diagnósticos explícitos. Sem execução, modelos, descoberta ou exportação Codex.
- Perfil experimental documentado antes da implementação; fontes oficiais de hooks
  e App Server e schema ResponseItem gerado localmente consultados. Envelope JSONL
  não certificado como API estável. Nenhuma conversa real lida.
- Autorrevisão: cabeçalhos inválidos limpam identidade anterior; texto idêntico
  não é deduplicado; replacement_history não é reproduzido como conversa.
  Sem revisão independente. Roteiro manual ampliado com itens 23–26, todos pendentes.
- Limitações: compatibilidade unverified, registro físico sem reconstrução de
  forks/rollback; parcial esperado com registros auxiliares/configuração adicional;
  limites e snapshot não atômico iguais aos do leitor inicial.
- Próximo passo: adaptação explícita para exportação Codex, descrita acima.
- Validação local: 95 testes Rust (14 novos), build, fmt e Clippy aprovados;
  validação Python dos pacotes v1/v2 e hashes passou. Links Markdown, JS do roadmap,
  numeração 1–26 e sintaxe shell do roteiro manual conferidos; git diff --check limpo.
- CI remoto e integração vinculados ao [PR #10](https://github.com/lippdev/memory-pier/pull/10).

## Etapa 05 — Exportação Codex revisável

- Recorte: export-codex por arquivo explícito, prévia/exclusões, pacotes v1/v2
  com eventos nativos Codex e avisos do perfil experimental. ADR 0011 registra
  adaptação ao núcleo existente; sem descoberta, seleção Claude ou lançamento.
- Aceite: proveniência e perdas preservadas, detecção de segredos nos metadados,
  verify e roundtrip de código selecionado com fixtures sintéticas; nenhuma escrita
  na origem ou inferência de projeto.
- Implementação concluída neste recorte: preview/output, exclusões, referência Git
  v1 e código selecionado v2; histórico mantém todos os campos do leitor Codex,
  renumerando apenas sequence. verify aceita os pacotes sem alterar o receptor.
- Validação local: 103 testes Rust (8 novos), build, fmt e Clippy aprovados. Python
  valida 13 pacotes sintéticos Claude/Codex com schemas v1/v2, hashes independentes,
  exclusões e verify; roundtrip Codex aplica código em clone sintético separado.
  Links Markdown, sintaxe JS, numeração 1–30/sintaxe shell do roteiro e diff conferidos.
- Autorrevisão da adaptação, seleção, privacidade e contrato; sem revisão independente.
  Detecção agora percorre todas as strings serializadas dos eventos, incluindo fase,
  namespace e turno, preservando localização dos achados. Testes cobrem referências
  sensíveis herdadas mesmo depois da exclusão de uma mensagem.
- Roteiro manual ampliado com itens 27–30, todos pendentes. Compatibilidade real,
  revisão humana e M1 permanecem pendentes; registro físico não reconstrói forks ou
  rollback. Sem novas dependências, modelos, descoberta ou lançamento.
- Próximo passo: descoberta Codex por projeto sob raiz explícita, descrita acima.
- Publicação e verificações remotas vinculadas ao [PR #11](https://github.com/lippdev/memory-pier/pull/11).

## Etapa 05 — Descoberta Codex por projeto

- Recorte: sessions-codex com raiz/projeto explícitos, travessia limitada e
  classificação pelo leitor Codex experimental. ADR 0012 registra layout,
  identidade, reutilização dos orçamentos e limites da associação por metadados.
- Aceite: resultados separados por arquivo, perdas e ambiguidades visíveis,
  limites/symlinks/erros cobertos por fixtures; caminho selecionado utilizável em
  inspect-codex/export-codex/verify; nenhuma escrita, busca de perfis ou lançamento.
- Implementação concluída neste recorte: busca em .jsonl na raiz e até três níveis,
  associação lexical por cwd, uma sessão conhecida por arquivo e resultados separados
  por caminho, mesmo com IDs repetidos. Listagem não contém texto da conversa.
- Validação local macOS: 116 testes Rust (13 novos), build, fmt e Clippy aprovados;
  Python confere os 13 pacotes sintéticos existentes com schemas/hashes/verify/apply.
  Fluxo automatizado novo cobre descoberta → inspect-codex → export-codex → verify.
  Links Markdown, JS, numeração 1–34/sintaxe shell do roteiro e diff conferidos.
- Autorrevisão de classificação, limites e regressão Claude; sem revisão independente.
  O código compartilhado mantém o layout Claude e seus testes; agent é campo aditivo
  da descoberta. partial_file só representa estado parcial do leitor, sem confundir
  ausência de metadados com corrupção do arquivo. IDs e projetos conflitantes não
  são apresentados como sessão selecionável.
- Teste de nomes não UTF-8 encontrados na árvore é específico de Linux: o sistema
  de arquivos macOS local recusou sua criação. Argumentos não UTF-8 foram testados
  localmente. Casos de permissões só afirmam recusa quando o processo realmente
  perde acesso; não presumem isso em execução privilegiada.
- Roteiro manual ampliado com itens 31–34, todos pendentes. Compatibilidade real,
  aliases entre plataformas, snapshot atômico, identificação de forks/subagentes e
  lançamento não são certificados. Nenhuma conversa real lida nesta entrega.
- Próximo passo: preparação explícita de retomada Claude/Codex (entregue na seção seguinte).
- Publicação e verificações remotas vinculadas ao [PR #12](https://github.com/lippdev/memory-pier/pull/12).

## Etapa 05 — Preparação explícita de retomada

- Recorte: prepare-resume verifica o pacote, observa o projeto explícito e gera
  prompt e passos manuais para Claude ou Codex. Mesmo checkout e nova worktree são
  modos separados por --worktree. ADR 0013 registra interfaces observadas, modos,
  formato dos passos e alternativas descartadas.
- Pesquisa de interfaces: ajuda local de Claude Code 2.1.282 e Codex CLI 0.156.1
  em 2026-09-24. Nenhuma CLI foi executada com prompt; flags não certificadas.
- Aceite: pacote inválido recusado; HEAD, sujeira e estado das mudanças v2
  (base/aplicadas/mistas/desconhecidas) visíveis como pontos de atenção; apply só
  sugerido com pré-condições observadas; prompt sem texto do histórico/manifesto;
  nenhuma escrita além do arquivo de prompt novo 0600 fora do pacote.
- Validação local macOS: 124 testes Rust (8 novos), build, fmt e Clippy aprovados;
  Python confere os pacotes sintéticos com schemas/hashes. Teste de integração
  executa os passos impressos do modo nova worktree (git worktree add, apply
  --check/--write) e confere o resultado. Comandos dos itens 35–38 conferidos pelo
  agente em pasta temporária; isso não substitui o ensaio do mantenedor.
- Autorrevisão de segurança dos passos, quoting POSIX, ordem de argumentos do
  Claude (--add-dir variádico) e ausência de escrita; sem revisão independente.
- Limitações: observação não atômica; checkout sujo com mudanças aplicadas não
  distingue alterações extras fora da seleção; leitura do pacote pelo Codex depende
  da sandbox; --add-dir do Claude dá acesso de ferramenta ao pacote. Roteiro manual
  ampliado com itens 35–38, pendentes. Nenhuma conversa real lida.
- Próximo passo: lançamento explícito opcional (entregue na seção seguinte).
- Publicação e verificações remotas vinculadas ao [PR #13](https://github.com/lippdev/memory-pier/pull/13).

## Etapa 05 — Lançamento confirmado

- Recorte: prepare-resume --output <arquivo> --launch <confirmação> inicia o agente
  do passo final. ADR 0014 registra token, pré-condições, códigos de saída e
  alternativas descartadas (pergunta interativa, --yes sem vínculo, encadeamento).
- Aceite: a prévia informa confirmation; o lançamento refaz verificação e
  observação e recusa token divergente antes de gravar. Só modo mesmo checkout sem
  passos pendentes. Prompt gravado antes de iniciar; argv sem shell, terminal
  herdado. Falha ao iniciar (1) ou saída não zero do agente (4) preservam pacote,
  prompt e origem.
- Validação local macOS: 129 testes Rust (5 novos, com agentes falsos no PATH),
  build, fmt e Clippy aprovados; validação Python dos pacotes aprovada. Comandos
  dos itens 35, 36 e 39 conferidos pelo agente em pasta temporária; isso não
  substitui o ensaio do mantenedor. Links relativos e git diff --check conferidos.
- Autorrevisão do vínculo do token, da ordem recusa/gravação/lançamento e dos
  caminhos de falha; sem revisão independente.
- Limitações: token detecta mudança, mas não é segredo nem assinatura; janela entre
  observação e início do agente; nenhum agente real foi lançado nesta entrega.
  Roteiro manual ampliado com itens 39–40, pendentes.
- Próximo passo: primeiro recorte do dashboard terminal descrito em "Próxima tarefa".
- Publicação e verificações remotas vinculadas ao [PR #14](https://github.com/lippdev/memory-pier/pull/14).

## Identidade — Memory Bee e protótipo da colmeia

- Recorte: decisão de nome e identidade visual registrada no
  [ADR 0015](decisions/0015-memory-bee-identity.md), que substitui o ADR 0002, e
  protótipo HTML privado [Clareira da Colmeia](https://claude.ai/artifact/W5a8EAd4UKXLS84Nsqa8Ab)
  com a cena da etapa 06: clareira ao entardecer, árvore com a colmeia pendurada,
  um gominho por projeto com mel por contexto, abelha que voa até o gominho
  selecionado, grama e flores. Sem Rust nesta rodada; nada muda em código,
  schemas ou pacotes.
- Aceite parcial: o protótipo é a especificação visual e de interação do dashboard
  (colmeia como seletor, lista e detalhe abaixo, degradações 100×30, 60×20 e 40×12,
  modos 256, `NO_COLOR`, glifos básicos e movimento reduzido). A aprovação do
  visual pelo mantenedor continua pendente; o protótipo anterior (farol/Hayate)
  fica arquivado fora do repositório.
- Validação local: sintaxe do script conferida com `node --check`; execução sem
  navegador (DOM simulado) por 300 quadros em 4 tamanhos × 2 paletas × 3 modos de
  cor com sequência de teclas, mais 900 quadros ociosos para o sono e o despertar
  da abelha, sem erro; quadros renderizados em PNG e revisados pelo agente. A
  animação no navegador não foi vista pelo agente. Links relativos e
  `git diff --check` conferidos.
- Autorrevisão do desenho e da cena; sem revisão independente.
- Limitações: dados sintéticos; o protótipo mostra vários projetos, enquanto a
  CLI atual descobre por um `--project` de cada vez, o que a etapa 06 precisa
  cobrir; nenhuma verificação de marca ou domínio para o nome.
- Próximo passo: PR de renomeação para `memory-bee` (inventário no ADR 0015) e,
  depois, o primeiro recorte do dashboard em Rust seguindo a cena aprovada.
- Publicação e verificações remotas vinculadas ao [PR #15](https://github.com/lippdev/memory-pier/pull/15).

## Renomeação — memory-bee

- Recorte: o mantenedor aprovou o visual da colmeia e autorizou a renomeação.
  Troca mecânica de `Memory Pier`/`memory-pier`/`memory_pier`/`MEMORY_PIER` para
  `Memory Bee`/`memory-bee`/`memory_bee`/`MEMORY_BEE` em Cargo.toml (crate e
  binário), Cargo.lock, `src/`, `tests/` (inclusive `CARGO_BIN_EXE_memory-bee`),
  schemas (títulos), `scripts/`, README, ROADMAP, AGENTS.md, specs, pesquisa,
  `docs/roadmap/index.html` e `docs/MANUAL_TESTS.md` (variáveis `mp_` → `mb_`).
  Contratos afetados: staging `.memory-bee-apply-<pid>-<n>`, comando `memory-bee`
  nos passos de retomada, textos gravados no HANDOFF e no prompt de retomada.
- Preservados: ADRs anteriores (o ADR 0009 descreve `.memory-pier-apply-*`, nome
  da época; o ADR 0015 registra a troca), registros históricos deste arquivo e os
  links de PRs em `github.com/lippdev/memory-pier`. Exemplos em `examples/` não
  citam o nome, então hashes e manifestos ficam iguais. Pacotes já exportados
  continuam válidos: `verify` confere hashes e layout, não o texto.
- Validação local macOS: `cargo fmt`, `cargo clippy --locked --all-targets
  -D warnings`, `cargo build --locked` e `cargo test --locked` aprovados após a
  troca; `python scripts/check_bundle.py` aprovado em venv com
  `scripts/requirements-validation.txt`. Sobras do nome antigo conferidas por
  grep: só as intencionais. Links relativos e `git diff --check` conferidos.
- Autorrevisão; sem revisão independente.
- Limitações: o remoto continua `lippdev/memory-pier` (renomear só com
  confirmação; o GitHub redireciona o nome antigo); a pasta local pode manter o
  nome antigo. Os comandos do roteiro manual passam a usar `memory-bee`.
- Próximo passo: primeiro recorte do dashboard em Rust seguindo a cena aprovada.

## Dashboard de terminal (ratatui) — primeiro recorte

- Recorte: subcomando `memory-bee dashboard`, somente leitura. ADR 0016 escolhe
  `ratatui 0.30.2` + `crossterm 0.29`. `src/tui/{mod,theme,app,ui}.rs` (módulo do
  binário, não da biblioteca): paleta escura/clara e `NO_COLOR`, descoberta
  unificada Claude/Codex por `App::new`, detalhe de sessão com seleção de ramo
  por teclado (`1`–`9`), prévia de `prepare-resume` e `receive::verify` sob
  `--bundle`, degradação em terminais estreitos (< 80 colunas: um painel por
  vez) e pequenos (< 30×8: aviso). `--once` renderiza um quadro em texto puro
  (sem terminal real) para acessibilidade, automação e os testes deste
  projeto; sem TTY e sem `--once`, saída 64. Contrato completo em
  [docs/specs/dashboard.md](specs/dashboard.md).
- Aceite parcial: os fluxos de listar, inspecionar e a prévia de retomada são
  concluídos por teclado, com e sem cor, sem cortar informações essenciais (a
  contagem de eventos, de diagnósticos e o símbolo de estado nunca são
  cortados; só o rótulo da sessão trunca com `…`). A dashboard chama só o
  núcleo já usado pela CLI. **Não** cobre "exportar e continuar pelo teclado"
  do critério de aceite da etapa — isso é o próximo recorte.
- Validação local macOS: `cargo fmt --all -- --check`,
  `cargo clippy --locked --all-targets -- -D warnings`, `cargo build --locked`
  e `cargo test --locked` aprovados (147 testes: 129 anteriores inalterados +
  10 unitários novos em `src/tui/app.rs` cobrindo navegação, seleção de ramo,
  retomada e verificação contra fixtures reais, + 8 de integração novos em
  `tests/dashboard.rs` cobrindo uso inválido, raiz inexistente, a guarda de
  TTY e `--once` em quatro tamanhos e dois temas); `python scripts/check_bundle.py`
  aprovado. Rodei manualmente `dashboard --once` contra `testdata/claude-projects`
  e `testdata/codex-sessions` em 140×40, 100×30, 60×20 e 40×12 para revisar o
  layout; dois problemas reais encontrados e corrigidos antes de comitar:
  agente e rótulo colados na lista, e o caminho do projeto sobrepondo e
  apagando a aba "ajuda" no cabeçalho estreito. A animação real num terminal
  interativo (não `--once`) não foi vista rodando pelo agente.
- Autorrevisão; sem revisão independente.
- Limitações: exportar, aplicar e lançar a retomada continuam só na CLI — a
  TUI não tem essas teclas. A cena animada da colmeia (abelha, favo) do
  protótipo aprovado não foi portada; o cabeçalho tem só o sinal estático
  `⬢`. Caminhos de arquivo longos no detalhe ainda podem quebrar no meio de
  uma palavra quando envolvem duas linhas (`Paragraph` não quebra por
  diretório); a informação continua completa, só a quebra é menos elegante.
  Compatibilidade real de terminal (emuladores sem 24 bits, larguras mínimas
  de fato usáveis) não foi ensaiada manualmente.
- Próximo passo: segundo recorte do dashboard — exportar, aplicar e lançar a
  retomada pela TUI, com as mesmas confirmações explícitas da CLI (ADR 0014).


## Dashboard — ações confirmadas (segundo recorte)

- Escopo: `e` exporta contexto Claude/Codex com ramo/exclusões, `a` confere e
  aplica pacotes v2, `p` grava prompt e `l` lança com token. Formulários de caminho,
  prévias roláveis, confirmação digitada e Esc sem escrita. ADR 0017 registra
  snapshots, revalidação, terminal e limites. Sem dependências novas.
- Exportação/aplicação usam bytes revistos; aplicação revalida checkout. Retomada
  refaz preparação/token antes de escrever; agente recebe terminal normal e TUI
  volta com resultado/código de saída. Falha preserva pacote e prompt.
- Validação: 156 testes Rust (9 novos de ações/teclado/layout), build, fmt,
  Clippy, schemas/hashes Python e teste PTY com agente falso. O ensaio PTY verifica
  TTY, modo canônico/echo, saída 7, preservação do prompt e restauração ao sair;
  script versionado e adicionado à CI. Não foi lançado agente real.
- Autorrevisão: guardas de escrita, cancelamento, snapshots, token, devolução do
  terminal e telas estreitas. Corrigida leitura concorrente do manifesto na
  prévia de aplicação: hash deve coincidir com o verificado. Teclas de detalhe
  não alteram mais seleção a partir da aba Retomada. Sem revisão independente.
- Limitações: exportação da TUI somente contexto v1 (Git/código selecionado na
  CLI); nova worktree na CLI; formulários sem expansão de shell; I/O síncrono;
  prévia JSON extensa exige rolagem. Cena animada e ensaios humanos pendentes.
  Itens 43–45 do roteiro manual adicionados, sem marcá-los executados.
- Próximo passo: cena da colmeia/revisão de experiência, conforme tarefa acima.
- Publicação e verificações remotas vinculadas ao [PR #18](https://github.com/lippdev/memory-pier/pull/18).


### Proposta em discussão — trabalhar dentro da TUI (registro histórico)

O mantenedor perguntou sobre manter a interface Memory Bee durante a programação,
usando os harnesses Claude/Codex. Possibilidade técnica pesquisada em 2026-09-25:
[Codex App Server](https://learn.chatgpt.com/docs/app-server) oferece integração
interativa e autenticação; [Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview)
fornece o motor, mas documenta aprovação prévia para terceiros oferecerem login e
limites claude.ai, indicando chave de API como caminho padrão. Perfis isolados e
continuidade entre provedores exigem desenho e validação próprios. Proposta ainda
não adotada como arquitetura nem implementada; não substitui silenciosamente o
escopo da entrega atual nem a próxima tarefa aprovada.


## TUI unificada — primeiro recorte demonstrável

- Retomada: sessão anterior deixou implementação local sem commit em
  `codex/unified-workspace`. A nova sessão preservou e concluiu esse recorte;
  o transcript anterior foi consultado somente para recuperar o plano aprovado.
- ADR 0018 registra a mudança de direção: interface própria permanente,
  assinaturas existentes e lançamento real conjunto. README, roadmap, aviso
  no HTML, AGENTS, contrato e roteiro manual atualizados.
- `workspace --demo`: conversa, eventos incrementais, ferramentas/diff fictícios,
  decisões, interrupção/erro, menus, perfis fictícios, sessões persistidas e
  exportação com prévia/exclusões/confirmação. Sem `--demo`, recusa antes de escrever.
  `--once` só renderiza estado sintético em memória.
- Pacote v1 com fonte `memory-bee-demo`, proveniência de entrada/simulação e
  indicação de snapshot parcial. `verify` aceita com avisos; não é sessão nativa.
- Autorrevisão: corrigidas aprovação já resolvida e eventos de turno antigo no
  protocolo experimental, symlink pendente no estado, contexto omitido da prévia,
  omissão de interrupção no Markdown e controles cortados na entrada estreita.
  Sem revisão independente.
- Limites: demo não edita código nem autentica. Protocolo Codex parcial, separado
  da TUI; nenhum harness real ou modelo chamado. Perfis não provam isolamento.
  Disco síncrono, trava por pasta de estado, recuperação de trava manual após
  crash, rascunho/tema não persistidos, projeto único e colmeia estática.
- Validação local macOS: build, fmt e Clippy aprovados; suíte completa de 176
  testes aprovada, seguida de 17 testes do workspace após adicionar um teste de
  transporte falso (177 testes no total). Pacotes v1/v2 aprovados no validador
  Python; PTYs do dashboard e workspace aprovados, incluindo schema do pacote
  demo e restauração do terminal. Links Markdown, JavaScript do roadmap,
  numeração 1–49/sintaxe shell do roteiro e `git diff --check` conferidos.
  CI remota e integração vinculadas ao [PR #19](https://github.com/lippdev/memory-bee/pull/19);
  não houve validação com agentes reais.
- Ensaios humanos 46–49 pendentes; não substituídos por testes automatizados.
- Próximo passo: prova de integração descrita no topo. O plano completo permanece
  em andamento; integração Claude com assinatura depende de confirmação externa.


## Workspace — referência visual e plano por ação

- Status: concluído o recorte de referência HTML e planejamento. Retomada da
  sessão interrompida por limite de uso, preservando suas alterações locais.
- Protótipo versionado em `docs/prototypes/workspace.html`: Clareira preservada,
  conversa linear, campo amarelo e abelha à direita. Estados de pensamento,
  leitura, edição, execução, permissão, conclusão, negação, interrupção, erro e
  sono. Exportação e troca com animação própria continuam planejadas.
- Plano em `specs/workspace-visual.md`; roadmap e ADR 0018 registram a frente
  visual independente da prova de integração, sem mudar o lançamento conjunto.
- Validação nesta retomada: sintaxe JavaScript via `node --check`; automação em
  nova página Orca confirmou posição à direita e sequência pensar → ler → editar
  → permissão → executar → concluir, negação, interrupção sem sucesso atrasado,
  erro e quadro estável com movimento reduzido. Links relativos e
  `git diff --check` conferidos. Nenhum código Rust alterado.
- Autorrevisão de escopo, callbacks invalidados por turno e rótulos de simulação;
  nenhuma revisão independente. Ensaio humano 50 permanece pendente, assim como
  compatibilidade de terminal, renderizador Rust e integração/autenticação real.
- Próximo passo: renderizador Rust conforme plano visual; esta entrega não inicia
  essa implementação. O artifact externo é temporário; o HTML versionado mantém
  a referência local.
- Publicação e verificações remotas vinculadas ao [PR #20](https://github.com/lippdev/memory-bee/pull/20).

## Esclarecimento — TUI sobre os CLIs oficiais

- O mantenedor esclareceu que o login ocorre no Claude Code e no Codex originais.
  A Memory Bee será uma interface e camada de memória sobre processos locais,
  sem capturar ou gerir credenciais. ADR 0018 e roadmap atualizados.
- Documentação oficial consultada: Anthropic permite login do usuário no binário
  Claude Code intacto mesmo hospedado por outro produto; Codex App Server tem
  interface para cliente próprio e conta gerenciada pelo Codex. Isso não valida
  ainda a TUI real nem libera uso de `-p`/Agent SDK com assinatura como premissa.
- Status: revisão documental concluída; prova dos dois processos pendente.
  Autorrevisão; nenhuma revisão independente ou chamada a modelo nesta revisão.
- Próximo: ensaio controlado com os CLIs oficiais em ambiente explícito, cobrindo
  conversa, eventos, permissões e interrupção antes de integrar à tela.

## Claude Code local na TUI — primeiro recorte nativo

- Escopo: `workspace --claude --project <dir> --state <pasta>` abre o Claude Code
  instalado em PTY, usando autenticação e permissões da CLI original. A Memory
  Bee encaminha entrada/colagem/resize e desenha a saída; `--resume` reutiliza o
  último ID nativo. Estado separado guarda só IDs e códigos de saída, com pasta
  privada e trava. Demo e dashboard anteriores continuam disponíveis.
- Evidências: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets
  -- -D warnings`, `cargo test --locked`, `cargo build --locked`,
  `python3 scripts/check_claude_native_pty.py`, links relativos e
  `git diff --check` passaram localmente. Teste sintético exercitou CLI falsa,
  entrada, retomada, privacidade, recusa sem TTY e restauração. Ensaio controlado
  iniciou e encerrou o Claude instalado com `/exit`, sem enviar prompt/modelo;
  não comprova fluxo de conversa nem permissões. CI macOS/Linux configurado para
  repetir o teste sintético.
- Autorrevisão de escopo, armazenamento e fronteira de autenticação; sem revisão
  independente. Ensaio manual 51 pendente. Não há ainda exportação integrada,
  eventos estruturados, UI de conversa própria, perfis isolados nem Codex real.
- Próximo: executar o roteiro 51 com conversa/permissões reais e vincular a sessão
  Claude ao histórico/exportação revisável, mantendo origem e omissões explícitas.
  Lançamento completo com ambos os agentes permanece pendente.

## Correção de produto — Claude em segundo plano

- O mantenedor testou o recorte PTY e esclareceu que não quer a tela original
  do Claude visível. A interface aceita é totalmente da Memory Bee; o processo
  Claude roda em segundo plano. A moldura PTY permanece identificada como prova
  técnica, não como visual final.
- Pesquisa nas referências oficiais: `claude -p` oferece fluxo `stream-json`,
  eventos parciais, retomada e prompt de permissão via ferramenta MCP. O modo
  programático permitiria uma UI própria, mas a orientação de autenticação da
  Anthropic não certifica o uso da assinatura em produto terceiro desse tipo.
  Nenhuma chamada a modelo foi feita nesta revisão; não há prova de permissões
  reais nem decisão de usar API paga.
- Status: direção e critérios atualizados no ADR 0018, roadmap, contrato e
  README. Autorrevisão documental; sem revisão independente. Próximo: prova
  sintética do adaptador estruturado e da UI Bee, seguida da resolução da
  fronteira de assinatura e ensaio real autorizado.

## Claude interativo oculto — primeira interface Bee funcional

- `workspace --claude` mantém o Claude Code original no PTY sem desenhar sua
  tela. Hooks oficiais carregados por `--settings` entregam texto, ações, status
  e solicitações de permissão à Bee via socket Unix privado. Decisões exigem
  `y`/`n`; falha/timeout negam. `--claude-terminal` preserva o recorte visível
  anterior para diagnóstico. Nenhum token é lido pela Memory Bee.
- Evidências: CLI falsa cobriu tela original invisível, mensagens, negar/permitir,
  IDs, retomada, restauração do terminal e negação quando o socket falta.
  O Claude instalado 2.1.282 foi iniciado em projeto já confiado; hook de sessão
  e resposta curta real apareceram na Bee, sem marca nativa visível, e a saída
  concluiu com código 0. No projeto descartável não confiado, `SessionStart`
  ocorreu mas um prompt nativo oculto impediu a saída normal; limite registrado.
- Autorrevisão; sem revisão independente. Teste manual de ferramentas reais,
  permissão, interrupção e retomada permanece pendente (item 52). O histórico
  completo ainda não é reidratado na tela e a exportação nativa não está ligada.
- Uma tentativa controlada de solicitar ação Bash real não produziu um pedido
  observável na Bee dentro do prazo; nenhum arquivo temporário foi criado. Esse
  ensaio é inconclusivo para o fluxo de permissão real, não um aceite.
- Próximo: cobrir estados nativos invisíveis de confiança/login com falha clara
  ou ação explícita, validar ferramentas/permissões reais e ligar a sessão Claude
  à exportação revisável. Não declarar o fluxo completo pronto nesta etapa.
- Retomada deste recorte em 2026-09-25: `cargo fmt --all -- --check`, `cargo
  test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo
  build --locked` e testes PTY sintéticos da moldura nativa e da UI oculta
  passaram. O teste oculto agora cobre `Ctrl+Q` com permissão pendente, que deve
  negar antes de sair. A suíte Rust exigiu execução fora do sandbox para criar
  um socket Unix; o mesmo valeu para o teste PTY da ponte. Links relativos e
  `git diff --check` conferidos. Autorrevisão, sem revisão independente.
- Corrigida a decisão por teclado: somente `y`, `n` ou Esc resolvem uma
  permissão; `Ctrl+C` e `Ctrl+Q` negam antes de interromper/sair. O roteiro 51
  identifica `--claude-terminal` como diagnóstico. Ainda não há prova de
  ferramentas e permissões reais além da resposta curta já observada.
