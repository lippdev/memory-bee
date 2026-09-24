# 0007 — Referência Git explícita e local

Status: adotado em 2026-09-24 para o primeiro recorte da etapa 04.

## Decisão

Adicionar `export --project <pasta>` como opção. Sem ela, preservar exportação
somente contexto sem executar Git. Não inferir o projeto pelo cwd histórico nem
por falas do agente: a escolha do usuário não certifica associação com a sessão.

Consultar a CLI Git instalada, sem nova dependência Rust. Isso suporta working
trees e worktrees vinculadas com a resolução nativa do Git. Executar argumentos
fixos sem shell: raiz, HEAD, branch simbólica, status porcelain e origin local.
Não fazer fetch, commit, push ou aplicar mudanças. Desabilitar locks opcionais,
fsmonitor e untracked cache; remover variáveis GIT_* herdadas para que não desviem
a consulta para outro repositório. Não exportar stderr, caminhos de arquivos ou
raiz absoluta. Cada stdout tem limite de 1 MiB; não há timeout nesta entrega.

Usar os campos existentes do manifesto v1. `base-reference` significa que o commit
foi observado; não promete checkout limpo, remoto atualizado ou commit publicado.
`dirty` conta mudanças staged/unstaged, conflitos, submódulos e arquivos não
rastreados conforme status Git. Ignorados não contam. Falhas deixam campos null,
aviso e saída parcial; sem commit, code_state continua unknown. Detached HEAD
válido mantém commit com branch null. HEAD/branch são relidos para detectar
mudanças durante a consulta; isso não produz um snapshot atômico.

Origin é a única referência remota consultada; nunca tentar contato. Aceitar HTTPS
e SSH de formato conservador sem credenciais, query, fragmento ou escapes.
Normalizar usuário convencional git de SSH para URL sem usuário; omitir URLs com
outros usuários, caminhos locais e formatos não suportados com aviso. Isso pode
exigir ajuste manual para clonar. URL/branch passam também pelo detector existente;
seus valores só aparecem na prévia e pacote, não nos achados de bloqueio.

## Limites e próxima entrega

Git precisa estar no PATH apenas quando --project é usado. Caminhos Git que não
possam ser representados em UTF-8 podem ficar indisponíveis. Configuração local do
Git continua relevante ao status; limites/erros não devem virar estado limpo.
Não incluir patches, novos arquivos ou inventário de caminhos neste recorte.
Próxima entrega: seleção e transporte de mudanças, com contrato e verificação de
base antes de aplicação explícita. M1 e ensaios manuais permanecem pendentes.

Atualização posterior: o [ADR 0008](0008-selected-code-and-bundle-v2.md) desabilita
filtros no status e evita recursão em submódulos, cujo estado interno passa a ser
explicitamente não verificado.
