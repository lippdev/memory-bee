# Roteiro de testes manuais

Status: **pendente com o mantenedor**, por escolha de testar depois, um por um.
Os testes automatizados/CI não substituem essa verificação. Comece com fixtures
sintéticas; não adicione transcrições pessoais ou pacotes reais ao Git.

Execute na raiz do checkout. Cada bloco é independente, exceto onde indicado.
O binário Rust é suficiente para usar o produto; Python só participa dos checks
de schema do desenvolvimento.

## 1. Inspecionar texto e proveniência

```sh
cargo run --locked -- inspect testdata/claude/basic.jsonl
```

Esperado: dois eventos, linhas 1 e 2, papéis user/assistant, `compatibility:
unverified`, saída 0. Teste `truncated.jsonl` em seguida: dois eventos preservados,
`state: partial`, diagnóstico de terceira linha incompleta e saída 2.

## 2. Descobrir por projeto

```sh
cargo run --locked -- sessions --root testdata/claude-projects --project /synthetic/project
```

Esperado: sessão principal e subagente em arquivos separados. Arquivo de outro
projeto não aparece. O diretório `/synthetic/project` não precisa existir.

## 3. Selecionar ramo

```sh
cargo run --locked -- inspect testdata/claude-projects/arbitrary/session.jsonl --leaf a1
```

Esperado: registros `u1` e `a1`, um evento excluído, referências originais mantidas.
Trocar por `--leaf inexistente` deve dar saída 1, sem relatório de sucesso.

## 4. Conferir prévia de exportação

```sh
cargo run --locked -- export testdata/claude/basic.jsonl --preview
```

Esperado: JSON com Markdown, histórico, manifesto v1 e achados vazios; nenhum
arquivo criado. Estado Git desconhecido e revisão de segredos pendente.

## 5. Abrir pacote sem Memory Bee

```sh
mkdir -p exports
cargo run --locked -- export testdata/claude/basic.jsonl --output exports/manual-basic
```

Abra `exports/manual-basic/HANDOFF.md` em editor de texto ou visualizador Markdown.
Confira pedido retido, último registro, omissões e links relativos para manifesto
e histórico. Copie a pasta para outro diretório e confira os mesmos links.
Não é necessário instalar Memory Bee para ler esses três arquivos.

Executar o mesmo comando novamente deve falhar com saída 1 e preservar a pasta.
Escolha outro nome para repetir o teste; não é necessário apagar resultados antigos.

## 6. Excluir conteúdo

```sh
cargo run --locked -- export testdata/claude/basic.jsonl --exclude-line 2 --output exports/manual-filtered
```

Esperado: apenas o pedido humano em `history.jsonl`; manifesto registra a exclusão.
Tentar `--exclude-line 99 --preview` deve falhar. Exportar `empty.jsonl` também.

## 7. Preservar avisos de leitura parcial

```sh
cargo run --locked -- export testdata/claude/truncated.jsonl --output exports/manual-partial
```

Esperado: pasta criada, saída 2 e avisos de perda no Markdown/manifesto. O histórico
contém os dois eventos recuperados, sem inventar o conteúdo da última linha.

## 8. Validar com Claude Code real — ainda pendente

Usar uma tarefa descartável com conteúdo sintético e registrar versão/ambiente.
Conferir manualmente texto, ferramentas, `cwd`, IDs, ramos, subagentes e compactação
contra a origem. Produzir fixtures reescritas a partir de divergências, sem copiar
transcrições reais para o repositório. Registrar resultados e limitações em
`docs/EXECUTION.md`; não concluir compatibilidade apenas por a leitura terminar.

Este roteiro não inicia Claude nem faz inferência. A implementação pode continuar
enquanto o mantenedor executa os itens depois, conforme sua orientação atual.

## 9. Referência Git explícita (pendente)

1. Em um projeto de teste com commit, rode `export <sessão-sintética> --project <pasta> --preview`.
2. Confira branch, commit e dirty contra `git status` e `git rev-parse HEAD`.
3. Crie um arquivo novo: dirty deve ser true e HANDOFF deve avisar que mudanças não estão incluídas.
4. Em checkout de teste detached, confira branch null e commit presente.
5. Use pasta sem repositório: saída 2, code_state unknown e aviso, mantendo contexto disponível.
6. Confira sem `--project`: campos Git null como antes.
7. Revise o remoto antes de compartilhar; sua ausência não significa falha da exportação.

Nenhum desses ensaios foi executado com conversas reais pelo agente.

## 10. Preparar projeto sintético para os próximos testes

Os itens 11–15 usam esta preparação, na mesma janela de terminal. Somente o
repositório temporário é alterado; não use um projeto de trabalho real nestes testes.

```sh
cargo build --locked
mb_root="$PWD"
mb_case="$(mktemp -d /tmp/memory-bee-manual.XXXXXX)"
mkdir "$mb_case/source"
git -C "$mb_case/source" init -b manual
git -C "$mb_case/source" config user.name 'Synthetic Test'
git -C "$mb_case/source" config user.email 'synthetic@example.invalid'
git -C "$mb_case/source" config commit.gpgsign false
git -C "$mb_case/source" config core.hooksPath /dev/null
printf 'base\n' > "$mb_case/source/tracked.txt"
printf 'original\n' > "$mb_case/source/unselected.txt"
git -C "$mb_case/source" add .
git -C "$mb_case/source" commit -m 'synthetic base'
mb_base="$(git -C "$mb_case/source" rev-parse HEAD)"
printf 'selected change\n' > "$mb_case/source/tracked.txt"
printf 'new file\n' > "$mb_case/source/new.txt"
printf 'leave outside\n' > "$mb_case/source/unselected.txt"
```

