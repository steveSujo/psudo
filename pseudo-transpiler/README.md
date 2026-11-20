# pseudo-transpiler

Bidirectional lowering crate between `.psudo` wire output and safe Rust via a shared tree-sitter-style IR (`TsNode`), part of [psudo](../README.md) and built on the `psudo` crate (`../pseudo-lang`).

Because `.psudo` and `pIR` exist as token-compressed scaffolding around the 100M ternary model rather than a standalone ecosystem, this crate lowers verified stubs into zero-`.unwrap()` Rust so `cargo` can compile them into native binaries.

## Modules (`src/`)

| File | Role |
| --- | --- |
| `ir.rs` | `TsNode` intermediate representation |
| `from_psudo.rs` | `.psudo` to IR (`psudo_to_ir`) |
| `to_rust.rs` | IR to Rust (`ir_to_rust`) |
| `from_rust.rs` | Rust to IR (`rust_to_ir`, uses `tree-sitter-rust`) |
| `to_psudo.rs` | IR to `.psudo` (`ir_to_psudo`) |
| `lib.rs` | High-level API, re-exporting `psudo::{blackbox, codegen, context}` |

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

Generated Rust follows the zero-`.unwrap()` and zero-`.expect()` rule.

## Build and test

```bash
cargo build --manifest-path pseudo-transpiler/Cargo.toml
cargo test  --manifest-path pseudo-transpiler/Cargo.toml
```

Tests: `tests/transpiler_tests.rs`.
