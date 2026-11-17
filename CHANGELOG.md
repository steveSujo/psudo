# Changelog

All notable changes to `psudo` are documented in this file.

## [0.9.0] - 2026-11-18

### Changed
- **Breaking CLI**: Removed the `-b` / `--check-borrows` flag across `psudo` and `psudo check`; borrow checking (`CheckedInterpret`) is now unconditionally enabled by default.

### Added
- **Declarative VFS testing & wiring**: `psudo vfs test <TEST_NAME> --call "<EXPR>" --expect "<VAL>"` (`WorkspaceCapsule::upsert_test`) and `psudo vfs wire [--entry <FN>] [--call "<STMT>"] [--return "<EXPR>"]` (`WorkspaceCapsule::wire_entrypoint`).
- **Direct expression evaluation**: `psudo run [<CAPSULE|FILE>] --call "<EXPR>"` invokes any capsule function without editing `main_task`.
- **Behavioral contract audit**: `psudo contract` `StaticImplSummary` now reports `params_used`, `branches`, `returns`, multi-guard + `@stub` directive guard extraction, and directive-to-AST call-drift warnings.

### Fixed
- **`psudo run` parity & exit codes**: Delegated stdlib builtins (`load_json`, `len`, `persistent_kv`, `set`, `get`, `str`, etc.) and early-return propagation in `CheckedInterpret`, matched unquoted string `print` output with `psudo build --run`, and exited non-zero on undefined runtime symbols.
- **`psudo dag export`**: Extracted call-graph edges from function bodies and `@stub` traces, rendered isolated vertices in `--format mermaid`, routed `[psudo dag]` banner to `stderr`, and made topological/DOT ordering deterministic.
- **`psudo contract` & `psudo synth`**: Mixed symbol name, params, and directive into `content_hash` to prevent hash collisions, fixed double-space `intent` formatting on single-word functions, and normalized synthesized body indentation to 4 spaces.

## [0.8.0] - 2026-11-16

### Added
- **Bundled 100.7M & 33.6M `TQ2_0` models (`pseudo-engine v0.3.0`)**: Added `pseudo-100m-tq2_0.gguf` (primary) and `pseudo-33m-tq2_0.gguf` (fallback), embedded `tokenizer.json` directly into `pseudo-engine`, and unified root Cargo workspace build (`cargo build --release`).
- **Starter templates & examples**: Added `blank`, `log`, `sensor`, and `quota` templates with `.pseudo/sandbox/` JSON fixtures, pre-built `examples/` capsules, and `.agents/skills/psudo-orchestrator/SKILL.md`.
- **Conformance suites**: Added 100-prompt Turing suite (`PROMPT_TESTCASES.md`) and 80-prompt multi-aspect suite (`MODEL_ASPECT_TESTCASES.md`).

## [0.7.0]

### Added
- **Multi-module verification**: `psudo check <capsule> --all-modules` scans all virtual `.psudo` modules in a capsule for unresolved stubs.
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
