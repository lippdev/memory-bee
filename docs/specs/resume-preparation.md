# Preparação de retomada

Implementado e experimental. Prepara a continuação de um pacote verificado em
Claude Code ou Codex e, só com confirmação explícita, inicia o agente. Decisões no
[ADR 0013](../decisions/0013-explicit-resume-preparation.md) e no
[ADR 0014](../decisions/0014-confirmed-agent-launch.md).

## Comando

```sh
memory-bee prepare-resume <pacote> --target claude --project <checkout> --preview
memory-bee prepare-resume <pacote> --target codex --project <repo> --worktree <pasta-nova> --output <prompt.md>
```

Exige exatamente um de `--preview` ou `--output`. `--launch <confirmação>` só é
aceito com `--output`. `--target` aceita `claude` ou
`codex`. Opções repetidas são uso inválido (saída 64).

## Relatório JSON

| Campo | Significado |
|---|---|
| `mode` | `same-checkout` ou `new-worktree`. |
| `verified` | Resumo de `verify`; integridade não é autenticação. |
| `source_agent` | `claude-code`, `codex` ou `null` quando não reconhecido. |
| `project` | Raiz Git, HEAD, sujeira e parcialidade observados. |
| `base_match` | `match`, `differs`, `unknown` ou `not-recorded`. |
| `changes` | `none`, `base`, `applied`, `mixed` ou `unknown`. |
| `attention` | Diagnósticos `código: texto`; tornam a saída 2. |
| `steps` | Passos manuais com `description`, `cwd`, `argv` e `shell`. |
| `prompt` / `prompt_file` | Instrução gerada e, com `--output`, o arquivo gravado. |
| `confirmation` | Token de 16 caracteres que vincula o lançamento a esta prévia. |
| `launched` / `launch_exit_code` | `true` e código de saída só após iniciar o agente. |
| `source_modified` | Sempre `false`. |

Códigos de atenção: `source_unrecognized`, `project_not_git`, `project_partial`,
`base_not_recorded`, `base_differs`, `project_dirty`, `changes_pending`,
`changes_mixed`, `changes_unknown`, `codex_bundle_read`.

## Lançamento confirmado

```sh
memory-bee prepare-resume <pacote> --target claude --project <checkout> --preview
# revise attention, steps e prompt; copie confirmation
memory-bee prepare-resume <pacote> --target claude --project <checkout> --output <prompt.md> --launch <confirmation>
```

O comando refaz verificação e observação e recalcula o token; divergência recusa
com saída 1 antes de gravar. Exige modo mesmo checkout e nenhum passo anterior
pendente. Grava o prompt, executa o `argv` do passo final sem shell, com o terminal
herdado, e imprime o relatório quando o agente termina. Saídas: 0 agente terminou
com 0, 4 terminou com outro código, 1 não iniciou. Pacote, prompt e origem são
preservados em todos os casos.

## Garantias

- Sem `--launch`, não lança agentes, não cria worktrees, não aplica código, não altera o pacote,
  o projeto nem a sessão de origem. Só `--output` escreve, e apenas o arquivo novo.
- O prompt não contém texto do histórico, do HANDOFF nem campos livres do manifesto.
- Passos de apply só aparecem quando `apply --check` tem pré-condições observadas:
  checkout limpo na base e arquivos selecionados na base (mesmo checkout), ou nova
  worktree criada no commit base.
- Caminhos são canonicalizados e recusados com caracteres de controle ou não UTF-8.

## Limitações

Observação não atômica. Leitura do pacote pelo Codex fora de `-C` depende de sua
política de sandbox. `--add-dir` do Claude dá acesso de ferramenta ao pacote; o
prompt pede que ele não seja alterado, mas isso não é imposto. Flags verificadas
só na ajuda das versões registradas no ADR 0013; o token não é assinatura e há
uma janela entre a observação e o início do agente. Lançamento real pendente no
[roteiro manual](../MANUAL_TESTS.md).
