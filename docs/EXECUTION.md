# Estado de execução

Última atualização: 2026-09-24. Este arquivo é o ponto de retomada entre agentes. Não substitui a inspeção do Git nem os critérios do [roadmap](../ROADMAP.md).

## Situação atual

- Existe documentação de produto, workflow e um HTML de planejamento.
- Não existe CLI, TUI, leitor de sessões ou integração de contas implementada.
- Stack, primeiro SO, nome definitivo e licença ainda não foram escolhidos.
- Não há comandos de build, lint ou testes de aplicação definidos.
- Na última inspeção, não havia remoto configurado. Conferir novamente antes de operações de publicação.
- A documentação está em branches locais; não presumir que já foi integrada à `main`.

## Próxima tarefa de produto

**Etapa 01 — Pesquisa e contrato do produto.** Quando solicitado a iniciar implementação, começar por uma avaliação curta de stack e do primeiro leitor, com os seguintes resultados:

1. Comparar opções de stack quanto a distribuição, TUI, leitura de registros e armazenamento de credenciais. Registrar recomendação e tradeoffs em ADR.
2. Validar o formato de sessões da versão escolhida de Claude Code ou justificar outra primeira origem, usando fixtures sintéticas.
3. Propor o contrato versionado do pacote: Markdown de entrada, manifesto, histórico e referência ao código, com omissões explícitas.
4. Definir uma tarefa sintética para demonstrar M1 em outro checkout.
5. Mapear incertezas de uso e contas separadamente; elas não bloqueiam o leitor nem a exportação.

O repositório de referência sobre consumo de contas ainda aguarda envio do mantenedor. Nenhuma stack ou API de autenticação deve ser tratada como escolhida apenas por ter aparecido em um exemplo.

## Status do roadmap

| Etapa | Prioridade | Status | Evidência / pendência |
|---|---|---|---|
| 01 Pesquisa e contrato | P0 | Pendente | Referências consultadas; avaliação técnica e contratos ainda ausentes. |
| 02 Leitor de sessão | P0 | Pendente | Depende de 01. |
| 03 Exportação revisável | P0 | Pendente | Depende de 02. |
| 04 Estado do código | P0 | Pendente | Depende de 03; fecha M1. |
| 05 Troca de agente | P1 | Pendente | Depende de 03–04. |
| 06 Dashboard terminal | P1 | Pendente | Depende de 05. HTML existente é planejamento, não implementação. |
| 07 Uso e alertas | P1 | Pendente | Depende de 01 e 06; falta referência de consumo. |
| 08 Perfis de conta | P1 | Pendente | Depende de 05–06 e prova de isolamento. |
| 09 Captura contínua | P2 | Pendente | Depende de 02–06. |
| 10 Novos adaptadores | P2 | Pendente | Depende dos contratos e testes relevantes. |
| 11 Distribuição | P2 | Pendente | Pode acompanhar M1 para alpha; nome e licença em aberto. |

## Registro de entregas documentais

### Bootstrap e plano visual

- `50d75f8`: escopo inicial e workflow em Markdown.
- `5632542`: HTML com prioridades, dependências e critérios de aceite.
- Verificações registradas: links locais, diff e sintaxe JavaScript do HTML. Aparência não validada em navegador.
- Revisão: autorrevisão; nenhuma revisão independente registrada.

### Guia operacional em Markdown

- Escopo: converter as 11 etapas do HTML para `ROADMAP.md`, conectar os pontos de entrada e registrar o estado de retomada.
- Resultado: regras mantidas em `CONTRIBUTING.md`, leitura inicial em `AGENTS.md` e roadmap canônico em Markdown; nenhuma funcionalidade de produto implementada.
- Validação: conferência dos 11 títulos, prioridades, dependências, entregas e critérios contra o HTML; links locais e `git diff --check`.
- Revisão: autorrevisão documental. Sem testes de aplicação, pois não há aplicação nesta entrega.
- Próximo passo: avaliação técnica descrita acima, quando o mantenedor solicitar continuidade do produto.

## Como manter este arquivo

Ao começar, registrar a entrega ativa e seus critérios. Ao encerrar, atualizar a linha da etapa e acrescentar um registro curto com resultado, verificações realmente executadas, tipo de revisão e pendências. Referenciar PR/commit quando existir, sem inventar identificadores. Não manter planos pessoais paralelos que o próximo agente não consiga consultar.
