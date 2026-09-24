# 0001 — Fluxo de desenvolvimento com autonomia e rastreabilidade

Status: aceito pelo mantenedor em conversa; detalhes operacionais registrados no bootstrap.

## Contexto

O repositório começa sem commits ou remoto. O mantenedor autorizou autonomia para todo o ciclo de desenvolvimento e pediu uma atuação organizada como equipe para melhorar o histórico.

## Decisão

Usar `main` estável, branches curtas e PRs com squash merge. Permitir um primeiro commit documental em `main`. Enquanto não houver remoto, trabalhar com registros e branches locais.

Separar planejamento, implementação, revisão e validação nos registros do trabalho, sem inventar pessoas ou aprovações. A revisão pode ser uma autorrevisão explicitamente identificada; independência deve corresponder a uma revisão realmente realizada.

Registrar no PR a intenção, os critérios de aceite, as verificações e achados relevantes. Usar decisões versionadas para escolhas duradouras. Evitar documentação extra para mudanças triviais.

## Consequências

O histórico da `main` descreve mudanças completas; detalhes da execução ficam nos PRs. A autonomia permite concluir o ciclo sem pedidos repetidos de permissão, mas não dispensa validação ou controles da plataforma. Proteções de branch e CI só poderão ser configurados após existir um remoto e uma stack.
