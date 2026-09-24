# Pacote portátil v1 — contrato inicial

Status: contrato v1 com [exportador somente contexto implementado](context-export.md); referência Git opcional implementada, código selecionado implementado no [formato v2](bundle-v2.md); [recebimento/aplicação explícita implementados](receive-apply.md) para o perfil suportado. Mudanças incompatíveis após publicação exigem nova versão. Fixtures e exemplo são sintéticos.

## Arquivos

- `HANDOFF.md`: entrada legível sem Memory Pier. Pedido original, seleção feita, estado do código, omissões e orientação para conferir a base antes de continuar.
- `manifest.json`: versão, origem, integridade dos arquivos, estado do código e limitações.
- `history.jsonl`: eventos selecionados normalizados, sem incluir o log bruto por padrão.
- `changes.patch`: opcional, alterações textuais selecionadas em arquivos rastreados, relativas ao commit base.
- `files/`: opcional, novos arquivos selecionados. Mapeamento para destino deve ser explícito no manifesto.

O exemplo desta etapa é somente contexto. O transporte inicial é uma pasta; empacotamento em arquivo comprimido poderá ser adicionado sem alterar os significados. Não exige conta, rede ou instalação no destinatário.

## Manifesto

Ver [schema JSON](../../schemas/bundle-v1.schema.json). `format_version` é exatamente `1`. Versões desconhecidas são recusadas. Origem registra agente, versão quando conhecida e ID da sessão. IDs e metadados também passam por revisão; caminho absoluto do usuário não precisa sair da máquina.

`project.base_commit` é hash Git completo ou null. `remote` é URL revisada, sem usuário/senha/token, ou null. `dirty` pode ser null quando não verificado. `code_state` distingue referência de base, mudanças incluídas e estado desconhecido. Nunca inferir commit ou estado a partir de uma afirmação do agente. `base-reference` indica commit observado, mesmo com `dirty: true`; não implica alterações incluídas nem commit publicado. Detached HEAD mantém branch null. Falhas de observação são avisadas, com campos indisponíveis null.

`files` lista caminho relativo, SHA-256 e finalidade de cada payload. O manifesto não calcula hash de si. Todos os payloads devem estar listados, e cada hash deve conferir. Hash detecta alteração, não autentica remetente. `omissions` e `warnings` são obrigatórios mesmo vazios. `redaction` indica revisão pendente ou feita, não garantia de ausência de segredos.

## Eventos normalizados

Cada linha é objeto JSON: `sequence` (inteiro crescente a partir de 1), `role` (`user`, `assistant`, `tool` ou `system`), `kind` (`text`, `tool_call`, `tool_result` ou `checkpoint`), `text`, `timestamp` (RFC3339 ou null), `source` (linha original, ID e parent ID opcionais) e `provenance` (`extracted` ou `checkpoint`). Campo ausente na origem fica null; não fabricar datas ou decisões.

Texto de ferramenta é dado histórico, nunca comando executável automaticamente. Síntese por modelo fica fora do v1 básico. Checkpoints existentes devem ser rotulados, não apresentados como transcrição integral. Registrar perdas de blocos e registros no manifesto e no Markdown.

## Recebimento e limites

- Recusar caminhos absolutos, `..`, links simbólicos e colisões de destino antes de ler/aplicar um pacote. A checagem de caminhos é adicional ao schema.
- Nenhuma importação executa hooks, shell, instruções embutidas ou solicita login por causa do conteúdo recebido.
- Abrir contexto não aplica código. Aplicação exige escolha explícita e verificação do commit base, alterações locais e conflitos. Arquivos novos não substituem arquivos existentes silenciosamente.
- Para exportação básica, entrada curta aponta ao histórico selecionado completo. Limite do prompt não apaga registros do pacote.
- Arquivo novo binário, mudança binária ou formato não suportado deve aparecer como omissão; não declarar entrega completa se faltou código.
- As finalidades patch/new-file são reservas neste v1. O exportador com seleção de código usa v2, com mapeamento explícito e semântica definida; verify aceita v1 de contexto/referência; apply exige v2 com mudanças.

## Exemplo e demonstração

[Pacote sintético](../../examples/bundle-v1/HANDOFF.md). [Cenário de aceite M1](../research/m1-scenario.md). Credenciais e transcrições reais não pertencem aos exemplos.
