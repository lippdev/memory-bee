# Exportação Codex — perfil experimental 1

`export-codex <rollout.jsonl>` adapta o [relatório Codex](codex-reader.md) ao núcleo
local de [exportação revisável](context-export.md). Sem modelos, descoberta de
arquivos, lançamento de agentes ou alteração da origem. Decisão no [ADR 0011](../decisions/0011-codex-context-export.md).

## Uso e seleção

```sh
cargo run --locked -- export-codex testdata/codex/basic.jsonl --preview
mkdir -p exports
cargo run --locked -- export-codex testdata/codex/basic.jsonl --output exports/codex-context
cargo run --locked -- export-codex testdata/codex/basic.jsonl --exclude-line 3 --preview
```

Exige exatamente `--preview` ou `--output <pasta-nova>`. `--exclude-line <n>` pode
repetir: exclui todos os blocos daquela linha original, que deve conter eventos.
Linhas apenas auxiliares/metadados não são exclusões válidas. Excluir tudo ou
exportar arquivo sem eventos é erro. Diagnósticos da origem continuam no pacote,
inclusive após excluir mensagens. Não há `--leaf`: Codex não é árvore Claude.

`--project <pasta>` observa referência Git explícita, sem inferir associação a
partir de cwd. `--include-path <arquivo-relativo>` repetível exige projeto e inclui
código selecionado conforme [v2](bundle-v2.md). Sem seletores, usa [v1](bundle-v1.md).
Limites, detecção de segredos no código, falhas de Git e aplicação são os mesmos
contratos existentes. Nenhum push, commit ou aplicação acontece na exportação.

## Histórico e integridade

`source.agent` no manifesto é `codex`. Session ID é o único ID conhecido entre os
eventos retidos ou null; mais de uma sessão conhecida é erro. Eventos com identidade
indisponível permanecem null, mesmo quando o manifesto tem uma sessão conhecida.
Versão é o único valor observado ou null quando ausente/múltiplo; não certifica
compatibilidade e não é inferida do programa instalado.

Cada evento mantém todos os campos do leitor, incluindo papel developer, phase,
tool_namespace, tipo do envelope/item, linha/bloco, IDs de sessão/turno e
proveniência. Só sequence é renumerada depois das exclusões. Não fabricar parent_id,
deduplicar texto ou reproduzir replacement_history. Caminho do arquivo e metadados
cwd não são copiados; caminhos presentes no texto histórico permanecem conteúdo.

HANDOFF informa perfil experimental, compatibility unverified e ordem física,
sem prometer reconstrução da conversa ativa após forks, rollback ou compactação.
Primeiro pedido humano retido e último evento são trechos extraídos, não síntese.
Checkpoints e instruções developer/system não viram pedidos humanos. Limites de
entrada Markdown, cercas e listas são os mesmos do exportador Claude.

A prévia contém os payloads preparados, incluindo texto potencialmente sensível.
A detecção examina todas as strings efetivamente exportadas dos eventos e os
metadados do manifesto. Fase, namespace e IDs herdados também são inspecionados.
Findings mostram localização/campo, sem ecoar valores. Exclusões não removem IDs
herdados de outras linhas; pode ser necessária uma cópia sanitizada da origem.

A gravação usa os mesmos bytes/hash da preparação, exige pasta nova e mantém
`redaction: pending-review`. `verify` aceita os pacotes v1/v2 e confere integridade;
não certifica completude, compatibilidade do leitor ou veracidade da conversa.
Código incluído pode ser conferido/aplicado via [apply explícito](receive-apply.md).

## Saídas e pendências

0: prévia/pacote sem perdas detectadas; 2: leitura/código parcial com pacote útil;
3: possíveis segredos (prévia disponível, escrita bloqueada); 1: erro de leitura,
seleção ou escrita; 64: argumentos inválidos. Compatibility unverified sozinha não
força saída 2. Limites de leitura e snapshot não atômico são os do leitor.

Compatibilidade real, revisão humana e retomada de M1 continuam pendentes; fixtures
sintéticas não substituem esses ensaios. Veja os itens 27–30 do
[roteiro manual](../MANUAL_TESTS.md). Descoberta explícita está em [sessions-codex](codex-discovery.md); lançamento
continua pendente.
