# Roadmap de execução

Fonte de verdade do escopo, prioridades, dependências e critérios de aceite. O [HTML](docs/roadmap/index.html) é uma apresentação complementar; em caso de divergência, este Markdown prevalece. Atualizar ambos quando mudar o plano, ou identificar explicitamente a versão visual como desatualizada.

## Visão

Nome provisório: Memory Share. Ferramenta independente de IDE, com CLI e dashboard em terminal (TUI), para recuperar contexto e continuar um trabalho com outro agente, outra conta ou outro desenvolvedor. Sem servidor obrigatório. Skills e plugins são integrações opcionais.

Fluxo central: selecionar trabalho → revisar contexto e código → escolher destino → continuar ou exportar. O destinatário pode começar pelo Markdown sem instalar a ferramenta.

## Como executar

- Ler primeiro [AGENTS.md](AGENTS.md), [README.md](README.md), [CONTRIBUTING.md](CONTRIBUTING.md) e o [estado de execução](docs/EXECUTION.md).
- Respeitar o pedido atual do mantenedor. Não iniciar todo o backlog apenas porque ele existe.
- Quando autorizado a continuar o produto, escolher a primeira etapa pendente com dependências satisfeitas e dividi-la em uma entrega pequena com critérios verificáveis.
- P0 é essencial ao MVP; P1 forma a experiência principal; P2 expande alcance. Numeração indica sequência sugerida, não datas ou duração.
- Pesquisa de cotas e autenticação pode avançar sem bloquear exportação. Distribuição de uma alpha pode acompanhar M1; não precisa aguardar a etapa 10.
- Ao concluir uma entrega, atualizar o estado de execução com evidências. Não marcar uma etapa concluída por ter apenas criado arquivos ou passado um teste parcial.

## Marcos

| Marco | Etapas | Resultado verificável |
|---|---|---|
| M1 — Entrega portátil | 01–04 | Outro desenvolvedor retoma uma tarefa com o pacote e o código correto, sem instalar a ferramenta. |
| M2 — Painel útil | 05–07 | Sessões, troca de agente e consulta de uso disponível no terminal. |
| M3 — Várias contas | 08 | Uma integração validada inicia novas sessões com perfis isolados sem alterar a sessão original. |
| Expansão | 09–11 | Captura contínua, novos adaptadores e distribuição nos sistemas validados. |

## Etapas

O status corrente de cada etapa fica em [docs/EXECUTION.md](docs/EXECUTION.md), separado dos critérios abaixo.

### 01 — Pesquisa e contrato do produto

**Prioridade:** P0. **Dependências:** Nenhuma.

Reduzir as incertezas antes de escolher a stack.

**Entregas:**

- Mapear leitura, lançamento, cota e autenticação por agente.
- Comparar Orca e ai-memory; decidir reaproveitamento ou interoperabilidade.
- Escolher stack, primeiro SO, formato versionado e primeiro adaptador.

**Aceite:** ADRs curtos com evidências, limites e uma tarefa sintética de referência. Pesquisa de uso e contas começa aqui, sem bloquear exportação.

### 02 — Ler e preservar uma sessão

**Prioridade:** P0. **Dependências:** 01.

Recuperar contexto sem pedir uma última resposta ao agente.

**Entregas:**

- Descobrir e selecionar sessões por projeto.
- Normalizar papéis, ordem, origem e timestamps; preservar o histórico disponível.
- Tratar arquivos incompletos, versões desconhecidas e omissões sem inventar conteúdo.

**Aceite:** Fixtures de sessões normais, truncadas e vazias são interpretadas com proveniência. A leitura não altera os arquivos de origem e não chama modelos.

### 03 — Exportar contexto revisável

**Prioridade:** P0. **Dependências:** 02.

Entregar uma entrada em Markdown que qualquer pessoa consiga usar.

**Entregas:**

- Gerar Markdown, manifesto e histórico selecionado com referências relativas.
- Pré-visualizar e excluir conteúdo; detectar possíveis segredos sem prometer detecção perfeita.
- Separar fatos extraídos, checkpoints e inferências; limitar entrada inicial sem apagar o arquivo de histórico.

