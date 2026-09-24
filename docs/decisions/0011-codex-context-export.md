# 0011 — Exportação explícita de contexto Codex

Status: adotado no recorte da etapa 05 em 2026-09-24.

## Decisão

Adicionar `export-codex <rollout.jsonl>` com arquivo explícito, prévia, exclusões
por linha e gravação local. Reutilizar gravação, hashes, detecção de segredos,
referência Git e código selecionado; manter os eventos nativos de cada leitor
em um adaptador interno. Nenhuma conversão de turnos Codex em UUID/parent Claude.

Os manifestos v1/v2 já permitem agente textual e histórico de objetos JSON.
`source.agent: codex` identifica o perfil descrito no contrato Codex; não alterar
os eventos Claude nem exigir nova versão do pacote. Documentar o papel developer
e os campos específicos do Codex no contrato do histórico.

Preservar linha/bloco, sessão/turno conhecidos, IDs, fase, namespace de ferramenta,
timestamps e proveniência. Só renumerar sequence após exclusões. Recusar múltiplas
sessões conhecidas nos eventos retidos; identidade desconhecida continua null.
Diagnósticos de leitura permanecem mesmo após excluir eventos. IDs herdados de
metadados são preservados e inspecionados para segredos; excluir uma mensagem não
remove referências mantidas por outras mensagens.

O HANDOFF e manifesto informam perfil experimental e registro físico, sem afirmar
conversa ativa após rollback/forks. `--leaf` é erro; nenhuma descoberta ou execução.
Não inferir projeto a partir de cwd. `--project` e `--include-path` têm a mesma
semântica explícita e os mesmos limites do exportador Claude.

## Validação e consequências

Fixtures sintéticas devem cobrir preview, exclusões, perdas, metadados sensíveis,
recusa de vazios/múltiplas sessões, origem preservada, colisão de destino e verify.
Validar também schema/hashes independentemente em Python e roundtrip v2 em clone
sintético. Compatibilidade real, revisão humana e retomada permanecem pendentes.
Nenhuma dependência nova ou chamada a modelos.
