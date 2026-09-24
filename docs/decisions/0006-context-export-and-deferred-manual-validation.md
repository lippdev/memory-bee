# 0006 — Exportação de contexto e validação manual posterior

Status: adotado em 2026-09-24, conforme orientação atual do mantenedor.

## Mudança de sequência

O mantenedor pediu continuidade das implementações e fará os testes manuais
posteriormente, um por um. A validação real de Claude Code permanece pendente,
mas deixa de bloquear código das etapas seguintes. Testes automatizados, revisão
e CI continuam obrigatórios. Não interpretar adiamento como certificação nem
como autorização para publicar conversas reais ou chamar modelos para validá-las.

## Recorte e decisões

Implementar a etapa 03 como exportação local somente de contexto, usando o leitor
e a seleção existentes. Produzir pasta com HANDOFF, manifesto v1 e histórico
normalizado. Estado do código é `unknown`, com campos Git null; obter referências
Git de verdade é a próxima entrega da etapa 04. Nenhum comando Git é executado
pelo exportador e não há push do projeto do usuário.

Prévia JSON mostra os mesmos payloads preparados que a biblioteca gravaria.
Exclusão opera por linha original e remove todos os blocos dessa linha. Ramo único
verificável pode ser escolhido automaticamente; ambiguidade exige `--leaf`.
Perdas da origem permanecem no manifesto e no Markdown; saída parcial é utilizável
e explicitamente sinalizada. Não criar pacote vazio nem inventar pedido inicial.

Entrada Markdown limita trechos e remete ao histórico completo selecionado.
Conteúdo histórico é colocado em cercas de código, com comprimento que não pode
ser fechado pelo texto selecionado. Texto não é interpretado como instrução.
Metadados de caminhos locais não são copiados automaticamente; caminhos que
aparecem no texto da conversa continuam conteúdo sujeito a revisão.

Usar `sha2` para SHA-256 e `regex` para um conjunto pequeno de padrões de possíveis
segredos. Isso evita implementar criptografia própria e permite padrões testáveis.
Detecção inspeciona campos efetivamente exportados e bloqueia gravação ao encontrar
suspeitas, sem prometer cobertura completa. Exclusões são aplicadas antes da busca.
O manifesto permanece `pending-review`; o programa não declara revisão humana.

Destino deve ser pasta nova, em pai existente. Criar com exclusividade e permissões
restritas em Unix; arquivos têm nomes fixos e abertura `create_new`. Manifesto é
gravado por último. Erros normais removem apenas arquivos criados pela operação;
interrupção abrupta pode deixar pasta incompleta. Sem substituição de destino,
sem merge de diretórios e sem promessa de transação atômica contra falha elétrica
ou alteração concorrente maliciosa.

## Validação e limites

Testes sintéticos cobrem preservação, proveniência, seleção, exclusões, detecção,
colisões, escrita parcial e retorno da CLI. Python/jsonschema é ferramenta de
validação do desenvolvimento/CI, sem requisito Python no binário ou no destino.
Schema e SHA-256 também são conferidos independentemente dos testes Rust.

Compatibilidade real, revisão manual dos pacotes e teste de retomada por outra
pessoa permanecem no roteiro manual. Estado do código, patches e arquivos novos
não fazem parte deste recorte. Próxima implementação: referência Git verificável.