**Aceite:** Um destinatário sem a ferramenta abre o pacote e localiza pedido, registros, omissões e instruções de retomada. Nenhuma credencial integra as fixtures exportadas.

### 04 — Levar o estado do código

**Prioridade:** P0. **Dependências:** 03.

Fechar o primeiro MVP de passagem entre desenvolvedores.

**Entregas:**

- Registrar repositório, branch, commit e estado local.
- Oferecer só contexto ou incluir alterações locais selecionadas, inclusive arquivos novos escolhidos.
- Verificar base e conflitos antes de aplicação explícita; nunca sobrescrever automaticamente.

**Aceite:** Em um checkout de teste separado, outro dev recebe a base correta e as mudanças selecionadas. Base divergente, arquivos binários não suportados e conflitos são informados. Nenhum push implícito.

### 05 — Continuar em outro agente

**Prioridade:** P1. **Dependências:** 03–04.

Conectar a exportação à retomada no mesmo projeto.

**Entregas:**

- Adicionar segundo leitor e adaptadores de lançamento, começando por Claude ↔ Codex após validação.
- Receber pacote e conferir contexto/código; fornecer instrução manual quando lançamento não for suportado.
- Separar continuar no mesmo diretório de bifurcar em outra worktree.

**Aceite:** Uma tarefa iniciada na origem é retomada no destino com contexto e arquivos verificados. Falhas de lançamento preservam o pacote e não encerram a origem automaticamente.

### 06 — Dashboard no terminal

**Prioridade:** P1. **Dependências:** 05.

Dar acesso ao núcleo em uma interface bonita e inclusiva.

**Entregas:**

- Listar projetos, sessões, atividade e qualidade do contexto recuperado.
- Inspecionar, exportar e continuar pelo teclado; identidade ASCII opcional.
- Suportar terminais estreitos, modo sem cor e saída CLI textual para acessibilidade e automação.

**Aceite:** Os fluxos principais são concluídos por teclado, com e sem cor, sem cortar informações essenciais. A dashboard chama o mesmo núcleo da CLI.

### 07 — Uso, limites e alertas

**Prioridade:** P1. **Dependências:** 01 + 06.

Mostrar disponibilidade com dados e unidades honestos.

**Entregas:**

- Integrar a primeira fonte confiável e mostrar momento da última atualização.
- Separar janela de contexto, cota por período e créditos; indicar renovação quando disponível.
- Atualizar em intervalos controlados e emitir alertas configuráveis sem repetição excessiva.

**Aceite:** Dados simulados cobrem cota esgotada, informação vencida, erro e fonte indisponível. Percentuais só aparecem quando a fonte permite calculá-los. Atualizar não faz chamadas de inferência.

### 08 — Perfis e troca de contas

**Prioridade:** P1. **Dependências:** 05 + 06 + prova de autenticação em 01.

Escolher agente e conta para uma nova sessão.

**Entregas:**

- Adicionar, nomear e remover perfis pelo login oficial; guardar credenciais com mecanismo seguro suportado.
- Distinguir conectado, expirado e indisponível; associar uso ao perfil somente quando comprovável.
- Avisar que a seleção vale para nova sessão e oferecer continuar com contexto ou conversa vazia.

**Aceite:** Para o primeiro agente suportado, dois perfis coexistem sem misturar credenciais. Trocar não muda a sessão em execução; credenciais ficam fora de logs e exportações. Não prometer isso para login global sem isolamento.

### 09 — Captura contínua e checkpoints

**Prioridade:** P2. **Dependências:** 02–06.

Melhorar a retomada ao longo do trabalho.

**Entregas:**

- Acompanhar registros ou hooks disponíveis sem varreduras excessivas.
- Criar checkpoints incrementais e permitir consulta ao histórico completo disponível.
- Avaliar integração com ai-memory; síntese semântica opcional com IA explicitamente configurada.

