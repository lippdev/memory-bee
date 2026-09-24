# 0008 — Código selecionado e pacote v2

Status: adotado em 2026-09-24. Segundo recorte da etapa 04.

## Escopo e contrato

Exportar arquivos explicitamente escolhidos por `--include-path`, repetível,
relativos à raiz Git de `--project`. Sem seleção, manter manifesto v1. Seleção
exige commit base verificável e produz v2, mesmo se tudo for omitido.

A v1 já publicada proíbe campos adicionais. Criar v2 para registrar `selected_paths`
e `changes`, com destino, payload, hashes anterior/posterior e modos por arquivo,
sem alterar silenciosamente a validação de consumidores v1. Schema e especificação
v1 ficam preservados como contrato; novos consumidores devem escolher por versão.

Arquivos presentes na base entram em `changes.patch`; novos em relação à base
entram em `files/NNNN.txt`, mapeados explicitamente no manifesto. A extensão é do
payload e não muda o nome de destino. Modificação, remoção e modo executável são
representados. Renomeação exige selecionar caminho antigo e novo, como remoção e
adição; não há detecção automática. Novos arquivos staged são adições também.

## Leitura e geração

Comparar blobs brutos do commit com bytes atuais no disco, sem executar filtros,
textconv ou diff externo. Staging não é preservado: cada seleção representa o
estado final do arquivo em disco relativo à base. Não chamar `git diff`, que pode
executar filtros clean mesmo com textconv desabilitado. Desabilitar clean/process/
smudge configurados também no status; isso pode diferir do status habitual de um
projeto com filtros. Desabilitar lazy fetch para manter consulta local.

Gerar hunks de substituição integral para texto UTF-8 sem NUL. É simples e testável
contra Git apply em checkout sintético separado; custa patches maiores e pode
incluir linhas sem mudança. Todos os bytes anteriores/posteriores incluídos passam
pelo detector de segredos, inclusive linhas removidas. Prévia inclui todo conteúdo;
achados indicam índice da seleção, nunca ecoam o valor no bloqueio de gravação.

Limites: 64 caminhos, 1 KiB por nome, 1 MiB por arquivo/versão e 8 MiB de conteúdo
anterior+posterior. Primeira versão aceita nomes ASCII com letras, números, espaço,
ponto, hífen, underscore e barra. Recusar absoluto, traversal, `.git`, pathspecs e
nome inexistente; omitir symlink, diretório, repositório aninhado, submódulo, binário,
conflito, erro de leitura e limite com saída parcial. Arquivo sem diferença também
gera registro de omissão. Somente caminhos escolhidos são enumerados no manifesto.
Arquivo ignorado pode ser incluído se explicitamente escolhido; não há seleção
implícita de diretórios. Gitignore não substitui revisão de compartilhamento.

Reler HEAD e arquivos incluídos antes de concluir a preparação. Falhar se mudarem.
Não há snapshot atômico nem proteção contra troca maliciosa concorrente de caminhos.
Bytes preparados são imutáveis para preview/escrita. Manifesto por último, pasta
nova e permissões privadas se aplicam também ao subdiretório files.

## Próximo recorte

Esta entrega exporta; não implementa importador/aplicador. Próxima tarefa é validar
versão, hashes, caminhos, base e conflitos antes de aplicação explícita, sem
sobrescrita automática. M1 continua em andamento. Atualizar sempre o roteiro manual
nas próximas implementações, conforme pedido atual do mantenedor.
