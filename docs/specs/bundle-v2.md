# Pacote portátil v2 — alterações selecionadas

Status: exportação implementada; validação de recebimento e aplicação pelo Memory
Pier pendentes. [Schema v2](../../schemas/bundle-v2.schema.json). O
[contrato v1](bundle-v1.md) continua sendo produzido quando não há seleção de código.

## Manifesto e arquivos

Os campos v1 são mantidos com `format_version: 2`. Acrescenta:

- `selected_paths`: seletores literais relativos à raiz Git, ordenados e sem
  duplicatas. Índices nos avisos/achados começam em 1 nessa lista.
- `changes`: um item por mudança efetivamente incluída, com `path` (destino na raiz
  do projeto), `kind` (`add`, `modify`, `delete`), `payload`, `base_sha256`,
  `result_sha256`, `base_mode` e `result_mode`.

SHA-256 cobre bytes brutos, sem conversão de fim de linha. Modos são strings
`100644` ou `100755`. Adição tem base hash/mode null; remoção tem result hash/mode
null. Modificação tem ambos. `project.base_commit` é obrigatório e não null.
`code_state` é `changes-included` se houver mudanças, senão `base-reference`.
Isso não declara que todo estado local está incluído: conferir seleção e omissões.

`changes.patch` contém modificações/remoções de arquivos existentes no commit
base, com cabeçalhos Git, caminhos a/ e b/ e hunks de substituição integral.
Preserva falta de newline final e modo executável. Patch pode conter texto que não
mudou e segredos removidos; revisar integralmente antes de compartilhar.

`files/NNNN.txt` contém bytes de arquivo inexistente na base. O manifesto mapeia
o payload ao caminho real e modo de destino; a extensão .txt não é extensão do
arquivo original. Arquivos vazios são payloads válidos. Permissões do pacote são
privadas e não equivalem ao modo de destino declarado.

`files` lista HANDOFF, histórico e cada payload real com hash SHA-256. Nenhum hash
do próprio manifesto. Ausência de `changes.patch` ou files/ é normal quando não há
mudanças daquela categoria. Arquivos selecionados sem diferença e não suportados
ficam nas omissões, sem entrada fictícia em changes. Retorno 2 indica omissões;
retorno 3 de possíveis segredos tem precedência e bloqueia escrita.

## Semântica da seleção

```sh
cargo run --locked -- export testdata/claude/basic.jsonl --project /pasta/do/projeto --include-path src/main.rs --include-path novo.txt --preview
```

Seleção compara base com disco, incluindo staged e unstaged no resultado final.
Não exporta versão exclusiva do índice. Arquivo novo staged ainda é adição;
arquivo removido no índice mas presente no disco pode não apresentar diferença.
Renomeação requer caminho antigo e novo. Seleção de subpasta não é recursiva.
Mesmo com --project apontando a uma subpasta, seletores são relativos à raiz Git.

Aceita arquivos regulares UTF-8 sem NUL, nomes ASCII com letras, números, espaço,
ponto, hífen, underscore e barra; máximo 1 KiB por nome. Recusa caminhos absolutos,
componentes vazios, `.`, `..`, `.git` (qualquer caixa), pathspecs e nomes inexistentes.
Limites: 64 seletores, 1 MiB por versão de arquivo, 8 MiB somando antes/depois.
Symlinks (inclusive em pais), diretórios, repositórios aninhados/submódulos, conflitos,
binários, UTF-8 inválido, erros e limites são omissões explícitas. Arquivos ignorados
só entram quando escolhidos explicitamente, passando pelo mesmo detector de segredos.

Sem filtros Git nem normalização de bytes. Status desabilita filtros configurados;
projetos com clean/smudge podem ter resultado diferente do Git habitual. Submódulos não são consultados internamente e geram aviso/saída parcial (dirty null quando nenhuma outra mudança é observada). Snapshot
não atômico: rechecagem detecta mudanças comuns de HEAD/conteúdo durante captura,
mas não garante proteção contra modificação concorrente maliciosa. Sem timeout.

## Recebimento (contrato; comando ainda pendente)

O receptor deve recusar versões desconhecidas, payloads ausentes/duplicados,
caminhos inseguros/symlinks/colisões e hashes divergentes. Conferir que changes
referencia payloads listados e caminhos únicos presentes em selected_paths; o
schema não substitui essas verificações relacionais e de filesystem.

Antes de aplicar, confirmar commit base e hashes/modos anteriores em checkout
separado, conferir conflitos e garantir que destinos de adição não existem.
Não executar filtros, hooks ou comandos históricos. Verificar resultado pelos
hashes e modos declarados. Nenhuma aplicação é efeito de abrir/exportar pacote.
O Memory Pier ainda não automatiza essas verificações ou aplicação.

Decisões e limites no [ADR 0008](../decisions/0008-selected-code-and-bundle-v2.md).
