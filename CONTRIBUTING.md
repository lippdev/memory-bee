# Desenvolvimento

## Autonomia e responsabilidade

O mantenedor autorizou o agente a planejar, implementar, validar, fazer commits, publicar branches, abrir e integrar PRs e preparar/publicar releases dentro do escopo acordado. Não é necessário pedir confirmação a cada etapa rotineira. Essa autorização não elimina controles de acesso da plataforma nem autoriza apagar trabalho alheio ou realizar mudanças destrutivas fora do escopo.

Trabalhamos com etapas de equipe, preservando a autoria real: planejamento, implementação, revisão e validação. Não inventar revisores, aprovações ou identidades. Uma revisão feita pelo próprio autor deve ser identificada como autorrevisão; revisão independente só pode ser registrada quando de fato ocorrer.

## Git

1. `main` deve permanecer utilizável. O primeiro commit pode estabelecer a documentação diretamente nela; depois disso, trabalhar em branches curtas, uma por mudança coerente.
2. Branches criadas por agentes usam `codex/<assunto>`. Não manter uma branch `develop` permanente.
3. Fazer commits pequenos e descritivos no formato `tipo: descrição`, usando `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci` ou `chore`. Nunca fabricar autoria com nomes de papéis de equipe.
4. Com remoto disponível, abrir PRs para integrar mudanças. Sem remoto, preservar o trabalho em branches e commits locais; não simular links ou números de PR.
5. Integrar por squash merge. O título do PR deve servir como mensagem clara do commit final. Preservar os registros de revisão e validação no PR.
6. Excluir a branch de trabalho após integração confirmada, quando ela não contiver trabalho adicional.
7. Não fazer force push em branches compartilhadas nem reescrever `main`. Preferir correções em novos commits e reversões explícitas. Antes de qualquer operação destrutiva, verificar se há trabalho de outras pessoas e se a ação está autorizada.

## Ciclo de uma mudança

O [ROADMAP.md](ROADMAP.md) define escopo e aceite; [docs/EXECUTION.md](docs/EXECUTION.md) registra progresso e retomada. Leia ambos antes de iniciar trabalho de produto. Mantenha o roadmap como referência canônica e sinalize divergências na apresentação HTML.

### Planejamento

Registrar problema, escopo e critérios de aceite em uma issue ou no corpo do PR. Enquanto não houver remoto, usar um documento local versionado para mudanças substanciais. Alterações pequenas não precisam de documentação burocrática. Decisões duradouras e suas razões vão em `docs/decisions/`.

Associar cada entrega à etapa do roadmap quando aplicável. O registro pode ficar em `docs/EXECUTION.md` enquanto não houver issue ou PR. Não avançar para uma etapa dependente com critérios essenciais pendentes sem registrar uma revisão explícita do plano.

### Implementação

Inspecionar o estado do Git antes de editar. Preservar alterações preexistentes. Evitar misturar mudanças sem relação e não adicionar dependências sem uma necessidade concreta.

### Revisão

Revisar o diff final, os critérios de aceite, a compatibilidade e o tratamento de falhas. Registrar achados e correções. Declarar se foi autorrevisão ou revisão independente. Não considerar uma revisão concluída apenas porque os testes passaram.

### Validação e integração

Executar verificações proporcionais à mudança e registrar comandos, resultados e limitações. Para documentação, conferir links locais, consistência e diff; para comportamento, testar resultados observáveis e casos de falha relevantes. Quando a stack existir, definir os comandos canônicos e automatizá-los no CI.

Integrar apenas quando os critérios de aceite estiverem atendidos, não houver achados bloqueantes e as verificações aplicáveis passarem. Não contornar falhas de CI para concluir um PR. Distinguir claramente verificações executadas de verificações indisponíveis.

Ao terminar, atualizar `docs/EXECUTION.md` com resultado, evidência de validação, tipo de revisão, limitações e próximo passo. Status permitidos: pendente, em andamento, bloqueado e concluído. Um bloqueio deve indicar o que falta e como destravá-lo; a conclusão exige evidência dos critérios de aceite.

## Comandos de validação

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked
```

Toolchain em `rust-toolchain.toml`; versionar mudanças de dependências no
`Cargo.lock`. CI em `.github/workflows/ci.yml` executa os mesmos comandos.
Usar apenas fixtures sintéticas em `testdata/` e testes temporários.
Para mudanças no exportador, instalar `scripts/requirements-validation.txt` em
venv e executar `python scripts/check_bundle.py` após o build; CI faz o mesmo.
Python é dependência de validação, não de execução/distribuição do produto.
Ensaios manuais estão separados em `docs/MANUAL_TESTS.md`; conforme orientação do
mantenedor, podem ocorrer depois sem impedir implementações testadas automaticamente.

## Registro do PR

O corpo deve conter problema e resultado, escopo, validação, revisão e limitações relevantes. Preferir uma explicação curta e concreta. Manter título e descrição atualizados com a implementação final.

### Revisão automática com Pullfrog

O Pullfrog revisa PRs novos e novos commits. O texto versionado em
[`.github/pullfrog/review.md`](.github/pullfrog/review.md) é a fonte das instruções
de revisão configuradas em `prompts.review` no repositório do Pullfrog. Após
alterá-lo e integrar o PR, sincronizar com:

```sh
npx --yes pullfrog@latest config set prompts.review --file .github/pullfrog/review.md --repo lippdev/memory-bee
npx --yes pullfrog@latest config get prompts.review --repo lippdev/memory-bee
```

O Pullfrog é uma revisão automatizada; seu parecer não substitui autorrevisão,
testes, ensaios humanos nem aprovação independente. O workflow gerado em
`.github/workflows/pullfrog.yml` é administrado pelo serviço; ajustar as
instruções pela configuração acima, sem editar os gatilhos no YAML.

## Dados e exportações

Não versionar conversas reais, credenciais ou pacotes de usuários. Usar fixtures sintéticas. Exportações devem ficar fora do repositório ou em `exports/`, ignorado pelo Git. Uma exportação de contexto não autoriza commit, push ou publicação do código do usuário.

## Releases

Releases são marcos deliberados, não consequência automática de cada merge. Quando houver um artefato distribuível, adotar versionamento semântico e tags `vX.Y.Z`, publicar notas com mudanças e limitações e validar a instalação. Versionar o formato do pacote separadamente quando ele for definido. A licença e o canal de distribuição ainda precisam ser escolhidos antes da primeira distribuição pública.
