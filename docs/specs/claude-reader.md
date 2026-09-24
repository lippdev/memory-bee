# Inspeção Claude Code — primeira implementação

Interface local experimental, separada do [pacote portátil v1](bundle-v1.md).
Não é exportação, descoberta automática ou retomada de sessão.

```sh
cargo run --locked -- inspect testdata/claude/basic.jsonl
```

A CLI recebe exatamente um arquivo e escreve JSON em stdout, com `inspection_version: 1`.
Erros de uso/I/O ficam em stderr. Saídas: `0` para leitura ou vazio, `2` para leitura
parcial com JSON aproveitável, `1` para falha de I/O/limite de arquivo, `64` para uso
inválido. `--help` descreve a interface. Caminhos com espaços e caminhos não UTF-8
são aceitos pelo sistema operacional; não são incluídos no relatório.

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
  eventos e seus vínculos, mas não escolhe ramo nem afirma continuidade. Seleção
  de ramo ainda será implementada antes da exportação.

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
| `invalid_timestamp`, `invalid_metadata`, `role_mismatch` | Metadados inconsistentes |
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
