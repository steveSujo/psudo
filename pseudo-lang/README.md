# pseudo-lang (`psudo`)

Core language implementation and `psudo` CLI for the [psudo](../README.md) project.

It started as a small Lox-inspired interpreter, then grew into the compiler, borrow checker, `psudo.bin` virtual capsule, `pIR` codec, MCP server, and LSP that sits in front of `pseudo-engine` and `pseudo-transpiler`.

## What's Inside (`src/`)

| File | Role |
|---|---|
| `main.rs` | `psudo` CLI (`init`, `contract`, `vfs`, `synth`, `check`, `test`, `run`, `build`, `mcp`, `--lsp`) |
| `lexer.rs`, `parser.rs` | 4-space Python-indented `.psudo` lexer, Pratt parser, and tree-walking interpreter |
| `pir_parser.rs` | `pIR` S-expression parser (`v`, `=`, `p`, `?`, `w`, `f`, `r`, `c`, `m`) |
| `ownership.rs` | Static borrow checker (`Copy` primitives vs `Move` arrays/maps) |
| `vfs.rs`, `contract_gen.rs` | Sealed `psudo.bin` workspace capsule and YAML/JSON behavioral contract extractor |
| `blackbox.rs` | Local GGUF synthesis bridge (`pseudo-100m-tq2_0.gguf` / embedded 33M fallback) |
| `codegen.rs` | Lowering pass to zero-`unwrap()` safe Rust |
| `mcp.rs`, `lsp/` | JSON-RPC 2.0 MCP stdio server and Language Server Protocol backend |

## Build & Run

```bash
# From repo root
cargo build --release
./target/release/psudo --help

# Common workflow
./target/release/psudo init my-app --template blank --with-docs
./target/release/psudo contract my-app/psudo.bin
./target/release/psudo vfs stub my-app/psudo.bin add_one x --directive "return x + 1"
./target/release/psudo synth my-app/psudo.bin --model ./pseudo-100m-tq2_0.gguf --write
./target/release/psudo check my-app/psudo.bin -a -b
./target/release/psudo build my-app/psudo.bin --run
```

You can also run standalone `.psudo` or `.pir` scripts directly:

```bash
./target/release/psudo -f pseudo-lang/scripts/hello.psudo
./target/release/psudo --pir -f pseudo-lang/scripts/hello.pir
./target/release/psudo -i          # Interactive REPL
./target/release/psudo --lsp       # Language Server Protocol (stdio)
```

## Tests

```bash
cargo test --manifest-path pseudo-lang/Cargo.toml --lib
```
