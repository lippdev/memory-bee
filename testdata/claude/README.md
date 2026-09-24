# Fixtures sintéticas Claude

Escritas manualmente a partir do formato observado em documentação e no parser público do Orca. Não são dumps da versão instalada nem certificam compatibilidade com ela.

- `basic.jsonl`: dois registros de texto, uma string e uma lista de blocos.
- `truncated.jsonl`: mesmo prefixo com uma terceira linha incompleta; o futuro leitor deve reportar a perda.
- `empty.jsonl`: nenhuma mensagem; resultado vazio explícito.

A etapa 02 deve acrescentar ferramentas, tipos desconhecidos, ramos, compaction, UTF-8 inválido e limites de tamanho com testes do leitor real.
