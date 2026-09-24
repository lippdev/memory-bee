# Árvore sintética de descoberta

Arquivos escritos a partir das fixtures sintéticas locais; nenhum chat real.

- `arbitrary/session.jsonl`: projeto `/synthetic/project`, raiz `u1` e pontas `a1`/`a2`.
- `arbitrary/session/subagents/agent-worker.jsonl`: mesmo projeto, subagente separado.
- `another/unrelated.jsonl`: projeto diferente, excluído do resultado pelo `cwd`.

```sh
cargo run --locked -- sessions --root testdata/claude-projects --project /synthetic/project
cargo run --locked -- inspect testdata/claude-projects/arbitrary/session.jsonl --leaf a1
```

Os caminhos de projeto são metadados fictícios; não é necessário criá-los no disco.
Testes em `tests/navigation.rs` criam árvores temporárias adicionais para erros,
permissões, symlinks, limites, colisões de nomes e parentesco inválido.
