# Pesquisa de integrações — 2026-09-24

## O que foi verificado

A documentação oficial do Claude Code descreve transcrições JSONL por projeto. Hooks expõem `transcript_path`; subagentes têm registros próprios. A CLI instalada respondeu `2.1.281` a `claude --version`. Não foram lidas conversas pessoais nem acionada inferência.

O parser do Orca, no commit `adc0c67f75dd44ee5b771faf3372e7dba3c73fa0`, observa `type`, `sessionId`, `timestamp` e `message.content`, com registros auxiliares. Isso é evidência de uma implementação de terceiro, não garantia de schema estável do fornecedor. As fixtures deste projeto foram escritas do zero como exemplos estruturais, não capturadas de uma sessão dessa versão.

## Matriz de investigação (não é matriz de suporte)

| Agente | Ler contexto | Iniciar destino | Cotas | Perfis isolados |
|---|---|---|---|---|
| Claude Code | JSONL documentado; primeiro leitor escolhido | Retomada nativa documentada; injeção de pacote ainda por validar | Não investigado; não inferir por contagem de tokens | Não comprovado; login global não basta |
| Codex | Inspeção experimental por arquivo explícito; fixtures sintéticas, compatibilidade real pendente | Integração da etapa 05 | Aguardar pesquisa da fonte | Não comprovado |
| Cursor | Parser identificado no Orca; CLI e IDE precisam ser distinguidas | Não investigado | Não investigado | Não comprovado |
| Pi | Parser identificado no Orca; não validado aqui | Não investigado | Dependente de provedor; não investigado | Não comprovado |
| Hermes | Parser identificado no Orca; não validado aqui | Não investigado | Dependente de provedor; não investigado | Não comprovado |
| Antigravity | Candidato futuro; formato não validado | Não investigado | Referência prometida ainda ausente | Não comprovado |

Uso e contas não bloqueiam M1. Antes de implementar essas capacidades, verificar interfaces do fornecedor, unidades, renovação, expiração, armazenamento seguro e isolamento real por processo. Ausência de dados deve ser explícita.

## Leitor Claude: contrato de comportamento a implementar

1. Abrir somente para leitura; primeiro aceitar arquivo explícito. Descoberta deve aceitar raiz configurável e não presumir que o nome codificado do diretório basta: conferir metadados do projeto.
2. Ler um snapshot limitado ao tamanho observado na abertura. Preservar ordem física e referência de linha/UUID; timestamps não reordenam eventos.
3. Interpretar texto de `user`/`assistant`, em string ou blocos. Separar chamadas e resultados de ferramentas dos pedidos humanos. Preservar parent UUID quando disponível.
4. Não interpretar ramificações/subagentes como uma única conversa linear. Identificar ambiguidade e exigir seleção de ramo antes de afirmar continuidade.
5. Registros desconhecidos, blocos não suportados e compaction devem gerar aviso com localização. Thinking, imagens e binários não entram no texto inicial; registrar omissão sem reproduzir payloads sensíveis.
6. Linha final incompleta: recuperar prefixo válido e informar perda. Linha inválida no meio: informar e não alegar histórico completo. Arquivo vazio: resultado vazio explícito, não conversa inventada.
7. Aplicar limites de arquivo/linha documentados na implementação; nunca cortar silenciosamente. Uma versão não testada deve ser marcada como tal, mesmo se registros conhecidos forem lidos.

## Referências e reaproveitamento

- [Claude sessions](https://code.claude.com/docs/en/sessions) e [hooks](https://code.claude.com/docs/en/hooks).
- [Parser Claude do Orca](https://github.com/stablyai/orca/blob/adc0c67f75dd44ee5b771faf3372e7dba3c73fa0/src/main/ai-vault/session-scanner-primary-parsers.ts).
- [Arquitetura ai-memory examinada](https://github.com/akitaonrails/ai-memory/blob/24ae5dd0b86041e29d25b6586adff5307d51cc00/docs/ARCHITECTURE.md).

Decisão: implementar o núcleo pequeno de forma independente. Não extrair o runtime/IDE do Orca nem exigir o servidor do ai-memory. Nenhum código copiado. Avaliar entrada/saída compatível com ai-memory na etapa 09; não chamar nosso pacote de OKF sem implementar e validar o contrato correspondente.

## Pesquisa Codex — 2026-09-24

Fontes, distinção entre App Server e transcrição instável e perfil testado em
[contrato Codex](../specs/codex-reader.md). A pesquisa não certifica formato real;
leitura controlada permanece no item 26 do roteiro manual.
