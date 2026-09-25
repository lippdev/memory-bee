# 0018 — TUI unificada e lançamento conjunto

Status: direção aprovada pelo mantenedor em 2026-09-25; primeiro recorte implementado.

## Decisão e mudança de plano

O mantenedor pediu uma interface permanente da Memory Bee: conversa, ferramentas,
permissões, menus, perfis e exportação na mesma TUI, com Claude/Codex como motores.
Escolheu usar assinaturas existentes e lançar a experiência real somente quando
**os dois agentes** atenderem ao fluxo. Isso substitui a proposta ainda em discussão
no registro anterior e prioriza esta entrega antes da cena animada. Continuamos
uma CLI com TUI, sem exigir IDE, editor, servidor remoto ou API paga separada.

Manter Rust/ratatui e preservar o dashboard existente. Entregar primeiro
`workspace --demo`, sempre identificado como simulação, com adaptador determinístico
para cada agente. O comando sem `--demo` recusa antes de criar estado. Não integrar
um terminal nativo como única experiência nem ativar apenas Codex como substituto
silencioso do requisito.

## Separação e armazenamento

`workspace::{Session, Event}` define identidade e registros normalizados;
`adapter::Adapter` separa envio, eventos, decisão e interrupção da tela.
`store` mantém snapshots privados em pasta explícita, um escritor por trava
`create_new`, limites e recuperação de turno interrompido sem reenvio. Perfis
neste recorte são nomes fictícios, não contas autenticadas. Mudanças de agente ou
perfil criam outra sessão e preservam a origem. Não usar estado nativo de agentes.

Exportação reutiliza o núcleo v1 e informa `source.agent = memory-bee-demo`,
`user_input` para entradas e `simulated` para eventos da demonstração. É um
pacote de contexto desconhecido pelo receptor, verificável com avisos; nunca
se apresenta como conversa nativa Claude/Codex. Nenhuma referência/cópia de código
é inferida. Não alterar schemas nem interpretar dados sintéticos como autenticação.

## Integração real e evidências

O módulo experimental `workspace::codex` é um início de transporte/protocolo,
não um adaptador de produto concluído. Não tem entrada na CLI/TUI. Implementa
JSONL/stdio, handshake, correlação, requests de thread/turno, tradução parcial de
eventos e respostas de aprovação; desconhecidos não recebem permissão. Eventos
de autenticação não são copiados para a conversa. Perfil explícito usa ambiente
reduzido; isso não certifica isolamento de credenciais, keychain ou configuração
de projeto. Persistência nativa, login utilizável e integração à tela ficam pendentes.

Fontes consultadas em 2026-09-25:

