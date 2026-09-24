# Fixtures sintéticas Claude

Escritas manualmente a partir do formato observado em documentação e no parser público do Orca. Não são dumps da versão instalada nem certificam compatibilidade com ela.

- `basic.jsonl`: dois registros de texto, uma string e uma lista de blocos.
- `truncated.jsonl`: mesmo prefixo com uma terceira linha incompleta; o leitor reporta a perda.
- `empty.jsonl`: nenhuma mensagem; resultado vazio explícito.

- `tools.jsonl`: chamadas/resultados, texto misto e imagem sintética omitida.
- `branches.jsonl`: respostas irmãs e subagente, exigindo seleção de ramo.
- `compaction.jsonl`: resumo, fronteira de compactação e mensagem de checkpoint.
- `unsupported.jsonl`: versão não certificada, registro futuro, thinking e imagem.

`cargo test --locked` também gera UTF-8 inválido, limites, erros intermediários,
metadados inconsistentes e arquivos temporários sintéticos. Os testes verificam
proveniência, preservação da origem, saída JSON e códigos de saída da CLI.
