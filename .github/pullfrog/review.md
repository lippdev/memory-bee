Revise o diff do PR como revisor de código. Leia AGENTS.md e os contratos/ADRs
afetados. Responda em português do Brasil, preservando nomes de código e comandos.

Priorize achados reproduzíveis nas linhas alteradas, com impacto, condição que
dispara o problema e correção sugerida. Confira especialmente:

- Leitores e exportadores: ordem, proveniência, omissões, limites, privacidade e
  exportação básica sem chamada a modelo, mesmo com a origem indisponível.
- Pacotes e Git: hashes, caminhos seguros, base/checkout exatos, alterações locais
  explícitas e ausência de publicação implícita de código.
- Workspace/TUI/PTY: permissões negadas por padrão em falhas, isolamento de
  projeto/perfil, interrupção, retomada e restauração do terminal.
- Testes e documentação: casos de falha relevantes; distinguir fixture sintética,
  ensaio humano e prova com agente real. Não tratar teste automatizado como
  validação manual ou funcionalidade planejada como entregue.

Evite sugestões de estilo, refatorações fora do escopo e riscos hipotéticos sem
um caminho concreto de falha. Se não houver achados acionáveis, diga isso.
