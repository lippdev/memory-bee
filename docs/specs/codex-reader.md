# Inspeção Codex — perfil experimental 1

Comando: `memory-pier inspect-codex <rollout.jsonl>`. Somente arquivo explícito,
sem descoberta, execução de ferramentas ou retomada de agentes.
Exportação separada via [export-codex](codex-export.md).

## Evidência e limites do formato

Pesquisa em 2026-09-24: a [documentação de hooks](https://learn.chatgpt.com/docs/hooks)
expõe `transcript_path`, mas declara o formato instável. A documentação do
[App Server](https://learn.chatgpt.com/docs/app-server) descreve itens persistidos,
compactação e rollback; seu protocolo não é o contrato do arquivo JSONL.
O schema da CLI 0.156.1 gerado localmente por `codex app-server generate-json-schema --out <dir>`
foi consultado para `ResponseItem`, blocos textuais e resultados de ferramentas.
Não foram lidas conversas reais nem acionados modelos.

O envelope `timestamp/type/payload` e os registros abaixo definem o perfil
experimental testado por fixtures sintéticas, não uma garantia sobre todas as
versões do Codex. `compatibility: unverified` e diagnóstico correspondente sempre
aparecem. A versão de CLI observada é metadado, não certificação de suporte.

## Extração

- `session_meta`: id, cwd e cli_version. Mudança/repetição do cabeçalho é parcial;
  não mistura silenciosamente identidades. Metadados adicionais não são executados.
- `turn_context`: id de turno e cwd. Contexto anterior não é inferido antes desse registro.
- `response_item/message`: papéis user/assistant/system/developer; blocos input_text
  e output_text, em ordem física. IDs e fase preservados quando presentes.
- `function_call` e `custom_tool_call`: argumentos/input literais, nome, namespace
  e call_id. Resultados correspondentes preservam strings ou blocos input_text/output_text.
  Não se infere sucesso da ferramenta nem se executa conteúdo.
- `compacted`: message como checkpoint; replacement_history não é reintroduzido
  como conversa e gera aviso. Compaction/context_compaction/compaction_trigger
  são marcadores de perda; conteúdo criptografado não é extraído.
- `event_msg`: registro auxiliar diagnosticado e omitido, mesmo quando parece
  duplicata de uma mensagem. Não há fallback nem deduplicação por igualdade de texto.
  Um arquivo contendo apenas esses registros é parcial, sem mensagens inventadas.
- Reasoning, multimídia, ferramentas especializadas e tipos não suportados geram
  avisos de omissão. Campos extras de itens suportados são sinalizados como omitidos.
  Configuração adicional dos metadados também é sinalizada quando presente.

O relatório próprio contém `agent`, versão de inspeção, estado, tamanho observado,
linhas, sessões/versões/projetos observados, registros físicos, eventos e diagnósticos.
Cada evento conserva linha/bloco (1-based), tipo do envelope/item, id opcional,
sessão/turno conhecidos naquele ponto, timestamp válido e proveniência extracted
ou checkpoint. Não usa UUID/parent do Claude nem afirma reconstruir a visão ativa
após rollback, forks ou compactação. É uma inspeção do registro físico.

## Leitura e saída

Limites padrão: 64 MiB por arquivo, 1 MiB por linha (incluindo newline). Leitura
limitada ao tamanho observado ao abrir; mudança detectável gera source_changed.
Linhas grandes são drenadas com memória limitada. JSON/UTF-8 inválidos e último
registro incompleto não impedem recuperar registros posteriores válidos.
Após uma linha ilegível, sessão/turno ficam desconhecidos até novos metadados.
Snapshot não atômico; alterações de mesmo tamanho/mtime podem escapar à detecção.

Saídas: 0 para leitura sem perdas detectadas (ou vazio), 2 para parcial, 1 para
I/O/limite de arquivo, 64 para uso inválido. Compatibilidade não verificada sozinha
não muda o código de saída. Saída contém texto original e pode conter segredos;
esta inspeção não é redação nem autorização de compartilhamento.
