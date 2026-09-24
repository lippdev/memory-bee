# Exportação local de contexto

Implementação inicial do [pacote v1](bundle-v1.md), sem exportar código nem chamar
modelos no modo somente contexto. Para incluir código, veja a seção final. A compatibilidade real do leitor ainda não foi certificada.

## Comandos

```sh
cargo run --locked -- export testdata/claude/basic.jsonl --preview
mkdir -p exports
cargo run --locked -- export testdata/claude/basic.jsonl --output exports/first-bundle
```

`export <arquivo>` exige exatamente uma opção de destino: `--preview` ou
`--output <pasta-nova>`. Não cria diretórios pais e não substitui pasta, arquivo
ou symlink existente, mesmo que a pasta esteja vazia.

Opções adicionais:

- `--leaf <uuid>`: seleção explícita de ramo pelo leitor existente. Obrigatória
  para registros ambíguos/subagentes. Sem ela, aceita apenas uma ponta verificável.
- `--exclude-line <n>`: exclui todos os eventos/blocos da linha original selecionada.
  Pode repetir. Número deve ser positivo e existir no ramo escolhido; erros de
  digitação não viram exclusões silenciosas. Excluir tudo é erro.

```sh
cargo run --locked -- export testdata/claude-projects/arbitrary/session.jsonl --leaf a1 --preview
cargo run --locked -- export testdata/claude/basic.jsonl --exclude-line 2 --output exports/only-request
```

## Resultado e proveniência

- `HANDOFF.md`: primeiro pedido humano **retido**, último registro retido,
  seleção, referência Git opcional, omissões e orientação manual de retomada.
  Não infere objetivos, próximos passos ou conclusões semânticas da conversa.
- `history.jsonl`: todos os eventos selecionados, sequência renumerada, fonte por
  linha/bloco e IDs preservados, papéis e ferramentas separados, checkpoints rotulados.
- `manifest.json`: contrato v1, origem observada, SHA-256 dos dois payloads,
  omissões, avisos e `redaction: pending-review`. Sem `--project`, estado Git é null/unknown.

Caminho do arquivo lido e metadados `cwd` não são copiados. Outros IDs, nomes de
ferramentas e textos são preservados: revise-os antes de compartilhar. Referências
parentais podem apontar para linhas excluídas e não implicam conteúdo recuperável.
Versão da origem é null quando ausente ou há múltiplos valores observados, com aviso
para a multiplicidade. Nenhuma versão é declarada compatível por esse campo.

HANDOFF limita cada trecho a 1.200 caracteres da origem, com indicação de corte,
e lista no máximo 20 omissões/avisos de cada categoria; o manifesto contém as
listas completas. Controles de terminal são escapados nos trechos Markdown.
Histórico retido permanece integral no JSONL; não incluir o log bruto.

## Prévia e possíveis segredos

Prévia produz JSON com `manifest`, `handoff`, `history`, `findings` e `partial`,
sem escrever arquivos. É uma prévia completa do conteúdo selecionado, podendo
conter texto sensível: use-a localmente para decidir exclusões. Uma invocação
posterior relê a origem e usa nova data; para revisão reproduzível use cópia estável.
A biblioteca prepara bytes imutáveis e calcula hashes desses mesmos bytes.

Detecção examina texto, entrada/saída de ferramentas e metadados exportados.
Padrões iniciais: alguns tokens `sk-`, GitHub e AWS, cabeçalhos de chave privada,
atribuições comuns de senha/chave/token, Bearer e URLs HTTP(S) com usuário/senha.
`findings` traz código, campo, linha e bloco quando disponíveis, nunca o valor
correspondente. Pode produzir falsos positivos e não detecta todo segredo,
credencial codificada ou dado pessoal. Ausência de achados não significa revisão.

Com achados, `--output` não cria pasta e devolve `written: false` com localizações.
Use a prévia e exclusões ou uma cópia sanitizada da origem. Não há flag para
ignorar detecção nem marca automática de revisão humana. Excluir uma linha não
apaga IDs dela referenciados por outras linhas; essas referências também são
inspecionadas e podem continuar impedindo a gravação.

## Códigos de saída e integridade da gravação

