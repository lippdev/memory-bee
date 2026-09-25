# 0017 — Ações confirmadas no dashboard

Status: adotado no segundo recorte da etapa 06 em 2026-09-25.

## Decisão

Reutilizar `bundle`, `receive` e `resume`, sem subprocessos da própria CLI nem
shell para interpretar entradas. Formulários pedem caminhos literais; uma tela
rolável mostra a prévia antes da confirmação digitada. Esc cancela sem escrever.

- Exportação: contexto v1, ramo mostrado na prévia e exclusões por linha;
  `EXPORTAR` grava exatamente o snapshot preparado, com as travas do núcleo para
  possíveis segredos e destino existente. O pacote passa a ser o pacote ativo.
  Referência Git e seleção de código permanecem na CLI neste recorte.
- Aplicação: verificar pacote e guardar `receive::Plan`; mostrar manifesto e
  relatório antes de `APLICAR`. O plano guarda os bytes verificados e refaz as
  verificações do checkout ao escrever. Alterações posteriores no pacote não
  trocam silenciosamente o conteúdo aprovado. Nenhum commit ou push.
- Prompt: `GRAVAR` salva um arquivo novo; refazer preparação e comparar token
  antes da escrita. Não criar worktrees pela TUI neste recorte.
- Lançamento: digitar o token do ADR 0014, liberar modo bruto/tela alternada,
  refazer preparação, conferir token, gravar prompt novo e iniciar com argv
  direto. Após saída ou falha, restaurar TUI e mostrar resultado. Não encadear
  aplicação e lançamento; cada etapa exige sua própria prévia/confirmação.

O laço continua síncrono. I/O bloqueia a interface durante a operação; Esc
cancela formulários/prévias, não interrompe uma aplicação já confirmada. O
agente iniciado recebe o terminal e seus próprios sinais. `--once` não tem
entrada de teclado e continua sem escrita ou lançamento.

## Justificativa e limites

Snapshots de exportação/aplicação evitam trocar os bytes aprovados entre prévia
e escrita. Lançamento precisa observar novamente o checkout, por isso mantém o
vínculo do token do núcleo. Confirmação por tecla única ou shell intermediário
não oferece essas propriedades. Não adicionar dependências.

Prévia detalhada em JSON prioriza auditabilidade neste recorte; PgUp/PgDn e Home
permitem percorrê-la em telas estreitas. Caminhos são literais: sem expansão de
`~`, variáveis ou glob. A cena animada continua pendente. Testes com agente falso
não certificam compatibilidade com agentes reais.
