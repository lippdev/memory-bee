# Cenário sintético de aceite M1

Tarefa: adicionar ordenação decrescente a uma função de listagem de tarefas. O primeiro dev altera a função, mas deixa a validação de entrada e um teste pendentes. O histórico informa o pedido e o ponto de parada; não presume que a afirmação do agente corresponde ao código.

## Preparação futura do teste de integração

1. Criar repositório Git temporário com função e teste de ordenação crescente. Capturar o hash real de um commit base gerado durante o teste; não fixar um hash fictício no pacote de demonstração funcional.
2. Criar uma mudança rastreada e um arquivo novo de teste ainda não versionado, mais um arquivo não selecionado que não deve sair da origem.
3. Usar a fixture Claude sintética e exportar um pacote revisado contendo somente as alterações selecionadas.
4. Clonar o commit base em outro diretório e ler `HANDOFF.md` sem instalar Memory Pier. Conferir manifesto e aplicar explicitamente as mudanças; terminar a validação de entrada e executar os testes.
5. Repetir com base divergente, destino modificado e arquivo novo já existente: nenhuma sobrescrita silenciosa.

## Critérios

Origem preservada; zero chamadas a modelos na exportação; pedido identificável; novo arquivo selecionado presente; arquivo excluído ausente; omissões declaradas; testes finais passando. Nenhum commit/push automático no projeto de teste. A leitura de uma fixture não equivale a cumprir M1.

Nesta etapa 01 foi entregue o cenário e exemplos estruturais. Exportação, aplicação e demonstração fim a fim continuam pendentes nas etapas 02–04.
