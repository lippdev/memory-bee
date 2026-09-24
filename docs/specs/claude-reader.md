# Inspeção Claude Code — primeira implementação

Interface local experimental, separada do [pacote portátil v1](bundle-v1.md).
Inclui descoberta explícita e seleção de ramo; não é exportação ou retomada de sessão.

```sh
cargo run --locked -- inspect testdata/claude/basic.jsonl
```

`inspect` recebe um arquivo e escreve JSON em stdout, com `inspection_version: 1`.
Erros de uso/I/O ficam em stderr. Saídas: `0` para leitura ou vazio, `2` para leitura
parcial com JSON aproveitável, `1` para falha de I/O/limite de arquivo ou seleção recusada, `64` para uso
inválido. `--help` descreve a interface. Caminhos com espaços e caminhos não UTF-8
são aceitos pelo sistema operacional; não são incluídos como caminho do arquivo no relatório de inspeção. Metadados
`project_paths` podem conter caminhos absolutos observados na origem.

## Semântica

- `state`: `read` significa que os conteúdos reconhecidos foram lidos sem perda
  detectada; `empty` significa ausência de eventos em entrada vazia/em branco;
  `partial` indica registros/conteúdos omitidos ou metadados inválidos detectados.
  Nenhum desses estados certifica compatibilidade ou continuidade da conversa.
- `compatibility` é sempre `unverified`: fixtures são sintéticas e não há versão
  real certificada. `observed_versions` apenas registra os valores `version` presentes.
- Eventos seguem ordem física, sem ordenação por timestamp nem deduplicação.
  Linha e bloco são contados a partir de 1; bloco null representa conteúdo string.
  UUID, parent UUID, sessão, agente e sidechain são preservados quando disponíveis.
  Timestamp RFC3339 é preservado; ausente ou inválido fica null, com aviso se inválido.
- Texto humano/assistente, chamadas de ferramentas e resultados têm tipos distintos.
  `tool_id`, `tool_name` e `tool_is_error` preservam associação e estado disponível.
  Entrada da ferramenta é JSON textual; saída suporta string ou blocos de texto.
  Nada do histórico é executado. Texto extraído não passa por detecção de segredos.
- `summary`, `isCompactSummary` e `compact_boundary` geram diagnóstico de
  compactação. Resumo disponível vira `checkpoint`, nunca transcrição extraída.
- Thinking, imagens, blocos desconhecidos e registros auxiliares não reconhecidos
  são omitidos com diagnósticos localizados. O relatório não replica seus payloads.
  Não preserva todo o JSON bruto nem todos os metadados do fornecedor.
- Ramos, múltiplas sessões, subagentes, UUIDs duplicados/ausentes e referências de
  parentesco incompletas acionam `requires_branch_selection`. A inspeção mantém os
  eventos e seus vínculos. A escolha ocorre somente com `--leaf`, descrito abaixo.

## Limites e leitura concorrente

A CLI usa no máximo **64 MiB de arquivo** e **1 MiB por linha**, contando CR/LF.
Arquivo acima do limite é recusado; linha grande é drenada sem armazenar todo seu
conteúdo, omitida com `line_limit`, e a leitura continua na próxima linha.
A biblioteca permite fornecer `Limits`; limites zero são recusados.

A leitura se limita ao tamanho observado após abrir o arquivo, sem acompanhar
acréscimos. Isso não é snapshot atômico: escrita simultânea no trecho existente
pode alterar o resultado. Diferença de tamanho, mtime ou bytes efetivamente lidos
produz `source_changed`; mudanças que restaurem metadados podem não ser detectadas.
Para inspeção reproduzível, use uma cópia estável do arquivo.

UTF-8 inválido e JSON inválido omitem só a linha afetada. Linha final com JSON
incompleto gera `incomplete_final_line`; JSON válido sem newline é aceito.
Linhas vazias são ignoradas. Falha de I/O é fatal, sem relatório parcial publicado.
A origem é aberta somente para leitura. Não há dependência de rede ou modelos no
leitor; baixar toolchain/dependências para compilar é uma etapa separada.

## Diagnósticos

`diagnostics` traz código estável nesta versão, linha e bloco quando aplicáveis:

| Código | Significado |
|---|---|
| `unverified_compatibility` | Formato real ainda não certificado |
| `invalid_json`, `invalid_utf8`, `incomplete_final_line` | Linha ilegível/incompleta |
| `line_limit`, `source_changed` | Limite excedido ou origem alterada durante leitura |
| `unknown_record`, `omitted_block`, `omitted_tool_content` | Conteúdo não suportado omitido |
| `invalid_content`, `invalid_block`, `invalid_tool_content` | Estrutura reconhecida, conteúdo inválido |
| `invalid_timestamp`, `invalid_metadata`, `invalid_project_metadata`, `role_mismatch` | Metadados inconsistentes |
| `compaction` | Histórico pode ter sido substituído por checkpoint |
| `missing_uuid`, `duplicate_uuid`, `missing_parent`, `nonpreceding_parent` | Proveniência incompleta/ambígua |
| `branch_selection_required` | Não interpretar eventos como conversa linear selecionada |

## Implementação e validação

Rust 1.98.1 fixado, Cargo.lock versionado, sem framework CLI, runtime async ou banco.
`serde`/`serde_json` atendem leitura e serialização tipada; `chrono` com features
mínimas valida RFC3339 sem implementar um parser de datas próprio. Licença do
produto pendente; publicação no crates.io desabilitada.

