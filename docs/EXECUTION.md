# Estado de execução

Última atualização: 2026-09-24. Este arquivo é o ponto de retomada entre agentes. Não substitui a inspeção do Git nem os critérios do [roadmap](../ROADMAP.md).

## Situação atual

- Existe documentação de produto, workflow e um HTML de planejamento.
- CLI de inspeção, descoberta, seleção e exportação somente contexto implementadas; referência Git opcional implementada; código selecionado v2 implementado; verify/apply explícito implementados; inspeção Codex experimental por arquivo explícito implementada; TUI e contas pendentes.
- Nome escolhido: Memory Pier (`memory-pier`). Núcleo em Rust, primeiro alvo macOS arm64 e leitor Claude Code escolhidos. Licença pendente.
- Cargo, toolchain Rust 1.98.1 e checks canônicos definidos no README; CI em macOS/Linux.
- Remoto: [lippdev/memory-pier](https://github.com/lippdev/memory-pier), público, após autorização do mantenedor.
- Documentação inicial integrada na `main` pelo PR #1.

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

**Etapa 05 — Exportação Codex revisável.** Definir adaptação explícita do relatório
Codex ao núcleo de pacotes, preservando o perfil experimental, proveniência e
omissões. Testar preview/exclusões/segredos e compatibilidade com verify, sem
reinterpretar turnos como árvore Claude. Não lançar agentes nem descobrir arquivos
implicitamente. Descoberta e lançamento continuam como entregas posteriores.
Manter `docs/MANUAL_TESTS.md` atualizado; validação real e retomada humana de M1
continuam pendentes sem bloquear implementação autorizada com testes/CI.

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
| 05 Troca de agente | P1 | Em andamento | Inspeção Codex explícita experimental; exportação, descoberta e lançamento pendentes. |
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
