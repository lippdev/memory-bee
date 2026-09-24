# 0012 — Descoberta Codex por projeto sob raiz explícita

Status: adotado no recorte da etapa 05 em 2026-09-24.

## Decisão

Adicionar `sessions-codex --root <pasta> --project <projeto>`, exigindo ambos os
argumentos. A raiz é escolhida pelo usuário; não consultar home, CODEX_HOME,
perfis, índices ou arquivos de configuração para encontrar sessões automaticamente.

Percorrer arquivos .jsonl na raiz e em até três níveis de subpastas. O limite
acomoda árvores de cópias locais e organização em ano/mês/dia sem certificar um
layout do fornecedor. Nomes de arquivo/pasta não identificam agente, projeto ou
sessão. Diretórios além da profundidade permitida geram depth_limit; nenhum
symlink interno é seguido. A raiz explícita pode ser symlink e é canonicalizada.

Reutilizar o leitor Codex experimental e os orçamentos da descoberta Claude:
10.000 entradas, 1.000 arquivos, 256 MiB acumulados, 64 MiB por arquivo e 1 MiB
por linha. Compartilhar travessia, contagem de recursos e normalização lexical;
manter layouts e classificação de cada agente separados. Relatório de descoberta
recebe campo aditivo agent; o restante do contrato Claude permanece igual.

Classificar apenas por cwd absoluto observado em session_meta/turn_context, com
normalização lexical. Exigir um projeto observado e um único session ID conhecido;
metadados ausentes, relativos ou conflitantes geram diagnóstico e exclusão do
resultado. Estado parcial e diagnósticos do leitor continuam visíveis. O comando
não certifica que todos os eventos do arquivo pertencem ao projeto/sessão conhecida,
nem interpreta conteúdo da conversa para completar metadados ausentes.

Cada arquivo é independente, inclusive arquivos com mesmo session ID. Não fundir
subagentes, forks, arquivos ativos/arquivados nem reconstruir conversa ativa.
Selecionar pelo caminho absoluto retornado em inspect-codex/export-codex. Nenhum
lançamento, exportação, escrita na origem ou consulta Git acontece na descoberta.

## Limitações e aceite

O relatório inclui caminhos, IDs e versões, contagens e diagnósticos, sem texto de
mensagens, argumentos/resultados de ferramentas ou payloads omitidos. Isso não é
sanitização dos metadados; uso local e revisão antes de compartilhar.

Comparação lexical não resolve aliases, symlinks ou equivalência entre plataformas;
projetos históricos podem não existir. Subdiretórios e worktrees são distintos.
A busca lê também outros projetos na raiz para classificá-los; erros nesses arquivos
não são escondidos. Leitura/travessia não atômicas; não há proteção contra troca
concorrente maliciosa de caminhos. Subconjunto sob limites depende da enumeração
nativa; resultados encontrados são ordenados ao final.

Aceite: fixtures sintéticas para profundidade, projetos homônimos, sessões
separadas/conflitantes, metadados inválidos, truncamento, symlinks, erros e todos os
orçamentos; seleção do caminho para inspect/export/verify; origem preservada.
Compatibilidade real e ensaios manuais continuam pendentes, sem impedir testes/CI.
Nenhuma dependência nova ou chamada a modelos.
