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

## 5. Abrir pacote sem Memory Pier

```sh
mkdir -p exports
cargo run --locked -- export testdata/claude/basic.jsonl --output exports/manual-basic
```

Abra `exports/manual-basic/HANDOFF.md` em editor de texto ou visualizador Markdown.
Confira pedido retido, último registro, omissões e links relativos para manifesto
e histórico. Copie a pasta para outro diretório e confira os mesmos links.
Não é necessário instalar Memory Pier para ler esses três arquivos.

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
mp_root="$PWD"
mp_case="$(mktemp -d /tmp/memory-pier-manual.XXXXXX)"
mkdir "$mp_case/source"
git -C "$mp_case/source" init -b manual
git -C "$mp_case/source" config user.name 'Synthetic Test'
git -C "$mp_case/source" config user.email 'synthetic@example.invalid'
git -C "$mp_case/source" config commit.gpgsign false
git -C "$mp_case/source" config core.hooksPath /dev/null
printf 'base\n' > "$mp_case/source/tracked.txt"
printf 'original\n' > "$mp_case/source/unselected.txt"
git -C "$mp_case/source" add .
git -C "$mp_case/source" commit -m 'synthetic base'
mp_base="$(git -C "$mp_case/source" rev-parse HEAD)"
printf 'selected change\n' > "$mp_case/source/tracked.txt"
printf 'new file\n' > "$mp_case/source/new.txt"
printf 'leave outside\n' > "$mp_case/source/unselected.txt"
```

Esperado: base com dois arquivos; duas modificações e um arquivo novo no disco.
Guarde o valor de `mp_case` para encontrar os resultados depois.

## 11. Prévia de código e seleção explícita

```sh
"$mp_root/target/debug/memory-pier" export "$mp_root/testdata/claude/basic.jsonl" --project "$mp_case/source" --include-path tracked.txt --include-path new.txt --preview
```

Esperado: manifesto v2, code_state changes-included, base igual a `mp_base`,
selected_paths com new.txt/tracked.txt e duas entradas em changes. A prévia contém
patch do tracked.txt e conteúdo de new.txt. unselected.txt não entra nos payloads.
Índice e arquivos da origem não mudam. Sem --include-path, continua manifesto v1.

## 12. Gravar pacote e conferir reconstrução em checkout separado

```sh
"$mp_root/target/debug/memory-pier" export "$mp_root/testdata/claude/basic.jsonl" --project "$mp_case/source" --include-path tracked.txt --include-path new.txt --output "$mp_case/bundle"
cat "$mp_case/bundle/HANDOFF.md"
cat "$mp_case/bundle/manifest.json"
git clone --no-hardlinks "$mp_case/source" "$mp_case/receiver"
```

Esperado: saída 0, changes.patch e files/0001.txt, ambos com hashes no manifesto.
A entrada `add` mapeia files/0001.txt para new.txt; conferir antes de copiar.
Verificação/aplicação abaixo é manual via Git, não funcionalidade do Memory Pier.
Execute apenas no receiver sintético, depois de revisar conteúdo e manifesto:

```sh
test "$(git -C "$mp_case/receiver" rev-parse HEAD)" = "$mp_base" && git -C "$mp_case/receiver" apply --check "$mp_case/bundle/changes.patch" && git -C "$mp_case/receiver" apply "$mp_case/bundle/changes.patch"
test ! -e "$mp_case/receiver/new.txt" && cp -n "$mp_case/bundle/files/0001.txt" "$mp_case/receiver/new.txt"
cmp "$mp_case/source/tracked.txt" "$mp_case/receiver/tracked.txt"
cmp "$mp_case/source/new.txt" "$mp_case/receiver/new.txt"
cat "$mp_case/receiver/unselected.txt"
```

Esperado: cmp sem diferenças; unselected.txt continua `original`. Conferir também
hashes dos payloads com `shasum -a 256` e comparar com manifesto. Repetir exportação
para a mesma pasta deve falhar e preservar o pacote. Não tratar hashes como assinatura.

## 13. Binário omitido e caminho inválido recusado

```sh
printf '\000synthetic' > "$mp_case/source/binary.dat"
"$mp_root/target/debug/memory-pier" export "$mp_root/testdata/claude/basic.jsonl" --project "$mp_case/source" --include-path binary.dat --include-path tracked.txt --output "$mp_case/partial"
```

Esperado: saída 2, patch textual preservado, binary_unsupported nas omissões e
binário ausente dos payloads. Prévia com --include-path ../escape deve falhar com
saída 1; com --include-path sem --project, saída 64. Nenhuma pasta nesses erros.
Nomes de diretório não selecionam conteúdo recursivamente.

## 14. Possível segredo em código bloqueia gravação

```sh
printf 'api_key=synthetic-fixture-only\n' > "$mp_case/source/suspect.txt"
"$mp_root/target/debug/memory-pier" export "$mp_root/testdata/claude/basic.jsonl" --project "$mp_case/source" --include-path suspect.txt --output "$mp_case/blocked"
```

Esperado: saída 3, written false, achado code_selection com índice da seleção,
sem ecoar o valor e sem criar blocked. É um valor sintético, não uma credencial.
Prévia inclui conteúdo sensível por definição; não compartilhar saída sem revisão.
Retirar a seleção é a forma de deixar esse arquivo fora. Segredos em linhas antigas
removidas também bloqueiam, pois o patch contém essas linhas.

## 15. Staging, exclusão e modos

1. No source sintético, faça `git add tracked.txt`, edite tracked.txt novamente e
   exporte só ele. Deve representar disco contra mp_base, não só o conteúdo staged.
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
informe número, sistema, commit do Memory Pier, resultado e mensagem de erro
(sanitizada). Atualizar esta tabela sem apagar observações anteriores.

| Itens | Estado | Evidência manual |
|---|---|---|
| 1–7: leitor e contexto | Pendente | — |
| 8: Claude real controlado | Pendente | — |
| 9: referência Git | Pendente | — |
| 10–12: seleção e reconstrução sintética | Pendente | — |
| 13–15: omissões, segredos e modos | Pendente | — |
| 16: submódulos | Pendente | — |

Aplicador do Memory Pier, checagem automática da base no recebimento e proteção
contra conflitos no destino ainda serão implementados. Não estão aprovados por
esta exportação nem pelos testes automatizados de reconstrução sintética.
