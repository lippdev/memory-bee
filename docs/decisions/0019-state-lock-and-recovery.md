# 0019 — Trava do estado pelo sistema e recuperação de escrita

Status: adotado em 2026-09-28 para a issue #41. Substitui, para as pastas de
estado do workspace, a trava por arquivo criado com `create_new` descrita no
[ADR 0018](0018-unified-workspace.md).

## Contexto

`workspace.lock` e `claude-native.lock` eram criados com `create_new` e
apagados na saída. Depois de um kill ou crash, a próxima abertura falhava até o
usuário confirmar que não havia processo ativo e remover o arquivo à mão. Um
`*.new` deixado por uma escrita interrompida também bloqueava todas as gravações
seguintes. O snapshot era sincronizado, mas o diretório não.

## Decisão

- Trava consultiva do sistema (`File::try_lock`, `flock` em Unix) sobre o arquivo
  de trava, mantida aberta enquanto o estado está aberto. O arquivo continua no
  disco e guarda só o PID, para a mensagem de "em uso". O kernel solta a trava
  quando o processo termina, inclusive por `kill -9`; um processo vivo nunca é
  desalojado. Arquivo de trava que não seja arquivo comum (symlink) é recusado.
- Com a trava obtida, um `*.new` remanescente é de uma escrita que parou antes do
  rename: o snapshot confirmado continua sendo o último estado completo. O
  remanescente é renomeado para `*.new.recovered-<nanossegundos>` na mesma pasta,
  nunca carregado nem apagado, e a interface informa onde ficou.
- Escrita: arquivo temporário novo 0600, `sync_all`, rename sobre o snapshot e
  `sync_all` do diretório. Em macOS, `sync_all` usa `F_FULLFSYNC`. Erro antes do
  rename remove o temporário e preserva o snapshot anterior.
- Código compartilhado em `src/workspace/private.rs` pelas duas pastas de estado.

## Garantias e limites

- Depois de `save` retornar sucesso, o snapshot sobrevive a queda de energia em
  sistemas de arquivos locais que honram `fsync`. Sistemas de arquivos de rede
  podem não implementar `flock` nem essa durabilidade; não são suportados.
- Proteção contra outro processo Memory Bee, não contra escritores hostis com o
  mesmo usuário nem contra alteração dos diretórios ancestrais.
- Versões anteriores apagavam a trava; uma versão antiga aberta junto com a nova
  na mesma pasta não respeita o `flock`. Não misturar versões numa pasta.
- Persistência continua síncrona. Medição local (Linux, disco virtual, `cargo
  test --release -- --ignored snapshot_latency`): estado Claude de 64 KiB com
  mediana 0,6 ms e máximo 23,6 ms; estado da demo no limite de 16 MiB com
  mediana 28,5 ms e máximo 130,6 ms. Discos lentos podem congelar a interface
  por mais tempo; mover a escrita para outra thread fica para quando um estado
  real grande existir.