| Código | Resultado |
|---|---|
| 0 | Prévia disponível ou pacote gravado sem perdas detectadas na leitura |
| 2 | Prévia/pacote disponível, com leitura parcial da sessão ou do Git declarada |
| 3 | Possíveis segredos; prévia disponível, gravação bloqueada |
| 1 | Falha de leitura, seleção, conteúdo vazio ou gravação |
| 64 | Argumentos inválidos |

`--output` confirma `written: true` somente após concluir a gravação. Saída 2 não
significa que a pasta deixou de ser criada. O manifesto é escrito por último;
tratar pasta sem manifesto válido e hashes conferidos como incompleta.

No Unix, pasta é criada com modo 0700 e arquivos com 0600 (ou mais restritos pela
umask). Em erro normal, a operação tenta remover somente seus próprios arquivos.
Interrupção abrupta pode deixar uma pasta incompleta, que não será reutilizada na
próxima tentativa. Não é snapshot atômico da origem nem proteção contra processo
malicioso modificando o destino simultaneamente.

O pacote é local; compartilhar, aplicar código ou publicar continuam ações
separadas. Veja o [roteiro manual](../MANUAL_TESTS.md) e o
[ADR 0006](../decisions/0006-context-export-and-deferred-manual-validation.md).

## Referência do código com Git

```sh
cargo run --locked -- export testdata/claude/basic.jsonl --project . --preview
```

`--project <pasta>` consulta explicitamente uma working tree local (aceita subpasta
ou worktree vinculada). Sem essa opção, não consulta Git. O projeto não é inferido
nem sua associação com a sessão é certificada. Git instalado é necessário apenas
para esta opção; a exportação de contexto continua disponível se a consulta falhar.

O manifesto registra `remote` (origin), `branch`, `base_commit` e `dirty`. Com commit
observado, `code_state` é `base-reference`, inclusive quando dirty é true. Isso
não significa que mudanças locais foram incluídas ou que o commit está publicado.
O HANDOFF mostra os valores e avisa sobre alterações que não acompanham o pacote.
Sem --include-path, não exporta nomes de arquivos nem patches. Caminhos absolutos da working tree não são copiados como metadados.

- Detached HEAD: branch null, commit preservado e aviso.
- Sem commit: base null, branch quando disponível, estado unknown e saída 2.
- Projeto ausente, bare, sem Git ou falha: aviso genérico sem caminhos privados;
  campos indisponíveis null e saída 2. O pacote de contexto ainda pode ser gravado.
- Sem origin: remote null com aviso. Apenas origin é consultado, sem rede.
- Origin sensível/local/não suportado: omitido. HTTPS e SSH simples são aceitos;
  usuário SSH convencional git é removido. Query, fragmento e escapes não são aceitos.
- Branch e remoto passam pelo detector de segredos: achados bloqueiam escrita
  com saída 3, com precedência sobre saída parcial.

Dirty inclui staged, unstaged, conflitos e não rastreados; ignorados
não contam. Submódulos não são consultados internamente: geram aviso e saída parcial; dirty fica null se nenhuma outra mudança for observada. Filtros clean/process/smudge são desabilitados no status. É observado antes da gravação do pacote, que pode alterar o status se
seu destino estiver dentro do projeto. Consultas não formam snapshot atômico;
HEAD/branch são relidos e descartados se mudarem. Não há fetch nem verificação de
existência remota. Limite de stdout por comando: 1 MiB; sem timeout nesta entrega.
Veja o [ADR 0007](../decisions/0007-explicit-git-reference.md).

## Incluir arquivos selecionados

Com `--project`, repetir `--include-path <arquivo-relativo-à-raiz-Git>` para incluir
mudanças textuais e novos arquivos. Isso produz manifesto v2, changes.patch e/ou
files/, conforme [contrato de código selecionado](bundle-v2.md). Sem seletores,
permanece v1. Não há aplicação automática. Omissões de código também dão saída 2;
achados de segredos dão saída 3, com `selection` apontando ao índice de selected_paths.
Os itens 10–15 do [roteiro manual](../MANUAL_TESTS.md) demonstram o fluxo sintético.
