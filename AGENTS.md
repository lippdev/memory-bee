# Instruções para agentes

Leia `README.md` e `CONTRIBUTING.md` antes de trabalhar. O projeto é uma CLI para portabilidade de contexto entre agentes e pessoas; não criar uma IDE como requisito de uso.

## Forma de trabalhar

- Atue com as etapas de uma equipe: planejamento, implementação, revisão e validação, com registros proporcionais à mudança.
- Preserve autoria real. Não invente integrantes, aprovações ou revisões independentes. Identifique autorrevisões.
- Há autonomia para commits, push, PRs, merge e releases dentro do escopo acordado, respeitando verificações, permissões da plataforma e trabalho preexistente.
- Após o commit inicial de documentação, use branches `codex/<assunto>` e PRs quando houver remoto. Integre por squash; não reescreva branches compartilhadas.
- Registre decisões duradouras em `docs/decisions/`. Não escolha uma stack ou amplie o escopo sem apresentar a justificativa.
- Relate o que foi alterado, como foi validado e limitações reais. Não declare CI, testes, PRs ou publicação como concluídos sem evidência.

## Invariantes do produto

- Exportação básica sem chamadas a modelos, inclusive quando a origem não consegue mais responder.
- Pacote legível por pessoas e agentes sem exigir instalação do Memory Share no destino.
- Distinguir registros extraídos de sínteses ou inferências; informar conteúdo omitido ou indisponível.
- Referenciar o estado do código e tratar alterações locais explicitamente. Não publicar código como efeito implícito de exportar contexto.
- Não versionar dados reais de conversas ou segredos. Usar fixtures sintéticas.

## Validação atual

Não há stack, build ou testes definidos ainda. Para mudanças documentais, verificar links relativos e executar `git diff --check`. Atualize esta seção quando os comandos reais do projeto existirem.
