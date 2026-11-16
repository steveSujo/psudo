# Changelog

All notable changes to the `pseudo` ecosystem will be documented in this file.

## [0.7.0] - 2026-11-09

### Added
- **Recursive Multi-Module Verification (`Imp 16`)**: Added `psudo check <capsule> --all-modules` (`-a`) to recursively scan and audit 100% of virtual `.psudo` modules in a capsule for unresolved stubs.
- **High-Resolution Monotonic Timing & Micro-Benchmarking Primitives (`Imp 17`)**: Added standard library built-ins `time_micros()`, `time_nanos()`, `sleep_ms(ms)`, and `benchmark(iterations, callable)`.
- **Topological DAG Task Engine (`Imp 11 & 12`)**: Implemented `TaskDAG` with Kahn's algorithm topological sorting, static compile-time cycle detection (`CycleError`), and CLI visual graph export (`psudo dag export --format mermaid/dot`).
- **Multi-Algorithm Rate Limiting & Persistent KV Storage (`Imp 10`)**: Added first-class rate limiters (`rate_limiter_sliding_window`, `rate_limiter_token_bucket`, `consume`) and persistent KV store (`persistent_kv`, `get`, `set`, `has`, `del`), transpiled thread-safely with zero `.unwrap()`.
- **In-Capsule Test Suite Runner (`Imp 02`)**: Added native `test` blocks, assertion builtins (`assert`, `assert_eq`, `assert_ne`), and `psudo test` with TAP and JUnit reporting options.
- **Model Probe & 1-Shot Auto-Healing Bridge (`Imp 05`)**: Added `psudo mcp info` model inspection command, 1-shot auto-healing error frame repair (`--auto-heal`), and PDA grammar transition tracing (`--trace-grammar`).

## [0.6.0] - 2026-11-09

### Added
- **Read-Only VFS & pIR Stub Occlusion Pipeline**: Enforced strict read-only access to workspace capsules (`psudo.bin`) for external callers and user LLMs. Prohibited raw `psudo vfs write`, confining user mutations exclusively to declarative stub upserts (`psudo vfs stub` and MCP `psudo_vfs_upsert_stub`).
- **Token-Compressed Stub Inspection**: `psudo vfs show` defaults to stub-only occlusion rendering in canonical pIR or human-readable `.psudo` format via `-h` / `--human`.
- **Precise Lexer/Parser Spans & Diagnostics (`Imp 01`)**: Enriched `Span` with line, column, offset, and length coordinates. Implemented structured diagnostics with `E001`–`E005` error codes, remediation help messages, terminal source underlining, and `--json` check output.
- **VFS Packaging Ergonomics & Name Sanitization (`Imp 04`)**: Added explicit package name precedence (`psudo.toml` → `--name` CLI flag → sanitized directory basename).
- **Multi-Module VFS Aggregation (`Imp 13`)**: Updated `codegen.rs` to aggregate all virtual `.psudo` modules inside capsule VFS into a unified safe Rust compilation buffer.
- **Playground Scenarios & Test Runner**: Added `rate-limiter`, `task-dag`, and `stress-cluster` playground workspaces along with automated `--test` and `--reset` test runner scripts (`100% pass rate` across all scenarios).

### Fixed
- **MCP Upsert Idempotence**: Delegated `psudo_vfs_upsert_stub` to `capsule.upsert_stub(...)` to prevent duplicate definitions on updates.
- **CLI Help Text**: Corrected `vfs write` help reference to `stub` in `main.rs`.