- [Codex App Server](https://learn.chatgpt.com/docs/app-server): transporte,
  conversas, eventos, aprovações por thread/turno e login gerenciado pelo Codex.
  Ajuda local de `codex-cli 0.156.1` confirma `app-server --listen stdio://`.
- [Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview): motor
  programável; oferta de login/limites claude.ai por terceiros depende de aprovação.
- [Autenticação Claude](https://code.claude.com/docs/en/legal-and-compliance):
  diferencia hospedar o binário original com login pelo usuário de oferecer login
  em aplicação própria. Essa distinção não comprova a viabilidade da nossa TUI.
  Apenas versão local `Claude Code 2.1.282` conferida, sem login ou inferência.

Não há evidência de integração Claude com assinatura para este produto.
Nenhuma credencial foi copiada e nenhuma chamada a modelo foi feita. Não escolher
API com cobrança separada como alternativa, pois contradiz a escolha do mantenedor.

## Entregas seguintes e limites

1. Prova controlada de cada harness/assinatura: conversa, edição, execução,
   permitir/negar, interrupção, retomada, falha e dois perfis isolados.
2. Conectar os dois adaptadores à TUI responsiva; autenticação oficial separada
   do histórico, descoberta/retomada nativa e exportação após falha/limite.
3. Validar troca entre agente/perfil com contexto revisado e origem preservada.
4. Lançamento conjunto somente com essas evidências. A demo não satisfaz esse aceite.

Snapshot é substituído por rename após sync do arquivo, mas não promete durabilidade
contra perda de energia (sem fsync de diretório), proteção de ancestrais contra
escritores hostis ou recuperação automática de trava após encerramento abrupto.
I/O de persistência/exportação é síncrono; a demo não certifica responsividade
sob disco lento. Uma sessão ativa por pasta de estado, não trava global de checkout
para um futuro motor real. Colmeia estática, projeto explícito e tema não persistido.
Autorrevisão e testes sintéticos não substituem ensaio humano ou prova real.

## Refinamento visual — 2026-09-25

O mantenedor aprovou preservar a Clareira da Colmeia e aproximar a organização da
conversa do Claude Code, com identidade amarela própria e abelha à direita do
campo. A mascote representa cada ação por animação, símbolo e texto; não substitui
permissões ou evidência de execução. O [plano visual](../specs/workspace-visual.md)
separa protótipo, renderizador Rust e integração real. Essa frente pode avançar
com eventos simulados sem certificar nem desbloquear lançamento real.

## Esclarecimento da arquitetura de login — 2026-09-25

O mantenedor esclareceu que o usuário faz login nos CLIs oficiais antes de usar
a Memory Bee. A TUI inicia e acompanha os processos locais de Claude Code e
Codex; não oferece login próprio, lê arquivos de token nem intermedeia credenciais.
O controle de autenticação e renovação permanece com cada fornecedor. O objetivo
é renderizar conversa, ações e memória na TUI da Memory Bee sobre esses processos.

A [documentação da Anthropic](https://code.claude.com/docs/en/legal-and-compliance)
permite ao usuário entrar no binário Claude Code original com a própria assinatura,
inclusive quando hospedado por outro produto, sob as condições ali descritas.
Isso corrige a leitura anterior de que a aprovação para login de terceiros seria
necessária para simplesmente hospedar o binário intacto. A mesma documentação
restringe oferecer login claude.ai próprio ou encaminhar requisições com tokens
da assinatura; essa distinção continua relevante para `-p`/Agent SDK. A
[CLI Claude](https://code.claude.com/docs/en/cli-reference) documenta sessão
interativa, retomada e saída estruturada no modo print, mas não comprova aqui
aprovações interativas e paridade total em uma TUI própria. Testar os limites
antes de escolher o transporte do adaptador Claude.

O [Codex App Server](https://learn.chatgpt.com/docs/app-server) é a interface
documentada para clientes próprios, com eventos, aprovações e estado de conta.
Iniciar esse processo a partir da instalação local deve permitir usar o login
que o Codex gerencia, sujeito à prova no ambiente do usuário. Não criar fluxo
de tokens externos na Memory Bee. Um terminal incorporado pode ser um transporte
ou um escape para interações nativas, mas não substitui sozinho o aceite da
conversa unificada e da exportação. Nenhum login ou modelo foi acionado nesta
revisão documental.

## Recorte Claude primeiro — 2026-09-25

O mantenedor mudou a ordem de implementação: tornar Claude nativo na TUI antes
de integrar Codex. A experiência completa ainda depende dos dois agentes, mas
o modo Claude pode ser entregue e ensaiado separadamente, identificado como
experimental. Isso substitui a restrição anterior de não disponibilizar um
provedor isoladamente durante o desenvolvimento.

Escolhemos iniciar o binário Claude Code original em PTY, com o login já feito
na CLI oficial. `--session-id` cria uma referência controlada pela Memory Bee;
`--resume` retoma esse ID no mesmo projeto. A TUI encaminha entrada e desenha
a saída; a pasta privada guarda apenas IDs/códigos, sem token ou transcrição.
O transporte preserva a interface de permissões do próprio Claude e não exige
API paga separada. É um passo concreto para testar assinatura e conversa nativa,
mas não comprova uma UI Bee própria para eventos, exportação ou perfis. Essas
lacunas, o ensaio humano e a integração Codex permanecem no aceite completo.

## Interface Claude invisível — correção do mantenedor em 2026-09-25

Após testar o recorte PTY, o mantenedor esclareceu que **a tela original do
Claude Code não deve aparecer**. A Memory Bee deve desenhar conversa, ações,
permissões e memória em sua própria TUI; Claude executa em segundo plano. O
`workspace --claude` atual é uma prova experimental do processo, login e sessão
nativa, não a interface de produto desejada. Sua moldura com terminal embutido
deve ser substituída antes de chamar essa experiência de concluída.

A [CLI oficial](https://code.claude.com/docs/en/cli-reference) oferece modo
`-p` com entrada/saída `stream-json`, eventos parciais, retomada e um MCP tool
para prompts de permissão. Isso torna tecnicamente possível desenhar uma TUI
própria sem mostrar a tela do Claude, desde que o adaptador preserve decisões
explícitas, interrupção, erros e proveniência. Não interpretar ANSI ou raspar
pixels do PTY para inferir permissões: a interface visual não é um protocolo
confiável. O primeiro recorte de implementação deve testar o protocolo com CLI
falsa, mantendo a ação de ferramenta negada por padrão até uma decisão do usuário.

A [orientação de autenticação da Anthropic](https://code.claude.com/docs/en/legal-and-compliance)
distingue hospedar o binário intacto com login do usuário de encaminhar requisições
por credenciais de assinatura em aplicação de terceiros; para produtos
programáticos recomenda API key ou provedor compatível. A documentação atual
não resolve por si só se o modo `-p` sob a assinatura do usuário atende ao modelo
de produto pretendido. Não prometer essa combinação como suportada nem coletar
tokens. Confirmar a fronteira antes de lançar a TUI própria com assinatura; se
não for permitida, apresentar a limitação ao mantenedor sem substituir
silenciosamente a assinatura por cobrança de API.

## Resolução de transporte — Claude interativo oculto com hooks

O primeiro recorte da interface Bee foi implementado mantendo **o binário
interativo original**, autenticado pelo próprio usuário. O Claude recebe um PTY
oculto para manter a sessão e o fluxo nativo; a Bee não desenha os bytes dessa
tela. Hooks de sessão, texto, ferramentas e permissões entregam eventos por
socket Unix privado à TUI. `PermissionRequest` aguarda decisão `y`/`n`; erro,
timeout ou segunda solicitação simultânea recebem `deny`. `Ctrl+C` é enviado ao
processo original. O modo anterior com terminal visível fica em
`--claude-terminal` apenas para diagnóstico.

Esta escolha evita `-p`/Agent SDK e mantém a condição de hospedar o CLI original
com login feito nele. A [referência de hooks](https://code.claude.com/docs/en/hooks)
documenta `MessageDisplay` e `PermissionRequest` também na sessão interativa.
Um ensaio real de mensagem curta confirmou texto na Bee sem exibir a tela nativa;
aprovação/negação e retomada passaram com CLI falsa. Não inferir dessa prova que
todos os prompts nativos, ferramentas, contas e exportação estejam cobertos.
Prompts de login ou confiança que não emitam hook podem ficar invisíveis; o
usuário deve completar essa preparação no Claude original. Até existir tratamento
fiel desses casos e ensaio real de permissões, o modo continua experimental.
