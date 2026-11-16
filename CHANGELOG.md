# Changelog

All notable changes to `psudo` are documented in this file.

## [0.7.0]

### Added
- **Multi-module verification**: `psudo check <capsule> -a` scans all virtual `.psudo` modules in a capsule for unresolved stubs.
- **Timing & micro-benchmarking builtins**: `time_micros()`, `time_nanos()`, `sleep_ms(ms)`, and `benchmark(iterations, callable)`.
- **Topological DAG task engine**: `TaskDAG` with Kahn's algorithm sorting, cycle detection (`CycleError`), and `psudo dag export --format mermaid/dot`.
- **Rate limiters & persistent KV**: `rate_limiter_sliding_window`, `rate_limiter_token_bucket`, `consume`, and `persistent_kv` (`get`, `set`, `has`, `del`).
- **In-capsule test runner**: native `test "..."` blocks, `assert` / `assert_eq` / `assert_ne`, and `psudo test` with TAP/JUnit output.
- **Model probe & 1-shot auto-heal**: `psudo mcp info`, `--auto-heal`, and `--trace-grammar`.

## [0.6.0]

### Added
- **Sealed VFS & stub occlusion**: workspace capsules (`psudo.bin`) default to stub-only inspection (`psudo vfs show -h`) and declarative stub upserts (`psudo vfs stub`).
- **Span diagnostics**: `E001`–`E005` error codes with line/column spans and `--json` output.
- **Multi-module Rust lowering**: `codegen.rs` aggregates all virtual `.psudo` modules into a single safe Rust crate.

### Fixed
- Idempotent MCP stub upserts (`psudo_vfs_upsert_stub`).

## [0.5.0]

### Added
- **pIR (Pseudo Intermediate Representation)**: compact prefix S-expression syntax (`v`, `=`, `p`, `?`, `w`, `f`, `r`, `c`, `m`) with `--pir` execution.
- **Classes & inheritance**: `class Name`, `class Derived < Base`, `super.method()`, and LSP support.
- **Python-style indentation**: replaced `;`, `{`, `}`, `:` blocks with 4-space indentation.