Esperado: base com dois arquivos; duas modificações e um arquivo novo no disco.
Guarde o valor de `mb_case` para encontrar os resultados depois.

## 11. Prévia de código e seleção explícita

```sh
"$mb_root/target/debug/memory-bee" export "$mb_root/testdata/claude/basic.jsonl" --project "$mb_case/source" --include-path tracked.txt --include-path new.txt --preview
```

Esperado: manifesto v2, code_state changes-included, base igual a `mb_base`,
selected_paths com new.txt/tracked.txt e duas entradas em changes. A prévia contém
patch do tracked.txt e conteúdo de new.txt. unselected.txt não entra nos payloads.
Índice e arquivos da origem não mudam. Sem --include-path, continua manifesto v1.

## 12. Gravar pacote e conferir reconstrução em checkout separado

```sh
"$mb_root/target/debug/memory-bee" export "$mb_root/testdata/claude/basic.jsonl" --project "$mb_case/source" --include-path tracked.txt --include-path new.txt --output "$mb_case/bundle"
cat "$mb_case/bundle/HANDOFF.md"
cat "$mb_case/bundle/manifest.json"
git clone --no-hardlinks "$mb_case/source" "$mb_case/receiver"
```

Esperado: saída 0, changes.patch e files/0001.txt, ambos com hashes no manifesto.
A entrada `add` mapeia files/0001.txt para new.txt; conferir antes de copiar.
Verificação/aplicação abaixo é alternativa manual via Git. Para o fluxo do Memory Bee, veja os itens 17–22.
Execute apenas no receiver sintético, depois de revisar conteúdo e manifesto:

```sh
test "$(git -C "$mb_case/receiver" rev-parse HEAD)" = "$mb_base" && git -C "$mb_case/receiver" apply --check "$mb_case/bundle/changes.patch" && git -C "$mb_case/receiver" apply "$mb_case/bundle/changes.patch"
test ! -e "$mb_case/receiver/new.txt" && cp -n "$mb_case/bundle/files/0001.txt" "$mb_case/receiver/new.txt"
cmp "$mb_case/source/tracked.txt" "$mb_case/receiver/tracked.txt"
cmp "$mb_case/source/new.txt" "$mb_case/receiver/new.txt"
cat "$mb_case/receiver/unselected.txt"
```

Esperado: cmp sem diferenças; unselected.txt continua `original`. Conferir também
hashes dos payloads com `shasum -a 256` e comparar com manifesto. Repetir exportação
para a mesma pasta deve falhar e preservar o pacote. Não tratar hashes como assinatura.

## 13. Binário omitido e caminho inválido recusado

```sh
printf '\000synthetic' > "$mb_case/source/binary.dat"
"$mb_root/target/debug/memory-bee" export "$mb_root/testdata/claude/basic.jsonl" --project "$mb_case/source" --include-path binary.dat --include-path tracked.txt --output "$mb_case/partial"
```

Esperado: saída 2, patch textual preservado, binary_unsupported nas omissões e
binário ausente dos payloads. Prévia com --include-path ../escape deve falhar com
saída 1; com --include-path sem --project, saída 64. Nenhuma pasta nesses erros.
Nomes de diretório não selecionam conteúdo recursivamente.

## 14. Possível segredo em código bloqueia gravação

```sh
printf 'api_key=synthetic-fixture-only\n' > "$mb_case/source/suspect.txt"
"$mb_root/target/debug/memory-bee" export "$mb_root/testdata/claude/basic.jsonl" --project "$mb_case/source" --include-path suspect.txt --output "$mb_case/blocked"
```

Esperado: saída 3, written false, achado code_selection com índice da seleção,
sem ecoar o valor e sem criar blocked. É um valor sintético, não uma credencial.
Prévia inclui conteúdo sensível por definição; não compartilhar saída sem revisão.
Retirar a seleção é a forma de deixar esse arquivo fora. Segredos em linhas antigas
removidas também bloqueiam, pois o patch contém essas linhas.

## 15. Staging, exclusão e modos

1. No source sintético, faça `git add tracked.txt`, edite tracked.txt novamente e
   exporte só ele. Deve representar disco contra mb_base, não só o conteúdo staged.
2. Remova tracked.txt do source sintético e exporte para pasta nova: changes deve
   marcar delete com hash/modo anteriores e resultado null.
3. No Unix, torne new.txt executável e exporte: result_mode deve ser 100755,
   enquanto o arquivo de transporte em files/ mantém permissões privadas.
4. Selecione um arquivo sem mudança: saída parcial, aviso sem diferença e sem
   payload inventado. Selecionar arquivo inexistente deve falhar.

## 16. Projeto com submódulos (pendente)

Em um projeto descartável com submódulo, exporte --project com ou sem seleção.
Esperado: aviso git_submodules_unverified e saída parcial. O estado interno do
submódulo não é inspecionado; dirty pode ser null se não houver outra mudança.
Selecionar o caminho do submódulo deve omiti-lo, sem copiar seu conteúdo. Não usar
esse resultado como certificação de que o submódulo está limpo.

## Registro dos ensaios

Todos os itens continuam **pendentes** até o mantenedor executá-los. Ao reportar,
informe número, sistema, commit do Memory Bee, resultado e mensagem de erro
(sanitizada). Atualizar esta tabela sem apagar observações anteriores.

