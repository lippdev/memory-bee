# Memory Share

Continue um trabalho de programação em outro agente ou entregue o contexto a outra pessoa, sem depender de uma nova resposta da IA de origem.

## Proposta

Memory Share será uma ferramenta de terminal independente de IDE. Seu núcleo lê registros disponíveis de sessões, organiza um pacote local e prepara a retomada. Skills e plugins poderão facilitar seu uso, mas não serão necessários para exportar.

O destinatário deve conseguir usar o pacote sem instalar Memory Share: um Markdown legível será a porta de entrada, acompanhado do histórico selecionado e das referências necessárias.

## Primeiro marco

Exportar uma sessão de um agente para um pacote que outro desenvolvedor consiga usar no mesmo projeto. A primeira origem e a stack ainda serão escolhidas após uma avaliação técnica curta.

O pacote deve identificar o pedido original, preservar a proveniência das mensagens, referenciar repositório/branch/commit e informar omissões. Alterações locais selecionadas poderão acompanhar o contexto; a exportação não deve publicar código automaticamente.

A exportação básica funciona sem chamadas a modelos. Sínteses semânticas dependem de checkpoints existentes ou de uma etapa opcional com IA. Abrir uma sessão nova não renova cotas de uso do provedor.

## Estado

Planejamento inicial; ainda não há CLI implementada. Exemplos de comandos discutidos são propostas, não interfaces disponíveis.

## Desenvolvimento

- [Fluxo de contribuição](CONTRIBUTING.md)
- [Instruções para agentes](AGENTS.md)
- [Decisões de arquitetura e processo](docs/decisions/0001-development-workflow.md)

O Orca é uma referência de pesquisa para captura de contexto e leitores de sessões: <https://github.com/stablyai/orca>. Nenhum código dele foi incorporado nesta etapa.
