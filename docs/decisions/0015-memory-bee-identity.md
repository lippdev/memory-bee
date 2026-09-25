# 0015 — Nome e identidade Memory Bee

Status: aceito pelo mantenedor em 2026-09-24. Substitui o [ADR 0002](0002-product-name.md).

## Decisão

Adotar **Memory Bee**, com slug `memory-bee`, no lugar de Memory Pier. A identidade
visual passa a ser uma abelha e uma colmeia numa clareira ao entardecer:

- A **abelha** é o mascote e representa a conexão entre contextos: ela coleta mel a
  cada contexto somado, a cada ideia acrescida a um repositório.
- A **colmeia** é o mapa dos projetos. Cada gominho é um repositório descoberto; o
  nível de mel dentro dele é o contexto acumulado (sessões e pacotes). A colmeia
  cresce a partir do galho conforme entram repositórios.
- Na dashboard de terminal, a cena é o painel central e a colmeia é o seletor de
  projetos: selecionar acende o gominho e a abelha voa até ele. A lista de sessões
  e o detalhe ficam abaixo.
- Estados de atenção (sessão parcial ou ambígua) fazem o contorno do gominho
  piscar; a abelha para com um `?`.

Referência visual aprovada como base: protótipo
[Clareira da Colmeia](https://claude.ai/artifact/W5a8EAd4UKXLS84Nsqa8Ab), com
paletas de entardecer (escura) e fim de tarde (clara), degradações para 100×30,
60×20 e 40×12, modos 256 cores, `NO_COLOR` e movimento reduzido. A cena é desenhada
só com o que um terminal reproduz: células, meio-blocos `▀▄`, braille 2×4 e cor de
24 bits, para que o porte em Rust/ratatui seja fiel. O protótipo anterior (farol no
píer, com o cachorro Hayate) fica arquivado fora do repositório.

## Contexto e limites

Decisão de produto tomada pelo mantenedor após brainstorm, em substituição ao
nome e à imagem do píer. Como no ADR 0002, não houve verificação de marca, domínio
ou registro de pacotes para o nome novo. A pasta local e o remoto podem conservar o
nome antigo até a renomeação.

## Impacto e sequência

Esta rodada registra a decisão e publica o protótipo. A troca no código vem em PR
próprio, antes do dashboard em Rust, cobrindo os pontos onde o nome atual é parte
de um contrato ou de um texto gravado:

- Nome do crate e do binário `memory-pier` (`Cargo.toml`, `Cargo.lock`), o caminho
  `memory_pier::` em `src/main.rs` e em `tests/*`, e `CARGO_BIN_EXE_memory-pier`
  nos testes.
- Uso da CLI em `src/main.rs` e o comando `memory-pier` gravado nos passos de
  retomada (`src/resume.rs`).
- Pasta de staging `.memory-pier-apply-<pid>-<n>` (`src/receive.rs`, ADR 0009,
  `docs/specs/receive-apply.md`, `docs/MANUAL_TESTS.md`).
- Textos gravados nos pacotes: HANDOFF (`src/bundle.rs`) e prompt de retomada
  (`src/resume.rs`); títulos em `schemas/bundle-v1.schema.json` e
  `schemas/bundle-v2.schema.json`; caminho do binário em `scripts/check_bundle.py`.
- Documentação: README, ROADMAP, AGENTS.md, `docs/EXECUTION.md`, `docs/specs/*`,
  `docs/MANUAL_TESTS.md`, `docs/roadmap/index.html` e a variável de ambiente
  planejada `MEMORY_PIER_REDUCED_MOTION` → `MEMORY_BEE_REDUCED_MOTION`.
- Remoto GitHub `lippdev/memory-pier`: renomear só com confirmação do mantenedor.

Pacotes já exportados carregam "Memory Pier" no HANDOFF e no prompt; continuam
válidos, porque `verify` confere hashes e layout, não esse texto.

## Alternativas descartadas

- Manter o farol no píer com nome novo: a imagem do píer não representa o acúmulo
  de contexto por projeto, que é o que a colmeia mostra de forma direta.
- Um gominho por sessão: cresce rápido demais e perde a leitura por repositório.
- Cena como faixa inferior, como no protótipo anterior: a colmeia deixa de ser o
  seletor e a interação abelha↔colmeia perde o centro.