| Itens | Estado | Evidência manual |
|---|---|---|
| 1–7: leitor e contexto | Pendente | — |
| 8: Claude real controlado | Pendente | — |
| 9: referência Git | Pendente | — |
| 10–12: seleção e reconstrução sintética | Pendente | — |
| 13–15: omissões, segredos e modos | Pendente | — |
| 16: submódulos | Pendente | — |

Aplicador e checagens automáticas foram implementados posteriormente; os itens 17–22 abaixo ainda precisam de validação manual. Testes automatizados não aprovam esses ensaios humanos.

## 17. Verificar pacote recebido (pendente)

Use o bundle sintético dos itens 10–12, preservado fora do checkout de destino:

```sh
cargo build --locked
"$mb_root/target/debug/memory-bee" verify "$mb_case/bundle"
```

Esperado: saída 0, valid true, format_version 2, duas mudanças, contagens de
omissões/avisos preservadas. Nada no pacote ou projeto é alterado. Repita com o
pacote de contexto do item 5: versão 1, zero mudanças. Valid não autentica origem
nem afirma ausência de segredos.

## 18. Conferir checkout sem escrever (pendente)

Crie OUTRO checkout, pois receiver do item 12 já foi modificado:

```sh
git clone --no-hardlinks "$mb_case/source" "$mb_case/receiver-app"
"$mb_root/target/debug/memory-bee" apply "$mb_case/bundle" --project "$mb_case/receiver-app" --check
git -C "$mb_case/receiver-app" status --porcelain
cat "$mb_case/receiver-app/tracked.txt"
```

Esperado: checked true, written false, changes 2, saída 0. Status vazio, tracked.txt
continua base e new.txt não existe. Rodar apply sem --check/--write deve retornar 64.

## 19. Aplicar explicitamente e conferir resultado (pendente)

```sh
"$mb_root/target/debug/memory-bee" apply "$mb_case/bundle" --project "$mb_case/receiver-app" --write
cat "$mb_case/receiver-app/tracked.txt"
cat "$mb_case/receiver-app/new.txt"
cat "$mb_case/receiver-app/unselected.txt"
git -C "$mb_case/receiver-app" status --short
git -C "$mb_case/receiver-app" diff --cached --exit-code
```

Esperado: saída 0, written true; textos selected change / new file / original.
Tracked modificado e new não rastreado; índice sem mudanças, nenhum commit/push.
Não comparar source após item 15, que já pode ter sido alterado: referência é o
bundle gravado no item 12. Repetir --write deve recusar checkout alterado e preservar
resultado. Pasta .memory-bee-apply-* não deve permanecer após sucesso.

## 20. Detectar adulteração antes da aplicação (pendente)

```sh
cp -R "$mb_case/bundle" "$mb_case/tampered"
printf 'changed\n' >> "$mb_case/tampered/HANDOFF.md"
"$mb_root/target/debug/memory-bee" verify "$mb_case/tampered"
```

Esperado: saída 1 e payload hash mismatch. A mesma pasta em apply deve ser recusada
antes de qualquer escrita. Em outras cópias, testar manifesto com versão 99,
caminho ../escape e payload ausente: todos recusados. Não alterar o bundle original.

## 21. Base divergente e trabalho local preservados (pendente)

```sh
git clone --no-hardlinks "$mb_case/source" "$mb_case/receiver-blocked"
printf 'my local work\n' > "$mb_case/receiver-blocked/tracked.txt"
"$mb_root/target/debug/memory-bee" apply "$mb_case/bundle" --project "$mb_case/receiver-blocked" --write
cat "$mb_case/receiver-blocked/tracked.txt"
```

Esperado: saída 1, conteúdo local preservado, nenhum new.txt criado. Em um checkout
sintético adicional, crie commit diferente da base e tente --check: deve recusar
por commit divergente. Não usar reset/clean para contornar a recusa num projeto real.

## 22. Colisão ignorada, symlink e recuperação (pendente)

Em novo clone sintético limpo, configure .git/info/exclude para ignorar new.txt e
crie new.txt com conteúdo próprio. apply --check deve recusar colisão mesmo com
Git status limpo; conteúdo deve permanecer intacto. No Unix, testar também destino
novo como symlink para arquivo descartável externo: deve recusar sem alterar alvo.

Para qualquer erro real de gravação/interrupção, conferir a mensagem antes de
repetir: uma falha com rollback informa originais restaurados; recuperação incompleta
informa pasta .memory-bee-apply-* com recovery.json e old-N. Preservar cópia dos
backups e conferir destinos manualmente. Não provocar interrupções em projetos reais.
Ensaio de interrupção abrupta/falha elétrica NÃO foi validado automaticamente.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 17–19: verify/check/write | Pendente | — |
| 20–22: adulteração, base, colisões e recuperação | Pendente | — |

## 23. Inspeção Codex por arquivo sintético (pendente)

Na raiz do repositório:

```sh
cargo run --locked -- inspect-codex testdata/codex/basic.jsonl
```

Esperado: saída 0, agent codex, compatibility unverified, state read, sete registros
físicos e cinco eventos. Mensagem inicial na linha 3/bloco 1; sessão synthetic-session,
turno synthetic-turn. Chamada e resultado ligados por call-1. Nenhum comando do
histórico executado, nenhuma pasta de exportação criada e origem preservada.

## 24. Compactação, perdas e mensagens auxiliares Codex (pendente)

```sh
cargo run --locked -- inspect-codex testdata/codex/losses.jsonl
```

