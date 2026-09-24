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
