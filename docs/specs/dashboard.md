# Dashboard de terminal

Primeiro recorte, somente leitura. Lista sessões Claude/Codex de um projeto sob
raízes explícitas, mostra o detalhe de uma sessão e, com um pacote, a prévia de
`prepare-resume` e o resultado de `verify`. Nunca exporta, aplica ou lança um
agente. Decisão da biblioteca no [ADR 0016](../decisions/0016-tui-library.md);
identidade visual e cena de referência no
[ADR 0015](../decisions/0015-memory-bee-identity.md) e no protótipo
[Clareira da Colmeia](https://claude.ai/artifact/W5a8EAd4UKXLS84Nsqa8Ab). A cena
animada da colmeia não foi portada nesta entrega; o cabeçalho traz só um sinal
estático (`⬢`), que é a identidade ASCII opcional prevista no roadmap.

## Comando

```sh
memory-bee dashboard --project <project-dir> [--claude-root <dir>] [--codex-root <dir>] [--bundle <dir>] [--theme (dark|light)] [--no-color] [--once [--width <n>] [--height <n>]]
```

Exige `--project` e pelo menos uma de `--claude-root`/`--codex-root`; nenhuma
raiz é varrida implicitamente, como no resto da CLI. `--bundle` habilita a aba
Retomada (`r`) e a verificação (`v`); sem ela, essas teclas não fazem nada.
`--theme` aceita `dark` (padrão) ou `light`. `--no-color` ou a variável
`NO_COLOR` removem toda cor; os estados continuam legíveis pelos símbolos
`●`/`◐`/`▲`. `--width`/`--height` só são aceitos junto de `--once`.

Sem `--once`, o comando exige um terminal real: `stdout` que não seja um TTY
sai com 64 e sugere `--once`. Com `--once`, um quadro é composto sobre um
backend de teste e impresso como texto puro, sem cor — serve para
acessibilidade, automação e para os testes deste projeto.

## Teclas

| Tecla | Ação |
|---|---|
| `↑` `↓` / `j` `k` | mover a seleção na lista de sessões |
| `tab` | abrir/fechar o detalhe da sessão selecionada |
| `enter` | abrir o detalhe (equivalente a `tab` a partir da lista) |
| `1`–`9` | escolher uma ponta de ramo, quando a sessão exige seleção |
| `esc` | voltar para a lista de sessões |
| `s` | ir para a lista de sessões |
| `r` | prévia de `prepare-resume` (exige `--bundle`) |
| `t` | trocar o destino da retomada entre `claude` e `codex` |
| `v` | rodar `receive::verify` sobre o pacote (exige `--bundle`) |
| `?` | tela de atalhos |
| `q` | sair; o terminal é restaurado |

## Dados e núcleo reutilizado

A dashboard só chama funções já usadas pela CLI, sem lógica própria de leitura:
`discovery::discover` e `codex_discovery::discover` para a lista; `claude::inspect`,
`codex::inspect` e `selection::select` para o detalhe; `resume::prepare` (sem
arquivo de prompt, somente leitura) para a prévia de retomada; `receive::verify`
para a verificação. Nenhuma escreve ou executa algo.

## Detalhe e seleção de ramo

Uma sessão Claude com mais de uma ponta (`requires_branch_selection`) mostra as
pontas numeradas no painel de detalhe; `1`–`9` escolhe uma e reaplica
`selection::select` só para exibição, sem gravar nada. Sessões Codex não têm
ramos. O "primeiro pedido" mostrado no detalhe segue a mesma convenção do
HANDOFF (`bundle.rs`): o primeiro evento retido com `role == "user"` e
`kind == "text"`.

## Degradação

- Abaixo de 80 colunas, um painel por vez (lista ou detalhe, alternando por
  `tab`/`enter`/`esc`), como no protótipo aprovado.
- Abaixo de 30 colunas ou 8 linhas, só uma mensagem de terminal pequeno.
- Toda linha da lista reserva a contagem de eventos, de diagnósticos e o
  símbolo de estado; só o rótulo da sessão é truncado com `…` quando falta
  espaço, para nunca cortar essas informações essenciais (critério de aceite
  da etapa 06).
- O cabeçalho só mostra o caminho do projeto à direita quando cabe sem
  sobrepor as abas; do contrário, omite o caminho em vez de misturar os dois
  textos.

## Códigos de saída

Iguais ao padrão do resto da CLI: `0` sucesso; `2` alguma descoberta foi
parcial (diagnósticos a conferir); `1` erro de E/S, por exemplo uma raiz que
não existe; `64` uso inválido. Uma sessão interativa que termina com `q` sai
com `0` ou `2`, nunca aborta sem restaurar o terminal — inclusive em pânico,
coberto por um hook que desfaz o modo bruto e a tela alternada antes de
repassar o pânico.

## Limitações

Não exporta, aplica nem lança um agente pela TUI; esses fluxos continuam só na
CLI (`export`, `apply`, `prepare-resume --launch`). A cena animada da colmeia
(abelha, favo, ambiente) do protótipo aprovado não foi portada — o visual atual
é funcional, não decorativo. Compatibilidade real de terminal (largura mínima
de fato usável, emuladores sem cor de 24 bits) não foi validada manualmente;
roteiro em [MANUAL_TESTS.md](../MANUAL_TESTS.md).
