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
um terminal nativo nem ativar apenas Codex como substituto silencioso do requisito.

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

Não há evidência de aprovação/integração Claude com assinatura para este produto.
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
