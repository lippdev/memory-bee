# Workspace unificado — demonstração e Claude local experimental

Contrato dos primeiros recortes do [ADR 0018](../decisions/0018-unified-workspace.md).
`workspace --demo` não é programação real: Claude/Codex, ferramentas, mudanças e
permissões são simulados. `workspace --claude` inicia o Claude Code instalado em
segundo plano e desenha conversa/ações na TUI Bee.

Para a versão real, a TUI deverá hospedar os CLIs oficiais como processos locais.
O usuário autentica cada um pelo seu próprio fluxo. A Memory Bee não lê nem
armazena tokens; associa sessões, eventos e exportações ao processo e ao perfil
selecionado. App Server é o transporte candidato para Codex; para Claude, testar
o binário original em PTY e verificar se a CLI permite uma conversa própria com
permissões fiéis. O PTY oculto e hooks oficiais fornecem o primeiro transporte
Claude com eventos para uma conversa Bee, ainda experimental.

## Claude local experimental

```sh
memory-bee workspace --claude --project . --state /tmp/bee-claude-state
memory-bee workspace --claude --project . --state /tmp/bee-claude-state --resume
```

Exige `claude` no PATH, TTY interativo, projeto existente e pasta de estado
privada (pai existente). Inicia o binário original com `--session-id <uuid>` ou
`--resume <uuid>`, sem copiar tokens. A Memory Bee usa `--settings` apenas na
invocação para registrar hooks `SessionStart`, `MessageDisplay`, `PreToolUse`,
`PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `Stop` e `StopFailure`.
O Claude continua controlando login, sessão e execução. Um socket Unix em pasta
temporária 0700 recebe eventos; o subprocesso de hook devolve decisão `allow` ou
`deny` ao Claude. Falha de ponte e timeout negam permissões. A tela nativa fica
oculta; a Bee mostra mensagens, nomes de ações e decisão `y`/`n`, com negação
inicial. Enter envia prompt, `Ctrl+C` interrompe e `Ctrl+Q` encerra. O terminal
volta ao estado anterior na saída. `--claude-terminal` reabre o modo anterior com
a TUI nativa visível para diagnóstico. Se houver permissão pendente, `Ctrl+C` e
`Ctrl+Q` negam antes de interromper ou sair; outras teclas além de `y`, `n` e
Esc não decidem a permissão.
O painel mostra o `tool_input` completo; quando não cabe para revisão, `y` também
nega.

Cada ação aparece com um resumo (`Ação: Bash · <comando>`). O resultado informa a
origem: "aprovada por você na Bee, uma vez" somente quando houve
`PermissionRequest` e o hook recebeu `allow`; ações sem pedido aparecem como
"sem pedido de permissão; liberada pelas regras ou modo do próprio Claude". A
associação usa `tool_use_id` quando existe; o `PermissionRequest` oficial não o
traz, então vale nome e entrada da ferramenta. Se o pedido corresponder a mais de
uma chamada idêntica ainda sem decisão, os resultados dessas chamadas aparecem
com "origem incerta", sem atribuir a aprovação a nenhuma delas. O
painel fecha em 85 s, antes do limite de 90 s do hook, e registra que o pedido
expirou negado; um segundo pedido enquanto outro aguarda é negado e registrado.
Se a resposta não chegar ao hook, a Bee informa que nada foi aprovado.

Telas nativas sem hook (login, confiança do projeto, configuração inicial ou
outro diálogo) são detectadas pela ausência de progresso: sem `SessionStart` em
5 s, a Bee informa que o Claude aguarda uma ação no terminal original e nomeia o
tipo provável (confiança, login, configuração inicial ou tela sem evento) por
palavras-chave da tela privada, sem copiar o texto nativo para a conversa. Com a
sessão pronta, um turno sem hooks por 20 s gera um aviso único (pode estar
pensando ou numa tela nativa). A saída do PTY alimenta um modelo de tela privado
que responde consultas de terminal e só é desenhado quando o usuário pressiona
`Ctrl+O`, ação explícita para concluir a preparação ou diagnosticar; nesse modo
as teclas vão ao Claude, `Ctrl+O` volta à Bee e pedidos de permissão sempre
retornam ao painel Bee. A Memory Bee não registra a tela nativa.

`Ctrl+Q` nunca digita em um diálogo nativo: antes do `SessionStart`, encerra o
processo imediatamente; com turno em andamento ou tela original aberta, envia
`Ctrl+C` e digita `/exit` só depois do `Stop` do turno (sem turno, aguarda o
encerramento); no prompt ocioso da Bee, digita `/exit`.
Em todos os casos, se o CLI não sair em cinco segundos, o processo é encerrado.

`claude-native.json` guarda projeto canônico, IDs e códigos de saída (até 128
sessões, 64 KiB), sem transcrição. A pasta é 0700 e arquivos são 0600 em Unix;
`claude-native.lock` impede segundo escritor. Após crash, verificar que não há
processo ativo antes de remover a trava antiga. `--resume` usa o último ID desta
pasta; não reenvia prompt. A sessão nativa pode não existir quando se sai antes
de enviar uma mensagem. Exportação/revisão de memória, isolamento de perfis e
integração Codex estão pendentes. O modo não comprova
conversa/permissões reais por si só; ver [roteiro manual](../MANUAL_TESTS.md).

**Direção atual:** o mantenedor não quer ver a TUI original do Claude. O modo
padrão `--claude` já oculta essa tela e mostra componentes da Memory Bee. O
terminal embutido anterior requer `--claude-terminal`. A UI Bee ainda é um
recorte funcional sem histórico/exportação nativos integrados. Prompts nativos
sem hook agora são sinalizados e podem ser concluídos com `Ctrl+O`; a detecção é
heurística (tempo e palavras-chave) e o diálogo de confiança real ainda não foi
ensaiado com conta autenticada. Conferir o [roteiro manual](../MANUAL_TESTS.md). O [ADR 0018](../decisions/0018-unified-workspace.md)
registra por que o transporte mantém o binário interativo sem usar `-p`.

```sh
cargo run --locked -- workspace --demo --project . --state /tmp/bee-demo-state
cargo run --locked -- workspace --demo --project . --agent codex --once --width 60 --height 20 --no-color
```

O pai de `--state` precisa existir. Pasta nova é criada 0700, arquivos 0600;
pasta existente precisa ser privada. `--once` usa memória, não aceita `--state`,
não grava nem lê sessões existentes. `--width/--height` (1–500) só com `--once`.
`--agent claude|codex` escolhe apenas a primeira sessão de um estado novo.
`--theme dark|light`, `--no-color` e `NO_COLOR` controlam aparência. Na demo,
flags repetidas, projeto ausente e uso interativo sem TTY retornam 64; falha de
I/O, 1. Modo sem `--demo` nem `--claude` segue recusado.
Projeto é normalizado para caminho absoluto e precisa ser diretório existente.

## Experiência e teclas

```text
┌─ Memory Bee · SIMULAÇÃO ──────────────────┐
│ Claude · Pessoal (demo) · sessão #1        │
│ /projeto                                 │
│ [1] Você: adicione busca                  │
│ [2] Claude: [SIMULAÇÃO] Vou mostrar...     │
│ [5] Alteração simulada: src/example.rs    │
│ [6] Permissão: cargo test (simulado)      │
│ Tab abre opções; nenhuma ação automática │
│ > mensagem                               │
│ Enter envia · F2 menu · Ctrl+Q sai        │
└──────────────────────────────────────────┘
```

- Enter entra na conversa/envia; Ctrl+J acrescenta linha. Colagem não envia.
  Editor simples: acrescentar texto e Backspace, sem movimentação de cursor.
- F2 ou `/` com campo vazio abre comandos. Menus ocupam a tela; eventos continuam
  chegando sem descartar o rascunho. Esc volta. Comando desconhecido mostra ajuda.
  Para executar um comando por texto pode-se colar `/help` e pressionar Enter.
- Tab ou `/permissions` abre decisão, com **Negar** inicialmente selecionado;
  setas + Enter escolhem. Ctrl+C interrompe e invalida aprovação pendente.
- Ctrl+Q ou `/quit` sai; turno ativo exige escolher interromper e sair.
- PgUp/PgDn rolam conversa/prévia; Home vai ao início, End acompanha conversa.
- `/agent`, `/accounts` e `/sessions` mudam destino só com turno parado.
  Agente/perfil cria conversa vazia ou pede revisão completa e `CONTINUAR`.
  Transferência copia apenas textos de usuário/resposta como referências de
  contexto; ferramentas e decisões permanecem no histórico de origem. Não é
  retomada nativa nem síntese. Não há exclusões na transferência neste recorte.
- `/export` pede pasta nova e números de registros a excluir. Prévia inclui
  Markdown, histórico, manifesto e achados. `EXPORTAR` grava o snapshot revisto;
  Esc cancela. Suspeita de segredo bloqueia escrita; detecção não é completa.
- `/diff` mostra última alteração fictícia, `/projects` volta à colmeia estática
  do projeto explícito, `/settings` muda tema/cor, `/help` explica teclas.
  `[erro]` numa mensagem provoca falha sintética com histórico preservado.

Em 30×10 ou maior, menus/entrada/rodapé continuam acessíveis; abaixo disso, aviso.
Cabeçalho e status podem truncar; projeto completo fica em `/projects`. Não há
animação, então a demo não exige opção de movimento reduzido. Não promete paridade
visual com o protótipo animado nem editor completo de texto.

## Persistência, falhas e exportação

`workspace.json` versionado guarda projeto, perfis fictícios, sessão selecionada
e eventos. Limites: 16 MiB, 128 sessões, 32 perfis, 100 mil eventos por sessão e
64 KiB por entrada. Falha de persistência congela envio/eventos; conteúdo já em
memória continua exportável. A sessão carregada como ativa é marcada interrompida,
sem reexecutar/reencaminhar mensagens. Tema e rascunho não são persistidos.

`workspace.lock` recusa segundo escritor. Após kill/crash, confirmar que não existe
outro processo usando a pasta antes de remover manualmente a trava. `workspace.new`
remanescente também exige inspeção manual; não apagar automaticamente. JSON inválido,
projeto diferente, versão desconhecida e symlink de estado são recusados sem
substituir o arquivo original. Não é armazenamento cifrado nem trava de checkout.

Pacotes v1 usam fonte `memory-bee-demo`, proveniência explícita e hashes normais.
`verify` aceita com avisos de origem desconhecida; não certifica sessão nativa.
Snapshot de turno ativo, interrompido ou com erro é parcial. Não inclui código,
credenciais, metadados de autenticação nem referência Git implícita. Texto digitado
pelo usuário continua sendo dado real local, mesmo numa simulação: não versionar.

## Validação e pendências

`cargo test --locked` cobre estado, teclado, cancelamento, permissões, retomada,
proveniência, exclusões e telas estreitas. `scripts/check_workspace_pty.py` percorre
colagem, decisão negada, exportação/schema/verify, resize, interrupção e restauração
em PTY com projeto sintético. O [roteiro manual](../MANUAL_TESTS.md) mantém ensaios
humanos separados. `scripts/check_claude_native_pty.py` usa CLI falsa para testar
entrada, ID, retomada, estado privado e restauração do terminal. Adaptador Codex
experimental é apenas protocolo não conectado. `scripts/check_claude_hidden_pty.py`
cobre a UI Bee sobre CLI falsa com hooks, texto nativo oculto, decisões de
permissão, retomada e negação na falha da ponte. Uma mensagem curta real chegou
à Bee; ferramentas/permissões reais, contas, isolamento, exportação nativa e
lançamento conjunto seguem pendentes.
