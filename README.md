# Memory Pier

Continue um trabalho de programação em outro agente ou entregue o contexto a outra pessoa, sem depender de uma nova resposta da IA de origem.

## Proposta

Memory Pier será uma ferramenta de terminal independente de IDE. Seu núcleo lê registros disponíveis de sessões, organiza um pacote local e prepara a retomada. Skills e plugins poderão facilitar seu uso, mas não serão necessários para exportar.

A visão inclui uma dashboard em terminal para sessões, uso disponível e perfis de contas, conforme suporte de cada agente. O núcleo de exportação vem primeiro; cotas e autenticação dependem de pesquisa técnica.

O destinatário deve conseguir usar o pacote sem instalar Memory Pier: um Markdown legível será a porta de entrada, acompanhado do histórico selecionado e das referências necessárias.

## Primeiro marco

Exportar uma sessão de um agente para um pacote que outro desenvolvedor consiga usar no mesmo projeto. A primeira origem e a stack ainda serão escolhidas após uma avaliação técnica curta.

O pacote deve identificar o pedido original, preservar a proveniência das mensagens, referenciar repositório/branch/commit e informar omissões. Alterações locais selecionadas poderão acompanhar o contexto; a exportação não deve publicar código automaticamente.

A exportação básica funciona sem chamadas a modelos. Sínteses semânticas dependem de checkpoints existentes ou de uma etapa opcional com IA. Abrir uma sessão nova não renova cotas de uso do provedor.

## Estado

Planejamento inicial; ainda não há CLI implementada. Exemplos de comandos discutidos são propostas, não interfaces disponíveis.

## Desenvolvimento

- Comece por [AGENTS.md](AGENTS.md), mesmo usando uma ferramenta que não carregue esse arquivo automaticamente.
- [Roadmap canônico em Markdown](ROADMAP.md)
- [Estado de execução e próxima tarefa](docs/EXECUTION.md)
- [Plano visual: prioridades e ordem de implementação](docs/roadmap/index.html)
- [Fluxo de contribuição](CONTRIBUTING.md)
- [Instruções para agentes](AGENTS.md)
- [Decisões de arquitetura e processo](docs/decisions/0001-development-workflow.md)

O Orca é uma referência de pesquisa para captura de contexto e leitores de sessões: <https://github.com/stablyai/orca>. Nenhum código dele foi incorporado nesta etapa.