Fixtures cobrem texto, ferramentas, ramos, checkpoints e tipos desconhecidos.
Testes geram arquivos temporários sintéticos para UTF-8 inválido, limites, erros e
metadados inconsistentes. Nenhuma conversa real é necessária.

A documentação oficial de [sessões](https://code.claude.com/docs/en/sessions) e
[hooks](https://code.claude.com/docs/en/hooks) foi conferida nesta entrega; descreve
armazenamento e transcrições, mas não certifica o schema deste parser. A referência
estrutural continua registrada na [pesquisa](../research/integrations.md).

## Descoberta por projeto

```sh
cargo run --locked -- sessions --root testdata/claude-projects --project /synthetic/project
```

`--root` aponta para o diretório **projects**, não para a home nem para um único
projeto. Ambos os argumentos são obrigatórios, em qualquer ordem. Nada consulta
`~/.claude` automaticamente. Formatos percorridos:

- `<root>/<project>/*.jsonl`
- `<root>/<project>/<session>/subagents/*.jsonl`

O nome de `<project>` não determina associação. `cwd` absoluto dos registros
`user`/`assistant` é normalizado lexicalmente e deve apontar para um único projeto.
Arquivo com `cwd` ausente, relativo ou conflitante é excluído com diagnóstico.
Uma identificação disponível vale para o arquivo; não é necessário que cada linha
repita `cwd`. Projetos sem diretório existente podem ser pesquisados. Symlinks e
aliases de projeto não são unificados; subdiretórios e worktrees são projetos distintos.

A saída tem `discovery_version: 1`, `root`, `project`, `compatibility`, `partial`,
`files_inspected`, `sessions` e `diagnostics`. Cada sessão traz caminho absoluto,
IDs observados, indicação de subagente, estado do leitor, quantidade de eventos,
pontas e diagnósticos, sem texto da conversa. Mesmo ID em arquivos diferentes
não é deduplicado. `is_subagent` indica localização em `subagents` ou marcador de
agente em algum registro; não funde sessões. Seleção do arquivo é feita usando
seu caminho em `inspect`, sem identificador global presumidamente único.

A raiz explícita é canonicalizada; caminhos da saída seguem essa raiz. Caminhos
não UTF-8 não podem ser serializados na descoberta: raiz/projeto inválido é erro;
entrada encontrada é omitida com `non_utf8_path`. `inspect` ainda aceita arquivo
por caminho não UTF-8. Arquivos/diretórios symlink encontrados são omitidos com
`symlink_skipped`; arquivos especiais `.jsonl` com `non_regular_file`.

Limites padrão: **10.000 entradas**, **1.000 arquivos inspecionados**, **256 MiB**
acumulados, além de 64 MiB por arquivo/1 MiB por linha. Todas as entradas visitadas
contam, inclusive as não JSONL. Arquivo grande é omitido; outros limites encerram
a busca com resultado parcial. Ordenação final por caminho é determinística sem
limite atingido; ao atingir limites, o subconjunto depende da ordem de enumeração
do sistema de arquivos. Não há garantia de snapshot atômico da árvore.

Códigos da descoberta: `directory_unreadable`, `entry_unreadable`, `file_unreadable`,
`missing_project_metadata`, `invalid_project_metadata`, `conflicting_project_metadata`,
`symlink_skipped`, `non_regular_file`, `non_utf8_path`, `entry_limit`,
`file_count_limit`, `file_size_limit`, `total_byte_limit`, `partial_file`.
Leitura parcial de qualquer arquivo visitado é sinalizada, mesmo quando os
metadados disponíveis apontam para outro projeto. Diagnósticos do leitor
ficam nas sessões incluídas. Raiz inacessível/inválida é fatal (saída 1); problemas
abaixo dela ou arquivo lido parcialmente geram JSON com `partial: true` e saída 2.
Raiz vazia ou apenas projetos diferentes retorna lista vazia e saída 0.

## Seleção explícita de ramo

```sh
cargo run --locked -- inspect testdata/claude-projects/arbitrary/session.jsonl --leaf a1
```

A inspeção inclui `records` (proveniência de mensagens reconhecidas mesmo sem
texto), `branch_tips` (UUIDs candidatos, ordenados), `project_paths` e `selection`
(null até escolher). Cada record informa `parent_known` e `valid_metadata`.
Pontas são candidatos estruturais; aparecer na lista não garante seleção válida.

`--leaf` exige ponta existente, UUIDs únicos na cadeia, parent explícito (null na
raiz), sessão não vazia e identidade de agente consistente. Falta de ancestral,
parent futuro, ciclo, UUID duplicado ou mudança de sessão/subagente recusa a
seleção com saída 1, sem JSON de sucesso. Não atravessa compactação ou registros
auxiliares não reconhecidos para fabricar ligações. Subagente independente pode
ser selecionado se sua cadeia for verificável dentro de seu próprio arquivo.

Seleção mantém todos os blocos dos registros ancestrais em ordem física, renumera
`sequence` e preserva linha/bloco da origem. `selection` registra `leaf_uuid`,
`excluded_records` (mensagens reconhecidas) e `excluded_events` (eventos normalizados).
Exclusões geram `branch_excluded`; registros desconhecidos continuam representados
por diagnósticos, não por essa contagem. `requires_branch_selection` passa a false.
Estado, diagnósticos de perdas, metadados de projeto e pontas continuam descrevendo
o arquivo original; um arquivo parcial não se torna completo pela seleção.
Resumos sem ligação ao ramo são excluídos como eventos e a compactação continua
avisada. Compatibilidade permanece `unverified`.

Veja a [decisão de descoberta e seleção](../decisions/0005-session-discovery-and-selection.md).
