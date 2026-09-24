# Descoberta Codex por projeto — perfil experimental 1

```sh
cargo run --locked -- sessions-codex --root testdata/codex-sessions --project /synthetic/project
```

`--root` e `--project` são obrigatórios, em qualquer ordem. A raiz é um diretório
explicitamente escolhido de arquivos Codex ou cópias locais, não uma configuração
global de conta. Nenhuma consulta automática a home, CODEX_HOME, índices ou perfis.
A descoberta não exporta, aplica código nem inicia agentes. Decisão no
[ADR 0012](../decisions/0012-codex-session-discovery.md).

## Travessia e associação

Buscar arquivos `.jsonl` na raiz e em até três níveis de subdiretórios:

- `<root>/*.jsonl`
- `<root>/<a>/*.jsonl`
- `<root>/<a>/<b>/*.jsonl`
- `<root>/<a>/<b>/<c>/*.jsonl`

Nomes a/b/c são arbitrários; uma árvore ano/mês/dia é um exemplo, não requisito
nem certificação de layout do fornecedor. Diretórios encontrados além desses
níveis são omitidos com `depth_limit`, mesmo se estiverem vazios. Extensão é
exatamente `.jsonl`; outros arquivos são ignorados, mas contam no limite de entradas.
Usar uma raiz mais específica para procurar em uma árvore mais profunda.

O [leitor Codex](codex-reader.md) inspeciona cada candidato com seus limites.
Associar por conjunto de `cwd` absolutos observados em `session_meta` e
`turn_context`, normalizados lexicalmente: exige um único projeto igual ao pedido.
Ausência, caminho relativo ou múltiplos projetos diferentes gera exclusão com
código. A presença de metadados válidos não certifica linhas com metadados omitidos,
ilegíveis ou desconhecidos; os diagnósticos e estado parcial continuam visíveis.
Não procurar nomes de projeto no texto da conversa ou no caminho do arquivo.

Também exigir um único session ID conhecido dos cabeçalhos Codex. Arquivos sem ID
conhecido ou com IDs diferentes não aparecem como sessões selecionáveis; recebem
diagnóstico. Repetição de cabeçalho com mesmo ID pode aparecer como arquivo parcial.
Um arquivo só com metadados válidos pode aparecer com `event_count: 0`; não significa
que contenha eventos exportáveis. Não identificar arquivos Claude como Codex por
conter cwd ou extensão JSONL: o leitor precisa reconhecer o envelope Codex.

Projetos fornecidos em caminho relativo são resolvidos contra o diretório atual.
Metadados históricos relativos não são aceitos. Projetos não precisam existir;
comparação lexical não resolve symlinks, aliases, diferenças de caixa nem caminhos
de outro sistema operacional. Worktrees e subpastas são projetos distintos.

## Relatório e seleção

JSON contém `agent: codex`, `discovery_version: 1`, `root`, `project`,
`compatibility: unverified`, `partial`, `files_inspected`, `sessions` e `diagnostics`.
Cada sessão contém:

- `path`: caminho absoluto sob raiz canonicalizada, usado para seleção explícita.
- `session_id`: único ID conhecido, sem garantia de unicidade entre arquivos.
- `observed_versions`: versões observadas, sem certificar suporte.
- `state`, `record_count`, `event_count`: estado e contagens do leitor físico.
- `diagnostics`: códigos do leitor com linha/bloco, incluindo compatibilidade.

Mesmo session ID em dois arquivos produz dois resultados. Não fundir cópias,
arquivos arquivados, subagentes ou forks. O nome da pasta não prova subagente;
a saída não inventa parentesco, pontas de ramos ou conversa ativa após rollback.
Selecionar o caminho retornado, usando os comandos existentes:

```sh
cargo run --locked -- inspect-codex testdata/codex-sessions/2026/09/24/session.jsonl
cargo run --locked -- export-codex testdata/codex-sessions/2026/09/24/session.jsonl --preview
```

Não há `--leaf` ou escolha automática por ID. Exportação continua exigindo revisão,
prévia/destino explícito e [contrato próprio](codex-export.md). O arquivo é relido;
modificações posteriores à descoberta podem mudar conteúdo e disponibilidade.

A listagem não inclui mensagens, argumentos/resultados de ferramentas ou payloads
omitidos. Caminhos, IDs e versões são metadados reais, sujeitos a revisão antes de
compartilhar; esta operação não é sanitização nem autorização de publicação.

## Orçamentos, erros e saídas

Padrões compartilhados com descoberta Claude: 10.000 entradas visitadas, 1.000
arquivos inspecionados e 256 MiB acumulados, além de 64 MiB por arquivo e 1 MiB por
linha do leitor. A biblioteca permite reduzir os orçamentos com `DiscoveryLimits`;
zero é erro. A CLI usa esses padrões. Arquivo grande é omitido sem impedir outros;
limites de entradas/arquivos/bytes encerram a busca com resultado parcial. Cada
arquivo tem bytes reservados antes da abertura; crescimento além da reserva é erro.

Raiz explícita pode ser symlink e é canonicalizada. Symlinks encontrados abaixo
dela não são seguidos, inclusive links para diretórios. Arquivos especiais .jsonl
são omitidos antes de abrir. Raiz/projeto não UTF-8 é erro; entrada não UTF-8 é
omitida com código no diretório pai. Erros de leitura abaixo da raiz não escondem
outros arquivos legíveis. Raiz inacessível/inválida é fatal.

Diagnósticos de descoberta: `directory_unreadable`, `entry_unreadable`,
`file_unreadable`, `missing_project_metadata`, `invalid_project_metadata`,
`conflicting_project_metadata`, `missing_session_metadata`,
`conflicting_session_metadata`, `symlink_skipped`, `non_regular_file`,
`non_utf8_path`, `depth_limit`, `entry_limit`, `file_count_limit`, `file_size_limit`,
`total_byte_limit` e `partial_file`. Havendo várias falhas de classificação, o
primeiro motivo suficiente pode excluir o arquivo; não é uma lista exaustiva delas.

Leitura parcial de qualquer arquivo inspecionado é informada, mesmo se pertencer
a outro projeto. Detalhes do leitor só aparecem nas sessões incluídas. Diagnósticos
de descoberta e sessões são ordenados por caminho ao final; sob limites, o
subconjunto visitado depende da ordem de enumeração do sistema de arquivos.

Saídas: 0 para listagem sem problemas detectados (inclusive vazia/outros projetos),
2 para listagem parcial, 1 para erro fatal e 64 para argumentos inválidos.
Compatibility unverified sozinha não força saída 2. Não há snapshot atômico da
árvore, índice persistente, resolução de aliases ou proteção contra troca concorrente
maliciosa de caminhos. Usar árvore estável para reprodução. Compatibilidade real e
ensaios humanos continuam pendentes nos itens 31–34 do [roteiro manual](../MANUAL_TESTS.md).
