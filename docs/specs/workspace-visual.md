# Plano visual do workspace

Direção aprovada pelo mantenedor em 2026-09-25. Preserva a Clareira da Colmeia
(ADR 0015), seus sprites e cores. Conversa linear inspirada na organização do
Claude Code: mensagens e ferramentas no histórico, campo com contorno amarelo,
abelha à direita do campo e identificação do agente/perfil abaixo. A mascote
não ocupa uma coluna do histórico nem substitui mensagens de estado.

[Protótipo local](../prototypes/workspace.html) ·
[Artifact compartilhado](https://share.onorca.dev/a/pNllOVQr_SZU).
O HTML é uma demonstração com dados sintéticos; não é a implementação Rust.

## Vocabulário de ações

| Ação | Animação e sinal legível | Estado da demonstração |
|---|---|---|
| Pronta | Voo leve junto ao campo | Implementado |
| Pensando | Asas lentas e reticências | Implementado |
| Lendo/buscando | Movimento lateral e linhas de documento | Implementado |
| Editando | Movimento vertical curto, pólen e sinal + | Implementado |
| Executando/testando | Asas rápidas e símbolo de terminal | Implementado |
| Aguardando permissão | Voo contido e interrogação; decisão explícita | Implementado |
| Concluído | Pólen e check temporário | Implementado |
| Negado/interrompido | Pouso com − ou quadrado | Implementado |
| Erro | Pouso com exclamação e histórico preservado | Implementado |
| Inativa | Dorme; digitar acorda | Implementado |
| Exportando | Transporta pólen até uma célula | Planejado |
| Trocando agente/perfil | Travessia curta entre duas células | Planejado |

## Entregas e aceite

1. **Referência visual e plano:** artifact com abelha à direita, estados de
   conversa e contorno amarelo; preservar a cena inicial. Esta entrega.
2. **Renderizador Rust:** sprites em células e máquina de estados independente
   do provedor. Testar precedência, cancelamento e animação desligada.
3. **Conexão à demo TUI:** ligar aos eventos existentes, incluindo exportação e
   troca. Verificar teclado, rolagem, entrada multilinha e terminal estreito.
4. **Eventos reais:** conectar os mesmos estados aos dois adaptadores quando
   validados conforme ADR 0018. Não deduzir sucesso pelo tempo decorrido.

Permissão tem prioridade sobre atividade. Interrupção invalida callbacks do
turno anterior; erro não termina em sucesso automático. Ferramenta desconhecida
usa atividade genérica. Operações simultâneas mostram contagem/resumo textual,
sem alternância rápida da mascote. A animação não executa nem aprova operações.

Movimento reduzido mantém pose estática, símbolo e texto. Sem cor mantém sinais
legíveis. Estado textual muda somente nas transições, sem anúncios a cada frame.
Em largura insuficiente, reduzir a mascote a um símbolo antes de sacrificar o
campo ou suas teclas. Evitar flashes e não apresentar porcentagens inventadas.

Validação humana do terminal e integração real permanecem pendentes. Aprovar o
artifact não certifica compatibilidade de terminal ou autenticação dos agentes.
