# Fixtures sintéticas Codex

Escritas para o [perfil experimental](../../docs/specs/codex-reader.md); não são
conversas reais, dumps de uma versão instalada nem certificação de compatibilidade.
Formas de ResponseItem comparadas com schema gerado localmente pela CLI; o envelope
JSONL é parte do perfil experimental e não uma API oficial estável.

- basic.jsonl: sessão, turno, mensagens, chamada e resultado de função; cinco eventos.
- losses.jsonl: evento auxiliar espelhado, reasoning, checkpoint, replacement_history,
  imagem e rollback; três eventos, com perdas explícitas.
- truncated.jsonl: prefixo básico e último registro incompleto.
- empty.jsonl: arquivo vazio.

Testes Rust geram casos adicionais: ferramentas customizadas, papéis, campos
futuros, corrupção, UTF-8, metadados inválidos e limites, sem executar histórico.