Esperado: saída 2, state partial, três eventos. Pedido aparece uma vez; checkpoint
identificado como checkpoint; texto após compactação preservado. Diagnósticos para
event_msg, reasoning, campos omitidos e imagem. NOT-REPLAYED e synthetic-opaque
não aparecem na saída. Rollback é diagnosticado como event_msg omitido; o leitor
não afirma reconstruir a conversa ativa após essa operação.

## 25. Codex vazio, truncado e uso inválido (pendente)

```sh
cargo run --locked -- inspect-codex testdata/codex/empty.jsonl
cargo run --locked -- inspect-codex testdata/codex/truncated.jsonl
cargo run --locked -- inspect-codex testdata/codex/basic.jsonl --leaf nonexistent
```

Esperado: respectivamente saída 0/state empty; saída 2 com cinco eventos e
incomplete_final_line na linha 8; saída 64 com ajuda (seleção Claude não se aplica).

## 26. Compatibilidade real controlada do Codex (pendente)

Criar posteriormente uma sessão descartável com uma tarefa sintética e sem segredos.
Usar apenas o arquivo explicitamente escolhido pelo mantenedor; não varrer o perfil.
Anotar versão do Codex, sistema e tipos observados, sem versionar conversa real.

```sh
# Substituir o caminho por uma cópia local do rollout controlado.
cargo run --locked -- inspect-codex /caminho/controlado/rollout.jsonl
```

Conferir papéis, ordem, texto, chamadas/resultados, linha/bloco, sessão/turno,
compactações e diagnósticos contra o arquivo. Campos de configuração e registros
auxiliares podem tornar a saída parcial; isso é esperado no perfil inicial.
Comparar bytes/hash da origem antes e depois. Não publicar a saída sem revisão de
segredos. Diferenças de formato devem ser registradas como pendências, sem declarar
suporte geral. Descoberta, exportação Codex e lançamento continuam fora deste recorte.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 23–25: inspeção Codex sintética, perdas e erros | Pendente | — |
| 26: compatibilidade real controlada Codex | Pendente | — |

## 27. Prévia, gravação e verify Codex (pendente)

```sh
cargo run --locked -- export-codex testdata/codex/basic.jsonl --preview
mkdir -p exports
cargo run --locked -- export-codex testdata/codex/basic.jsonl --output exports/manual-codex
cargo run --locked -- verify exports/manual-codex
```

Esperado: saída 0; manifesto v1 com source.agent codex, cinco eventos, Git desconhecido,
redaction pending-review. HANDOFF abre sem Memory Bee, informa perfil experimental
não certificado e registro físico. history preserva sessão/turno, linha/bloco e
ferramentas. verify informa valid true. Repetir gravação deve falhar (saída 1), sem
modificar destino. Comparar bytes/hash da origem antes/depois. Escolher pasta nova
se já houver exports/manual-codex de um ensaio anterior.

## 28. Exclusões e perdas Codex (pendente)

```sh
cargo run --locked -- export-codex testdata/codex/basic.jsonl --exclude-line 3 --preview
cargo run --locked -- export-codex testdata/codex/losses.jsonl --preview
cargo run --locked -- export-codex testdata/codex/empty.jsonl --preview
cargo run --locked -- export-codex testdata/codex/basic.jsonl --preview --leaf synthetic-turn
```

Esperado: respectivamente saída 0 (quatro eventos, sequência 1–4, linhas originais
4–7, sem pedido humano retido); saída 2 (três eventos, checkpoint e omissões,
sem NOT-REPLAYED); saída 1 (vazio); saída 64 (--leaf não suportado). Excluir linha
1 de basic deve falhar com 1, porque contém metadados e nenhum evento exportável.

## 29. Detecção de segredos Codex (pendente)

Usar apenas o marcador sintético abaixo, sem credenciais reais:

```sh
mkdir -p exports
cat > exports/manual-codex-secret.jsonl <<'JSONL'
{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"password=synthetic-only-value"}]}}
{"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Resposta sintética."}]}}
JSONL
cargo run --locked -- export-codex exports/manual-codex-secret.jsonl --output exports/manual-codex-blocked
cargo run --locked -- export-codex exports/manual-codex-secret.jsonl --exclude-line 1 --preview
```

Esperado: primeira saída 3, written false, achado na linha 1 sem ecoar valor e sem
criar destino. Segunda prévia não contém o texto excluído nem findings. IDs sensíveis
em fase/turno/namespace também bloqueiam; excluir só a mensagem não remove um ID
herdado que continue em outros eventos. A ausência de achados não dispensa revisão.

## 30. Referência e código selecionado Codex (pendente)

Usar o repositório sintético e clone limpo do roteiro 10–19, com mudanças em
tracked.txt/new.txt e base exata no clone. Substituir os caminhos abaixo pelos
diretórios descartáveis preparados; não usar projeto real para este ensaio.

```sh
cargo run --locked -- export-codex testdata/codex/basic.jsonl --project /caminho/sintetico/origem --include-path tracked.txt --include-path new.txt --output exports/manual-codex-code
cargo run --locked -- verify exports/manual-codex-code
cargo run --locked -- apply exports/manual-codex-code --project /caminho/sintetico/clone --check
cargo run --locked -- apply exports/manual-codex-code --project /caminho/sintetico/clone --write
```

Esperado: manifesto v2 com agent codex, commit base e payloads selecionados; check
não escreve; write reproduz conteúdo selecionado no clone. Origem, índice e commit
não são alterados pela exportação/aplicação. Retirar --include-path em novo destino
produz v1/base-reference e não inclui código. Descoberta e lançamento não ocorrem.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 27–29: pacotes Codex, exclusões, perdas e segredos | Pendente | — |
| 30: referência Git e roundtrip Codex | Pendente | — |

