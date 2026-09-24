# 0013 — Preparação explícita de retomada Claude/Codex

Status: adotado no recorte da etapa 05 em 2026-09-24.

## Contexto

A etapa 05 pede que um pacote verificado seja retomado por outro agente no mesmo
projeto, com instrução manual enquanto o lançamento não estiver validado, e que
continuar no mesmo diretório seja separado de bifurcar em outra worktree.

Interfaces oficiais observadas na ajuda das CLIs instaladas no macOS do mantenedor,
em 2026-09-24: Claude Code 2.1.282 aceita `claude [prompt]`, com `--add-dir
<directories...>` variádico para acesso adicional; Codex CLI 0.156.1 aceita
`codex [PROMPT]` com `-C/--cd <DIR>` para a raiz de trabalho e `--add-dir` para
diretórios **graváveis** adicionais. Ambos têm `resume`/`--continue` para sessões
próprias, que não se aplicam a pacotes de outro agente. Nenhuma CLI foi executada
com prompt nesta entrega.

## Decisão

Adicionar `prepare-resume <pacote> --target (claude | codex) --project <projeto>
[--worktree <pasta-nova>] (--preview | --output <arquivo-novo>)`.

- Verificar o pacote com o mesmo receptor de `verify`; falha interrompe (saída 1).
- Observar o projeto explícito com a inspeção Git existente e comparar HEAD com a
  base registrada. Para pacotes v2, comparar hashes/modos dos arquivos selecionados
  com base e resultado (`base`, `applied`, `mixed`, `unknown`), só lendo arquivos
  regulares, sem seguir symlinks.
- Modo mesmo checkout (sem `--worktree`): sugerir `apply --check`/`--write` só
  quando o checkout está limpo, na base exata e com as mudanças ainda não aplicadas.
- Modo nova worktree (`--worktree`): exigir projeto Git, base registrada e commit
  presente no repositório; recusar caminho existente, dentro do projeto ou do pacote.
  Sugerir `git worktree add --detach <pasta> <base>`, sem criar branch, seguido de
  apply na nova pasta quando houver mudanças.
- Gerar instrução de retomada a partir de dados controlados: caminhos
  canonicalizados, commit validado, contagens, estados e diagnósticos próprios.
  **Nenhum texto do histórico, do HANDOFF ou de campos livres do manifesto entra no
  prompt**; o agente de destino é orientado a ler o pacote como dado histórico.
- Retornar passos como `argv` exato, `cwd` e renderização POSIX com aspas simples.
  Para Claude, o prompt precede `--add-dir` porque a opção é variádica. Para Codex,
  não conceder escrita ao pacote; a leitura fora da raiz depende da política de
  sandbox configurada e é sinalizada.
- `--output` grava apenas o prompt, em arquivo novo 0600 fora do pacote, sem
  sobrescrever. A renderização shell passa a ler o arquivo com `"$(cat …)"`.
- Saída 0 sem pontos de atenção, 2 com pontos de atenção, 1 erro, 64 uso inválido.
  Relatório declara `launched: false` e `source_modified: false`.

## Alternativas descartadas

- Lançar o agente diretamente: sem validação real de flags, autenticação, cota e
  sandbox; um lançamento que falha não pode encerrar nem alterar a origem.
- Criar a worktree automaticamente: é escrita no repositório do usuário; fica como
  passo explícito até haver um fluxo de confirmação (TUI).
- Embutir resumo ou trechos do histórico no prompt: misturaria dados históricos com
  instruções e contrariaria a distinção entre extração e síntese.

## Limitações

Estado observado não é atômico; o checkout pode mudar entre preparação e uso. Um
checkout sujo com mudanças já aplicadas não distingue alterações extras fora da
seleção. Flags das CLIs podem mudar entre versões; compatibilidade real, execução
dos comandos sugeridos e comportamento de sandbox continuam pendentes no roteiro
manual. Sem dependências novas, rede ou chamadas a modelos.
