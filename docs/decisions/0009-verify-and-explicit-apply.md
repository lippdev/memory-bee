# 0009 — Verificação e aplicação explícita

Status: adotado em 2026-09-24. Terceiro recorte da etapa 04.

## Separação de ações

`verify <pacote>` valida pasta local sem consultar ou alterar projeto.
`apply <pacote> --project <checkout> --check` prepara e verifica aplicação sem
escrever. Somente `--write` aplica. Não escolher checkout, clonar, fazer staging,
commit ou push implicitamente. Operar inicialmente em macOS/Linux com modos Unix.

Verificar formatos v1/v2 suportados com tipos Rust estritos, presença dos campos,
limites e regras semânticas; não incorporar motor genérico de JSON Schema nesta
entrega. Schemas versionados continuam contrato e validação independente no CI.
Receptor é mais restritivo: v1 somente contexto/referência, v2 no dialeto de patch
emitido pelo exportador, nomes sem colisões de caixa ou prefixo arquivo/diretório.
Não aceitar extensões desconhecidas ou campos duplicados. Hash confere integridade,
mas não autentica remetente nem certifica revisão de segredos ou conteúdo histórico.

## Conteúdo recebido não é comando

Não passar patch recebido a `git apply`. Decodificar apenas hunks de substituição
integral; reconstruir patch canônico e comparar bytes para validar cabeçalhos,
caminhos, contagens e ausência de mudanças ocultas. Conferir hashes por arquivo.
Mudanças apenas de modo são verificadas contra o blob real da base durante check.
Nenhum filtro, hook ou texto histórico é executado; Git só lê metadados/blobs e
status com as proteções existentes.

## Destino e gravação

Exigir HEAD exato e checkout limpo/inteiramente verificável. Recusar submódulos,
base divergente, qualquer alteração local (mesmo fora da seleção), colisões de
adição inclusive ignoradas, symlinks, repositórios aninhados e aliases de caixa.
Comparar disco e blobs da base, incluindo hashes/modos, e conferir resultados.
Essa restrição permite primeira aplicação conservadora; mesclar com trabalho local
ou base divergente é escopo futuro, sem fallback destrutivo.

Plano usa bytes já lidos, sem reabrir pacote durante aplicação. Revalidar destino
antes de escrever. Criar pasta privada `.memory-pier-apply-<pid>-<contador>` na raiz,
com arquivos preparados e recovery.json. Mover originais para backup dentro dela;
instalar por hard link exclusivo, nunca substituindo um destino que reapareceu.
Índice Git intocado; modos preservam bit executável, com permissões privadas 0600/
0700. Exigir suporte a hard links no filesystem; falhas comuns disparam restauração.

Falha comum tenta restaurar originais sem apagar conteúdo concorrente divergente.
Falha de restauração mantém backups e mensagem para intervenção. Interrupção abrupta
pode deixar operação parcial: não há transação atômica multiarquivo nem recovery
automático. recovery.json mapeia índices/caminhos e originais old-N; não executá-lo.
Limpeza que falhar após sucesso é relatada como erro com alterações já aplicadas.
Não prometer proteção contra processo malicioso concorrente; operar sem escritores
simultâneos. Falha elétrica e recuperação de permissões especiais/ACLs de resultados
não foram certificadas.

## Validação e sequência

Testes sintéticos de ponta a ponta e negativos verificam integridade, preservação,
modos, limites, colisões e rollback com falha injetada. Python confere schemas e
hashes independentemente e executa verify/check/write em clone sintético separado.
Ensaios manuais continuam pendentes no roteiro; M1 ainda requer retomada humana
controlada. Próximo recorte de produto: segundo leitor (Codex), sem iniciar TUI ou
contas antes de fechar os contratos de leitura/retomada.