## 31. Descoberta Codex com raiz/projeto explícitos (pendente)

```sh
cargo run --locked -- sessions-codex --root testdata/codex-sessions --project /synthetic/project
cargo run --locked -- sessions-codex --project /synthetic/other --root testdata/codex-sessions
cargo run --locked -- sessions-codex --root testdata/codex-sessions --project /synthetic/absent
```

Esperado: saída 0 nos três casos, quatro arquivos inspecionados. Primeiro retorna
três caminhos (arquivo datado, copy.jsonl e workers/worker.jsonl); segundo retorna
other/session.jsonl; terceiro, lista vazia. Dois arquivos com synthetic-session
permanecem separados; o nome workers não vira relação de subagente. Cada sessão tem
três registros/um evento, versão synthetic e diagnóstico unverified_compatibility.
A saída não contém o texto do pedido. Conferir origem sem alterações; nada é exportado.

## 32. Selecionar caminho para inspecionar e exportar (pendente)

Escolher o caminho datado retornado no item 31; abaixo está seu equivalente relativo.
Use destino novo para não colidir com ensaios anteriores:

```sh
cargo run --locked -- inspect-codex testdata/codex-sessions/2026/09/24/session.jsonl
cargo run --locked -- export-codex testdata/codex-sessions/2026/09/24/session.jsonl --preview
mkdir -p exports
cargo run --locked -- export-codex testdata/codex-sessions/2026/09/24/session.jsonl --output exports/manual-codex-discovered
cargo run --locked -- verify exports/manual-codex-discovered
```

Esperado: inspeção com um evento, prévia correspondente, pacote v1 com agent codex
e verify valid true. Descoberta/seleção não inicia agente, não consulta Git e não
reconstrói conversa ativa. Compatibilidade e revisão de segredos continuam pendentes.

## 33. Profundidade, metadados conflitantes e links (pendente)

Preparar uma árvore descartável com cópias sintéticas, sem dados pessoais:

```sh
mkdir -p exports/manual-codex-tree/a/b/c/d
cp testdata/codex-sessions/copy.jsonl exports/manual-codex-tree/a/b/c/visible.jsonl
cp testdata/codex-sessions/copy.jsonl exports/manual-codex-tree/a/b/c/d/too-deep.jsonl
cat testdata/codex-sessions/copy.jsonl testdata/codex-sessions/workers/worker.jsonl > exports/manual-codex-tree/mixed.jsonl
ln -s a/b/c/visible.jsonl exports/manual-codex-tree/link.jsonl
cargo run --locked -- sessions-codex --root exports/manual-codex-tree --project /synthetic/project
```

Esperado: saída 2, apenas visible.jsonl listado; depth_limit para d, symlink_skipped
para link.jsonl e conflicting_session_metadata para mixed.jsonl. Too-deep não é
aberto e sessões misturadas não são apresentadas como uma única sessão. Repetir o
comando com --root exports/manual-codex-tree/a permite alcançar too-deep, pois a
profundidade é relativa à raiz escolhida. Nessa segunda busca, a saída esperada é
0 com dois arquivos; mixed/link ficam fora da raiz escolhida.

## 34. Erros e descoberta real controlada (pendente)

```sh
cargo run --locked -- sessions-codex
cargo run --locked -- sessions-codex --root testdata/codex-sessions
cargo run --locked -- sessions-codex --root testdata/codex-sessions --project /synthetic/project --leaf synthetic-session
cargo run --locked -- sessions-codex --root testdata/codex-sessions/no-such-root --project /synthetic/project
```

Esperado: saídas 64, 64, 64 e 1. Nenhum comando consulta home/perfis como fallback.
Os limites de entradas/arquivos/bytes são cobertos automaticamente com valores
reduzidos na biblioteca; a CLI mantém os padrões documentados.

Posteriormente, usar uma pasta explicitamente escolhida com cópia de uma sessão
controlada do item 26. Informar o cwd histórico exato, comparar caminhos/contagens
com inspect-codex e anotar versão/sistema/diferenças. Não varrer perfis reais sem
escolher a raiz e considerar que a descoberta lê todos os projetos dentro dela.
Metadados também podem conter dados pessoais; não versionar nem publicar relatórios.
Compatibilidade real não é certificada pelos testes sintéticos.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 31–33: descoberta sintética, seleção, profundidade e ambiguidades | Pendente | — |
| 34: uso inválido e descoberta real controlada | Pendente | — |

## 35. Preparar retomada no mesmo checkout (pendente)

Cria um repositório sintético, um pacote v2 e um clone limpo na base:

```sh
mkdir -p exports/manual-resume
git init -q -b synthetic exports/manual-resume/source
printf 'base\n' > exports/manual-resume/source/a.txt
git -C exports/manual-resume/source add a.txt
git -C exports/manual-resume/source -c user.name=Synthetic -c user.email=synthetic@example.invalid commit -q -m base
git clone -q exports/manual-resume/source exports/manual-resume/target
printf 'result\n' > exports/manual-resume/source/a.txt
cargo run --locked -- export testdata/claude/basic.jsonl --output exports/manual-resume/bundle --project exports/manual-resume/source --include-path a.txt
cargo run --locked -- prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --preview
```

Esperado: saída 0, mode same-checkout, base_match match, changes base, attention
vazio e três passos: apply --check, apply --write e claude com o prompt antes de
--add-dir. O prompt não contém o pedido do histórico. target/a.txt continua "base"
e nada é lançado. Os passos usam `memory-bee` instalado; com cargo, substitua por
`cargo run --locked --`.

