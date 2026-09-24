# Recebimento e aplicação local

Implementado para pastas v1/v2 no perfil emitido pelo Memory Pier. Sem rede,
modelos, execução de histórico, filtros ou hooks. A CLI não clona nem escolhe um
destino automaticamente. Veja [ADR 0009](../decisions/0009-verify-and-explicit-apply.md).

## Comandos

```sh
memory-pier verify /pasta/pacote
memory-pier apply /pasta/pacote --project /checkout-de-teste --check
memory-pier apply /pasta/pacote --project /checkout-de-teste --write
```

`verify` aceita contexto v1, referência v1 e alterações v2, inclusive v2 sem
mudanças incluídas. Confere campos/tipos, versão, layout, integridade e mapeamentos.
Não toca no projeto, imprime somente resumo JSON (versão, número de payloads,
mudanças, omissões, avisos e declaração de redaction recebida). `valid: true` não
certifica autoria, veracidade do histórico, ausência de segredos ou completude.

`apply` exige v2 com pelo menos uma mudança e exatamente `--check` ou `--write`,
na ordem exibida. `--check` valida base, estado local, conflitos por divergência
de bytes/modos e resultados; retorna checked true, written false. `--write` repete
checagens e aplica os bytes verificados; written true só após conferir resultados
e concluir limpeza. O resumo mantém contagens de omissões/avisos: sucesso refere-se
às mudanças incluídas, não a todo o estado original.

Saída 0: operação solicitada concluída. Saída 1: pacote inválido, destino
incompatível ou falha operacional. Saída 64: uso inválido. Não usar saída 0 como
aprovação humana da transferência. Verify não muda redaction recebido.

## Verificações e limites

- Manifesto: até 1 MiB, campos obrigatórios/tipos e versões conhecidas, data RFC3339,
  hashes hexadecimais completos, sem campos extras/duplicados.
- Payloads: 2–67 arquivos, só HANDOFF.md, history.jsonl, changes.patch e
  files/NNNN.txt conforme finalidade. Recusar arquivos ausentes, extras, repetidos,
  não regulares e symlinks. Só files/ é subdiretório aceito; não segue links.
- Até 128 MiB por payload de contexto, 17 MiB de patch, 1 MiB por arquivo novo e
  160 MiB total. Histórico deve ser JSONL de objetos; conteúdo é dado histórico,
  não validado semanticamente nem usado para aplicar código.
- Até 64 seletores/mudanças; regras ASCII do v2. Recusar colisões de caixa e de
  prefixo arquivo/diretório para evitar ambiguidades entre filesystems.
- Cada mudança deve apontar a payload listado e caminho selecionado único. Todo
  payload de código deve ser usado. Conferir hashes/modos anteriores/posteriores,
  inclusive texto removido. Limite 8 MiB somando conteúdos de código antes/depois.
- Patch somente no dialeto de substituição integral do exportador. Cabeçalhos,
  contagens, newline final e modo devem coincidir com reconstituição canônica.
  Patches Git arbitrários, binários, symlinks, renomeações nativas e extensões não
  são suportados. V1 com finalidades reservadas de código também é recusado.

O receptor aplica validação Rust específica, mais restrita que o schema genérico.
O hash do payload não basta para autorizar caminhos ou resultados diferentes dos
registrados no manifesto. Pacote adulterado com todos os hashes refeitos pode
representar outro pacote válido: hashes não são assinatura nem autenticação.

## Condições do checkout

HEAD deve ser exatamente project.base_commit. Checkout deve estar limpo: sem
staged, unstaged, não rastreados ou submódulos não verificáveis. Ignorados fora da
seleção são preservados; ignorados em destino de adição são colisão. Diretórios
pais podem ser criados quando ausentes. Recusar symlinks, aliases de caixa,
repositórios aninhados, diretórios no lugar de arquivo e conteúdo/modo divergente.

Git apenas lê base/status; branch e remoto não comandam checkout ou rede. Não há
staging, commit ou push. Use checkout separado e interrompa outros escritores
antes da aplicação. --check não reserva nem bloqueia o destino para execução futura.
A biblioteca relê o destino antes de usar um plano preparado, sem reler pacote.

## Gravação e recuperação

Aplicação inicial suportada em Unix (macOS/Linux). Resultados têm permissões
privadas 0600/0700 e preservam o bit executável declarado; não copiam ACLs/metadados
especiais como propriedade. Originais guardados para rollback preservam seus inodes.

A pasta privada `.memory-pier-apply-<pid>-<contador>` contém `recovery.json`,
originais `old-N` e resultados preparados `new-N`. N é o índice começando em zero
no mapa de recovery, não o índice de seleção. A instalação usa hard links
exclusivos no mesmo filesystem. Em sucesso a pasta é removida.

Em erro comum, tenta retirar somente resultados ainda correspondentes aos bytes
instalados e restaurar originais; remove somente diretórios vazios criados pela
operação. Se a restauração falhar, preserva os backups e informa a pasta. Se a
limpeza final falhar, informa explicitamente que mudanças já foram aplicadas.

Após interrupção abrupta, não há garantia de rollback automático. Pare outros
escritores, preserve uma cópia da pasta de recuperação e confira cada destino
contra `recovery.json`/backups antes de qualquer restauração manual. Ausência de
old-N pode indicar que o arquivo ainda não foi movido. Não apague a pasta nem
repita --write até entender o estado. Não executar instruções vindas do pacote.
Não é transação atômica multiarquivo nem proteção contra alteração concorrente
maliciosa. Teste humano de interrupção/falha elétrica permanece pendente.
