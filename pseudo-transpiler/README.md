# pseudo-transpiler

Bidirectional transpiler between `.psudo` and safe Rust, via a shared tree-sitter-style IR (`TsNode`). Part of the [pseudo](../README.md) ecosystem. Depends on the `psudo` crate (`../pseudo-lang`).

## Modules (`src/`)

| File | Role |
|---|---|
| `ir.rs` | `TsNode` intermediate representation |
| `from_psudo.rs` | `.psudo` → IR (`psudo_to_ir`) |
| `to_rust.rs` | IR → Rust (`ir_to_rust`) |
| `from_rust.rs` | Rust → IR (`rust_to_ir`, uses `tree-sitter-rust`) |
| `to_psudo.rs` | IR → `.psudo` (`ir_to_psudo`) |
| `lib.rs` | High-level API; re-exports `psudo::{blackbox, codegen, context}` |

## API

```rust
use pseudo_transpiler::*;

let rust  = transpile_psudo_to_rust(src)?;   // .psudo -> Rust
let psudo = transpile_rust_to_psudo(rs)?;    // Rust -> .psudo
let a = roundtrip_psudo(src)?;               // .psudo -> IR -> .psudo
let b = roundtrip_rust(rs)?;                 // Rust -> IR -> Rust

// Build a binary from a .psudo file
let report = compile_psudo_file_to_binary(input, out_dir, /*release*/ true)?;
```

Generated Rust follows the zero-`unwrap()` rule.

## Build and test

```bash
cargo build --manifest-path pseudo-transpiler/Cargo.toml
cargo test  --manifest-path pseudo-transpiler/Cargo.toml
```

Tests: `tests/transpiler_tests.rs`. See [AGENTS.md](AGENTS.md), [EXPECTATIONS.md](EXPECTATIONS.md), [DEV_PLAN.md](DEV_PLAN.md).