## 36. Mudanças aplicadas e arquivo de prompt (pendente; depende do 35)

```sh
cargo run --locked -- apply exports/manual-resume/bundle --project exports/manual-resume/target --write
cargo run --locked -- prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --output exports/manual-resume/prompt.md
ls -l exports/manual-resume/prompt.md
cargo run --locked -- prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --output exports/manual-resume/prompt.md
```

Esperado: primeira preparação com saída 0, changes applied e um único passo, cujo
shell usa `"$(cat …/prompt.md)"`; arquivo com permissão -rw-------. A repetição
retorna 1 sem sobrescrever. Alterar target/a.txt para outro conteúdo e repetir com
novo arquivo deve dar saída 2 com project_dirty e changes_mixed.

## 37. Preparar nova worktree para Codex (pendente; depende do 35)

```sh
cargo run --locked -- prepare-resume exports/manual-resume/bundle --target codex --project exports/manual-resume/source --worktree exports/manual-resume/tree --preview
```

Esperado: saída 2 apenas com codex_bundle_read, mode new-worktree e passos
`git worktree add --detach <tree> <base>`, apply --check/--write na nova pasta e
`codex -C <tree> <prompt>`. A pasta tree não é criada pela preparação. Executar os
passos manualmente, conferir tree/a.txt = "result" e a origem inalterada. Depois:
`git -C exports/manual-resume/source worktree remove --force ../tree` antes de
apagar exports/manual-resume.

## 38. Erros e lançamento real controlado (pendente)

```sh
cargo run --locked -- prepare-resume examples/bundle-v1 --target gemini --project . --preview
cargo run --locked -- prepare-resume examples/bundle-v1 --target claude --project . --worktree exports/x --preview
cargo run --locked -- prepare-resume examples/bundle-v1 --target claude --project . --preview --output exports/p.md
```

Esperado: saídas 64, 1 (pacote v1 sem base não permite nova worktree) e 64.

Posteriormente, com pacote sintético e conta de teste, executar manualmente o passo
final para Claude Code e Codex. Anotar versões, se o prompt foi recebido inteiro,
se o agente leu HANDOFF.md fora da raiz (sandbox do Codex), se pediu confirmação
antes de editar e se a origem continuou intacta. Registrar diferenças de flags. Não
usar conversas reais nem publicar prompts ou pacotes. Lançamento não é certificado
pelos testes automatizados.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 35–37: retomada no mesmo checkout, prompt em arquivo e nova worktree | Pendente | — |
| 38: uso inválido e lançamento real controlado | Pendente | — |

## 39. Lançamento confirmado com agente falso (pendente; depende do 36)

Usa um `claude` falso para não iniciar o agente real:

```sh
mkdir -p exports/manual-resume/fake-bin
printf '#!/bin/sh\npwd > "$0.calls"; printf "%%s\\n" "$@" >> "$0.calls"\n' > exports/manual-resume/fake-bin/claude
chmod +x exports/manual-resume/fake-bin/claude
cargo build --locked
TOKEN=$(target/debug/memory-bee prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --preview | python3 -c 'import json,sys;print(json.load(sys.stdin)["confirmation"])')
PATH="$PWD/exports/manual-resume/fake-bin:/usr/bin:/bin" target/debug/memory-bee prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --output exports/manual-resume/launch.md --launch "$TOKEN"
cat exports/manual-resume/fake-bin/claude.calls
PATH="$PWD/exports/manual-resume/fake-bin:/usr/bin:/bin" target/debug/memory-bee prepare-resume exports/manual-resume/bundle --target claude --project exports/manual-resume/target --output exports/manual-resume/launch2.md --launch 0000000000000000
```

Esperado: o lançamento retorna 0 com launched true e launch_exit_code 0; o arquivo
.calls mostra o diretório target, o prompt e `--add-dir <pacote>`. O último comando
retorna 1 por confirmação divergente e não cria launch2.md. Alterar target/a.txt
depois da prévia também deve invalidar o token.

## 40. Lançamento real controlado (pendente)

Com pacote sintético, conta de teste e checkout descartável, repetir o item 39 sem
o PATH falso, uma vez com `--target claude` e outra com `--target codex`. Conferir
se o agente abre no diretório esperado, recebe o prompt inteiro, consegue ler
HANDOFF.md, pede confirmação antes de editar e se, ao sair, o relatório traz o
código de saída. Testar também saída com erro (4) e agente ausente do PATH (1),
confirmando que pacote, prompt e origem ficam intactos. Anotar versões e
diferenças de flags. Não usar conversas reais.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 39: lançamento confirmado com agente falso e token divergente | Pendente | — |
| 40: lançamento real controlado em Claude Code e Codex | Pendente | — |

## 41. Protótipo da colmeia no navegador (pendente)