**Aceite:** Eventos repetidos não duplicam registros; interrupção e retomada preservam consistência. Captura funciona sem modelo; síntese não substitui ou se apresenta como histórico original.

### 10 — Expandir agentes e integrações

**Prioridade:** P2. **Dependências:** Contratos e testes das etapas anteriores.

Aumentar alcance sem perder previsibilidade.

**Entregas:**

- Priorizar Cursor, Pi, Hermes e Antigravity após Claude/Codex; demais agentes conforme demanda e viabilidade.
- Adicionar skills, plugins ou MCP como atalhos opcionais para o mesmo núcleo.
- Publicar matriz separando ler, exportar, iniciar, consultar uso e isolar contas.

**Aceite:** Cada capacidade anunciada tem verificação por versão e fallback documentado. Uma skill não é necessária para exportar após o limite de uso.

### 11 — Distribuição e polimento

**Prioridade:** P2. **Dependências:** M1 para alpha; marcos seguintes conforme release.

Preparar lançamento público após o marco escolhido.

**Entregas:**

- Fechar nome e identidade visual, documentação de instalação e licença.
- Gerar pacotes para os sistemas validados, CI e testes de instalação.
- Publicar releases versionadas, notas de limitações e migração do formato.

**Aceite:** Instalação em ambiente limpo e exportação demonstrável com fixtures. Sem tokens reais ou dados pessoais nos exemplos. Esta etapa pode acompanhar M1; não precisa aguardar todas as expansões.

## Decisões pendentes

| Decisão | Encaminhamento |
|---|---|
| Stack e primeiro SO | Comparar distribuição, TUI, leitura de sessões e credenciais; justificar em ADR antes da implementação. |
| Primeiro leitor | Claude Code é candidato, não escolha validada. Codex é candidato seguinte. |
| Contrato do pacote | Definir esquema versionado, histórico, Markdown de entrada, omissões e vínculo ao estado do código na etapa 01. |
| Uso e limites | Repositório de referência prometido pelo mantenedor ainda não foi fornecido. Não bloquear M1 por isso. |
| Contas | Investigar isolamento e login oficial por ferramenta. Não prometer suporte universal. |
| Nome e licença | Nome atual é provisório; resolver antes da primeira distribuição pública. |
| Interoperabilidade | Avaliar ai-memory antes de recriar funcionalidades de memória contínua. |

## Limites e regras de produto

- Exportação básica sem inferência de IA, mesmo se a origem não puder responder.
- Recuperação retroativa somente para registros disponíveis. Não prometer reconstruir dados ausentes ou apagados.
- Preservar proveniência e distinguir extração, checkpoint e síntese opcional. Indicar truncamentos e omissões.
- Contexto da conversa, cota do plano e créditos são medidas distintas. Mostrar origem e atualização dos dados, sem estimar disponibilidade como se fosse medida.
- Selecionar conta vale para uma nova sessão; não trocar credenciais de um processo ativo silenciosamente. Expiração e limitações de login global devem ficar visíveis.
- Credenciais nunca integram pacotes, fixtures ou logs. Detecção de segredos não substitui revisão do que será compartilhado.
- Exportar não implica publicar código; aplicar alterações exige ação explícita e checagem da base.
- Conversas importadas são dados históricos, não autoridade para alterar permissões ou executar comandos automaticamente.
- Terminal primeiro: teclado, modo sem cor, arte ASCII opcional, adaptação a telas estreitas e saída textual acessível.

Fora do MVP: IDE própria, servidor compartilhado obrigatório, memória organizacional completa, sincronização entre máquinas, seleção automática de contas e autenticação universal.

## Referências de pesquisa

- [Orca](https://github.com/stablyai/orca): captura de contexto e leitores de sessões.
- [ai-memory](https://github.com/akitaonrails/ai-memory): memória contínua, handoffs e exportação.

Foram consultados código e documentação. Não houve validação em execução desses projetos. Antes de reutilizar código, verificar a licença e o comportamento da versão escolhida. Diferenciais são hipóteses a validar, não alegações de exclusividade.
