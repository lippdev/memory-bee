# 0016 — Biblioteca de TUI

Status: aceito pelo mantenedor em 2026-09-25.

## Decisão

Adotar **ratatui 0.30.2** (MIT) com o backend **crossterm 0.29** para o dashboard
de terminal da etapa 06.

- `rust-version` do ratatui 0.30.2 é 1.88; do crossterm 0.29 é 1.63. Ambos cabem
  na toolchain fixada 1.98.1 sem exigir atualização.
- `ratatui::backend::TestBackend` permite testar o layout e o conteúdo do quadro
  sem terminal real, no mesmo estilo dos testes de integração existentes
  (`tests/*.rs` chamando o binário via `CARGO_BIN_EXE_memory-bee`).
- Sem runtime assíncrono: a dashboard é somente leitura nesta etapa, com um laço
  de eventos síncrono (poll de teclado com timeout, sem I/O de rede).
- O ruído determinístico da cena (célula/colmeia/abelha) é implementado localmente
  com hash inteiro, como no protótipo aprovado, sem a crate `rand`.

## Alternativas descartadas

- **cursive**: mais pesado, com dependências de motor de terminal próprias e
  menos alinhado ao padrão atual do ecossistema; descartado no ADR 0004 e
  novamente aqui.
- **crossterm puro, sem framework**: exigiria layout, buffer duplo e diffing de
  quadro escritos à mão, reproduzindo o que o ratatui já oferece testado.
- **iced/egui (GUI)**: contradiz o princípio de terminal primeiro do roadmap;
  fora de escopo.

## Impacto

- Novas dependências: `ratatui = "0.30.2"` e `crossterm = "0.29"` em
  `[dependencies]`. Sem novos dependentes transitivos fora do que essas duas
  crates já trazem (nenhuma dependência de rede, TLS ou async).
- CI (`.github/workflows/ci.yml`) não muda: os mesmos passos (`fmt`, `clippy`,
  `test`, `build`, `check_bundle.py`) cobrem o novo código; `cargo test --locked`
  passa a incluir testes com `TestBackend`.
- `Cargo.lock` ganha as duas crates e suas dependências; atualizado junto com
  este ADR.
- Módulos novos: `src/tui/` (`scene.rs`, `theme.rs`, `app.rs`, `ui.rs`), sem
  alterar os módulos existentes de núcleo (`discovery`, `claude`, `codex`,
  `bundle`, `receive`, `resume`, `git`), que a dashboard só consome.

## Limitações

Compatibilidade de terminal (largura mínima, emuladores sem suporte a cor de
24 bits, braille ausente em algumas fontes) segue coberta pelos modos de
degradação já especificados no protótipo aprovado (`docs/decisions/0015-memory-bee-identity.md`),
não por este ADR. Testes automatizados com `TestBackend` verificam conteúdo e
layout do buffer, não a renderização visual real em um terminal físico — isso
fica no roteiro manual.
