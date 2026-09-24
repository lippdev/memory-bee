# 0014 — Lançamento de agente confirmado

Status: adotado no recorte da etapa 05 em 2026-09-24.

## Contexto

O ADR 0013 entrega instrução e passos manuais sem lançar agentes. A etapa 05 pede
adaptadores de lançamento em que uma falha preserve o pacote e não encerre a origem.
O mantenedor autorizou seguir sem os ensaios manuais, que continuam pendentes.

## Decisão

Adicionar `--launch <confirmação>` a `prepare-resume`, exigindo `--output`.

- `--preview` passa a informar `confirmation`: os 16 primeiros caracteres
  hexadecimais do SHA-256 sobre o hash do manifesto, alvo, modo, caminhos, resumo
  verificado, estado observado do projeto, pontos de atenção, `cwd`/`argv` dos
  passos e prompt. O caminho do arquivo de prompt fica de fora.
- Com `--launch`, o comando refaz verificação e observação e recalcula o token.
  Divergência recusa antes de escrever qualquer arquivo. Assim, a confirmação
  vale só para o que foi revisto: outro alvo, outro pacote, mudança no checkout
  ou em pontos de atenção exige nova prévia.
- Lançar só no modo mesmo checkout e só quando o passo final é o único restante.
  Worktree nova ou apply pendente são recusados; o usuário executa esses passos e
  prepara de novo com `--project <checkout>`.
- Gravar o prompt antes de iniciar. Executar `argv` diretamente, sem shell, com
  terminal herdado e resolução pelo PATH. Relatório impresso depois que o agente
  termina, com `launched` e `launch_exit_code`.
- Saída 0 quando o agente termina com 0; 4 quando termina com outro código ou por
  sinal; 1 quando não pode ser iniciado. Nenhum caso remove pacote ou prompt, nem
  toca a sessão de origem.

## Alternativas descartadas

- Confirmação interativa (pergunta no terminal): não é testável em CI, disputa o
  terminal com o agente e não prova que o usuário viu os mesmos pontos de atenção.
- Opção `--yes` sem vínculo: aprovaria um estado diferente do revisto.
- Encadear worktree, apply e lançamento: agruparia escritas diferentes numa única
  aprovação; fica para um fluxo com confirmações por etapa (TUI).

## Limitações

O token detecta mudanças entre prévia e lançamento, mas não é segredo nem
assinatura; qualquer pessoa pode recalculá-lo. Resta uma janela entre a última
observação e o início do agente. Flags das CLIs não foram certificadas com os
agentes reais; os testes usam agentes falsos no PATH. Autenticação, cota, sandbox
e aceitação do prompt pelo agente continuam no roteiro manual.
