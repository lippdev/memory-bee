# 0010 — Inspeção explícita e experimental do Codex

Status: aceito no recorte autorizado da etapa 05, 2026-09-24.

## Contexto

O núcleo já exporta Claude e verifica/aplica pacotes. O próximo passo registrado
é um segundo leitor. A documentação oficial declara a transcrição instável;
o schema de ResponseItem gerado pela CLI não estabelece o envelope do rollout.
Não há ensaio real autorizado/executado como evidência de compatibilidade.

## Decisão

Adicionar `inspect-codex <rollout.jsonl>` com relatório próprio e perfil sintético
versionado no [contrato](../specs/codex-reader.md). Reutilizar apenas limites,
estados, diagnósticos e drenagem limitada de linhas do leitor existente. Não
adaptar artificialmente a árvore UUID/parent do Claude aos turnos do Codex.

Extrair response_item suportado em ordem física. Diagnosticar e omitir event_msg
sempre, sem inferir que cada evento é duplicado e sem duplicar os registros de
mensagem na saída. Reasoning e tipos fora do perfil são perdas explícitas.
Compactação conserva checkpoint disponível, mas não reproduz replacement_history.
O relatório não representa a visão ativa após forks/rollback.

Saída parcial é esperada em transcrições com registros auxiliares, configuração
adicional ou itens não suportados. Isso é preferível a afirmar captura integral.
Compatibilidade permanece unverified, mesmo em fixtures sem perdas.

## Consequências

Exportação Codex, descoberta e lançamento são entregas posteriores. Esta mudança
não altera contrato nem comportamento de exportação/seleção Claude. Ensaios reais
controlados continuam no roteiro manual; os testes sintéticos não os substituem.
O envelope experimental precisará de revisão se a validação real revelar diferenças.
