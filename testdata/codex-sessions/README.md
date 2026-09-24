# Árvore sintética de descoberta Codex

Dados escritos do zero; nenhuma conversa real, segredo ou certificação de formato.

- `2026/09/24/session.jsonl` e `copy.jsonl`: mesmo session ID em arquivos distintos,
  projeto `/synthetic/project`; devem aparecer separadamente.
- `workers/worker.jsonl`: sessão independente no mesmo projeto. O nome da pasta
  não comprova que é subagente; o resultado não deve inventar esse vínculo.
- `other/session.jsonl`: outro projeto, apesar do mesmo nome de arquivo.

Todos têm três registros físicos/um evento, versão `synthetic`. A busca por
`/synthetic/project` inspeciona quatro arquivos e retorna três caminhos; por
`/synthetic/other`, retorna um. Não há seleção automática, exportação ou execução.