Abrir o protótipo privado [Clareira da Colmeia](https://claude.ai/artifact/W5a8EAd4UKXLS84Nsqa8Ab)
e conferir, clicando no terminal para dar foco:

- Abertura: os gominhos nascem um a um a partir do galho e a abelha circula a
  colmeia antes de patrulhar as flores.
- `←`/`→`: o gominho aceso muda, a abelha voa até ele, deposita e o nível de mel
  pulsa; o nome do projeto aparece à esquerda da colmeia e na linha abaixo da cena.
- `receipts-ocr` (terceiro gominho): contorno piscando e `?` sobre a abelha; a
  lista mostra a sessão ambígua.
- `enter` pousa a abelha no gominho; `esc` a solta. `n` acrescenta um gominho.
  `f` solta pólen e vagalumes. Vinte segundos sem tecla: ela dorme no alto da
  colmeia com zzz; qualquer tecla acorda.
- Controles: 140×40, 100×30 (colmeia miúda), 60×20 (três gominhos) e 40×12 (sem
  cena, cabeçalho com `n/total projeto`); paletas entardecer e fim de tarde;
  24 bits, 256 e `NO_COLOR` (gominhos vazios ocos, sem fundo colorido); glifos
  básicos; movimento reduzido (quadro fixo com a abelha pousada).

Esperado: nada corta texto, a colmeia nunca invade a grama, a abelha fica dentro
da cena e o tema claro mantém a abelha legível sobre o céu pêssego. Anotar
navegador e o que destoar; o visual só é aprovado com esse registro.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 41: protótipo da colmeia, teclas, tamanhos e modos | Pendente | — |

## 42. Dashboard de terminal em terminal real (pendente)

Testes automatizados cobrem `--once` e a lógica de teclas sem terminal real
(`tests/dashboard.rs`, testes unitários em `src/tui/app.rs`). Falta o ensaio
num terminal de verdade:

```sh
cargo build --locked
target/debug/memory-bee dashboard --project /synthetic/project --claude-root testdata/claude-projects --codex-root testdata/codex-sessions --bundle examples/bundle-v1
```

Conferir: `↑`/`↓` move a seleção; `tab`/`enter` abre o detalhe e `esc` volta;
na sessão `arbitrary/session.jsonl`, `1` e `2` trocam a ponta do ramo e o
detalhe atualiza; `r` mostra a prévia de retomada e `v` o resultado de
`verify`; `t` troca o destino entre `claude` e `codex`; `?` mostra os atalhos;
`q` sai e devolve o terminal ao estado normal (sem tela alternada nem modo
bruto grudados). Redimensionar a janela do terminal abaixo de 80 colunas deve
trocar para um painel por vez sem cortar a contagem de eventos, diagnósticos
ou o símbolo de estado — só o nome da sessão trunca com `…`. Repetir com
`--no-color` e com `--theme light`. `Ctrl+C` também deve sair e restaurar o
terminal, como `q` (o modo bruto impede o sinal SIGINT normal; o app trata
`Ctrl+C` como tecla).

Esperado: navegação e prévias não escrevem. As novas ações exigem confirmação
e são ensaiadas nos itens 43–45. Anotar terminal e sistema operacional usados.

| Item novo | Estado | Evidência manual |
|---|---|---|
| 42: dashboard em terminal real, teclas, redimensionamento e sinais | Pendente | — |


## 43. Exportar contexto pela TUI (pendente)

```sh
cargo build --locked
mb_tui_tmp=$(mktemp -d)
printf '%s\n' "$mb_tui_tmp"
target/debug/memory-bee dashboard --project /synthetic/project --claude-root testdata/claude-projects --codex-root testdata/codex-sessions --no-color
```

Selecionar `arbitrary/session.jsonl`, abrir detalhe, escolher ramo `2`, pressionar
`e` e digitar o caminho absoluto da pasta temporária impresso acima seguido de
`/pacote` (sem aspas nem variável). Enter, exclusões vazias, Enter. Revisar
origem/ramo/destino e o conteúdo com PgDn/PgUp/Home. Enter sozinho não grava.
Esc cancela; repetir e digitar `EXPORTAR` para gravar. Repetir com mesmo destino
para conferir recusa de sobrescrita; conferir `verify` pela CLI. Exportar também
uma sessão Codex; repetir com uma linha existente em `exclude_lines`, conferindo
omissões e histórico. Uma linha inexistente ou zero deve gerar erro sem pacote.

Esperado: somente contexto v1, pacote novo verificável, ramo escolhido preservado,
origem intacta e pacote exportado disponível em `r`. Redimensionar para 40×12;
prévia rolável e confirmação/cancelamento devem continuar acessíveis.

## 44. Aplicar pacote pela TUI (pendente)

Usar pacote v2 e checkout sintéticos separados preparados nos itens 17–22.

```sh
# Substituir os caminhos por aqueles do ensaio sintético.
target/debug/memory-bee dashboard --project /caminho/checkout-limpo --claude-root testdata/claude-projects --bundle /caminho/pacote-v2
```

`r`, `a`: conferir checkout, manifesto e resumo. Esc deve cancelar sem alterar
arquivos. Reabrir e digitar `APLICAR`. Esperado: mesmo resultado de `apply --write`,
sem commit/push nem staging Git. Em checkout sujo/base divergente deve recusar.
Alterar um arquivo em outro terminal depois da prévia e antes de confirmar:
a escrita deve ser recusada, preservando essa alteração.

## 45. Prompt e lançamento pela TUI (pendente)

No checkout sintético do item anterior, `r`, `t` para escolher destino, `p` para
arquivo novo fora do checkout/pacote; revisar e digitar `GRAVAR`. Conferir prompt
0600 e recusa de sobrescrita. `l` pede outro arquivo e mostra comando, diretório,
pontos de atenção e token; Enter vazio ou Esc não lançam. Alterar o checkout entre
prévia/confirmação deve recusar antes de gravar. Apply pendente não pode ser pulado.

Para ensaio automatizado com agente falso, sem credenciais nem modelos:

```sh
python3 scripts/check_dashboard_pty.py
```

Esperado: agente falso recebe TTY sem modo bruto, saída 7 aparece no dashboard,
prompt é preservado, painel retorna e `q` restaura terminal. Esse teste automatizado
não substitui o ensaio humano. Com agentes reais, seguir o controle sintético do
item 40 e registrar versões, autenticação, aceitação do prompt e retorno à TUI.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 43: exportação por teclado, ramo, exclusões e cancelamento | Pendente | — |
| 44: aplicação confirmada e checkout alterado entre prévia/escrita | Pendente | — |
| 45: prompt e lançamento, restauração e agentes reais | Pendente | — |

## 46. Conversa unificada e permissões simuladas (pendente)

```sh
cargo build --locked
mb_workspace_tmp=$(mktemp -d)
mkdir "$mb_workspace_tmp/project"
target/debug/memory-bee workspace --demo --project "$mb_workspace_tmp/project" --state "$mb_workspace_tmp/state" --no-color
```

Enter abre a conversa. Enviar tarefa sintética, abrir F2 durante eventos, Esc
volta com texto digitado preservado. Tab abre permissão: negar, depois repetir
e permitir uma vez. Ctrl+C durante evento/permissão interrompe; aprovação antiga
não pode ser aceita. Enviar `[erro]`: erro aparece sem perder histórico.
Ctrl+Q durante turno exige escolha; Esc cancela. Nenhum arquivo do projeto muda.

Repetir com `--agent codex` e outra pasta `--state`; em estado existente a sessão
persistida prevalece. Esperado: SIMULAÇÃO visível, nenhum login/modelo/processo
Claude ou Codex; ferramentas/diff são fictícios. Não usar conversas ou tokens reais.

## 47. Perfis, contexto e persistência da demo (pendente)

Na sessão do item 46, F2 → `/accounts`: adicionar perfil fictício. Com turno
parado, selecionar outro agente/perfil, revisar passagem com PgDn/Home e digitar
`CONTINUAR`; Esc antes disso preserva origem sem nova sessão. `/sessions` deve
permitir consultar a origem intacta. Perfil não autentica nem modifica conta real.

Sair e executar o mesmo comando: histórico reaparece sem reenviar mensagens.
Segundo processo com mesma pasta deve recusar. Para simular crash, usar apenas
estado descartável e encerrar seu processo abruptamente: conferir que não resta
processo, inspecionar/remover manualmente `workspace.lock`, reabrir e verificar
interrupção registrada. `workspace.new` remanescente também requer inspeção;
não remover arquivos de outra sessão. Corrupção de JSON e projeto diferente devem
recusar preservando os bytes originais. Rascunho/tema não são restaurados.

## 48. Exportar a simulação e revisar origem (pendente)

F2 → `/export`, informar pasta nova absoluta fora do projeto, excluir número de
registro existente (visível entre colchetes), revisar manifesto/Markdown/histórico.
Esc cancela. Repetir e digitar `EXPORTAR`; destino existente deve ser recusado.

```sh
# Substitua pelo destino usado no formulário.
target/debug/memory-bee verify /caminho/do/pacote-demo
```

Esperado: fonte `memory-bee-demo`, aviso de origem desconhecida, marca SIMULAÇÃO,
proveniência `user_input`/`simulated`, omissões das exclusões. Snapshot após erro
ou interrupção é parcial. Nenhum patch ou referência Git inferida. Detector de
segredos bloqueia escrita de suspeitas até exclusão; não promete detectar tudo.

## 49. Terminal e limites de lançamento (pendente)

Testar 100×30, 60×20, 40×12 e 30×10, claro/escuro e `NO_COLOR`. Rolar menus longos,
prévia, colar texto multilinha (não deve enviar), Ctrl+J, Unicode e Backspace.
Entrada curta mantém Enter/F2 acessíveis; abaixo do mínimo há aviso. Sair e
conferir terminal restaurado. A colmeia deste recorte é estática.

```sh
target/debug/memory-bee workspace --demo --project . --agent codex --once --width 40 --height 12 --no-color
# Deve recusar sem criar o estado nem iniciar agente:
target/debug/memory-bee workspace --project . --state /tmp/bee-live-refused
# Automatizado, separado do ensaio humano; requer dependências de validação:
python3 scripts/check_workspace_pty.py
```

Aceite real continua pendente: os dois agentes com assinaturas existentes,
edição/execução real, permitir/negar, interrupção/retomada, exportar após falha/limite
e dois perfis sem mistura. Não interpretar a demo como evidência desses aceites.

| Itens novos | Estado | Evidência manual |
|---|---|---|
| 46: conversa e permissões simuladas | Pendente | — |
| 47: perfis fictícios, contexto, reinício e crash | Pendente | — |
| 48: exportação revisada com proveniência e omissões | Pendente | — |
| 49: terminal, acessibilidade e recusa de modo real | Pendente | — |

## 50. Artifact visual e abelha no campo (ensaio humano pendente)

```sh
open docs/prototypes/workspace.html
```

Abrir conversa e enviar tarefa sintética. Esperado: abelha à direita do campo
amarelo; pensar → ler → editar → aguardar permissão. Permitir mostra execução
simulada e depois check; negar pousa sem executar. Interromper durante execução
não pode produzir sucesso atrasado. Enviar `[erro]` mostra falha preservando o
histórico. Após inatividade, digitar acorda. Conferir movimento desligado, sem
cor, tema claro/escuro, larguras 140/100/60/40 e janela estreita. A cena inicial
continua disponível. Nenhum teste, arquivo ou agente real é executado pelo HTML.
Esse roteiro não valida o renderizador Rust, ainda pendente.
