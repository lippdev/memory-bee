# 0005 — Descoberta explícita e seleção conservadora

Status: adotado na implementação da etapa 02, em 2026-09-24.

## Contexto

O leitor por arquivo não permite encontrar sessões por projeto nem escolher um
ramo. Nomes codificados de diretórios podem colidir ou ser personalizados. A
[documentação oficial](https://code.claude.com/docs/en/sessions#where-transcripts-are-stored)
confirma armazenamento JSONL e avisa que seu formato interno pode mudar.

## Decisão

Adicionar `sessions --root <projects-dir> --project <project-dir>`, sem varrer a
home implicitamente. Reutilizar o leitor limitado existente e comparar `cwd`
observado em registros de mensagem, nunca o nome do diretório ou o texto da conversa.
Normalizar caminhos absolutos lexicalmente, sem exigir que projetos históricos
existam nem resolver seus symlinks. Caminhos relativos de projeto fornecidos pelo
usuário são relativos ao diretório atual; `cwd` relativo na origem é inválido.

Listar separadamente arquivos principais e de subagentes no layout suportado.
Ausência/conflito de metadados, erros e limites geram resultado parcial com caminho
e código. Não inferir projeto de um arquivo vazio a partir de seus vizinhos.
Limitar entradas visitadas, arquivos e bytes, além dos limites do leitor. A busca
não é índice persistente e não adiciona dependências.

Adicionar `inspect <arquivo> --leaf <uuid>`: seguir ancestrais de uma ponta conhecida,
exigir UUID único, parent explícito, sessão identificada e identidade de agente
consistente. Recusar parentes ausentes, ciclos, referências futuras e mistura de
agentes/sessões. Manter diagnósticos do arquivo inteiro e contar exclusões da seleção.
O resultado é seleção de registros, não garantia de transcrição completa.

## Consequências

- Relatórios de inspeção ganham campos aditivos de metadados, registros e pontas.
  O contrato de pacote portátil permanece inalterado.
- Comparação lexical não reúne aliases/symlinks de projeto ou worktrees diferentes.
- Logs compactados com ancestrais ausentes continuam inspecionáveis, mas podem
  não ser selecionáveis. Expandir suporte exige evidência e testes específicos.
- Diretórios/arquivos symlink encontrados na busca são ignorados com diagnóstico;
  a raiz explícita pode ser symlink e é canonicalizada. Isso não é sandbox contra
  troca concorrente de caminhos. Usar uma árvore estável para reprodução.
- A descoberta lê arquivos de outros projetos na raiz para identificar `cwd`,
  mas sua saída não inclui texto das mensagens.
- Compatibilidade real continua não certificada. Separar validação com amostra
  controlada desta entrega evita confundir testes sintéticos com suporte por versão.
