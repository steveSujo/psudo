# AGENTS.md — `psudo` Orchestrator & Repository Manifest

> **Primary Agent Role (`skill://psudo-orchestrator`):** Read [`.agents/skills/psudo-orchestrator/SKILL.md`](./.agents/skills/psudo-orchestrator/SKILL.md) first when building apps or orchestrating `.psudo` capsules. Never bypass `psudo.bin` with raw binary/hex edits or dump monolithic algorithms into a single `@stub`.
> **Repository Scope:** `psudo` is an exploration into fitting local coding synthesis onto sub-4 GB VRAM (`100.7M` / `33.6M` `TQ2_0` weights) by compiling a token-compressed `.psudo` DSL via Polish-prefix `pIR` + PDA logit masking into panic-free Rust (`0` `.unwrap()` / `.expect()`).

---

## 1. Orchestrator Workflow (`skill://psudo-orchestrator`)

- **Division of Labor:**
  - **Orchestrator (You):** Architect dataflow, choose/create `.pseudo/sandbox/` JSON fixtures, declare small single-purpose `@stub` signatures (`psudo vfs stub`), wire `main_task` (`psudo vfs wire`), author `test "..."` assertions (`psudo vfs test`), and run static/build gates (`psudo check -a -b`, `psudo test`, `psudo build --run`).
  - **Inner Engine (`pseudo-engine` + `pseudo-100m-tq2_0.gguf`):** Synthesizes `@stub` bodies as `pIR` under `PirPdaConstraint` grammar masking (`47.0 MB` `TQ2_0` checkpoint; `33.6M` embedded fallback for smoke tests).
- **Standard 6-Step Loop:**
  1. `psudo init <DIR> --template <blank|log|sensor|quota|life|fractal|finance> [--with-docs]`
  2. `psudo contract <CAPSULE> [--json|--yaml]` & `psudo vfs show <CAPSULE> -h`
  3. `psudo vfs stub <CAPSULE> <FN> [PARAMS...] --directive "<concise atomic rule>"`
  4. `psudo synth <CAPSULE> --write`
  5. `psudo vfs wire <CAPSULE> [--call "<STMT>" ...] [--stdin] [--return "<EXPR>"]` & `psudo vfs test <CAPSULE> <NAME> --call "<EXPR>" --expect "<VAL>"`
  6. `psudo check <CAPSULE> -a -b` -> `psudo test <CAPSULE>` -> `psudo build <CAPSULE> --run [--headless]`
- **Extensibility Without Retraining (`psudo.toml` + `ratatui`):**
  - TUI builtins (`tui_metric`, `tui_gauge`, `tui_sparkline`, `tui_table`, `tui_log`, `tui_canvas`, `tui_barchart`, `tui_row`, `tui_col`, `tui_dashboard`, `tui_render`, `tui_grid`, `tui_fractal`) and `tokio_sleep_ms` (`#[tokio::main]`).
  - Declare custom Rust builtins under `[addons.<name>]` in `psudo.toml` (`builtins`, `cargo_deps`, `rust_main_attr`, `rust_main_async`, `rust_file`, `mock_returns`). Keep `rust_file` UTF-8 without BOM.

---

## 2. Docs, Skills & Templates Index

| Item | Path | Purpose |
|---|---|---|
| **Orchestrator Skill** | [`.agents/skills/psudo-orchestrator/SKILL.md`](./.agents/skills/psudo-orchestrator/SKILL.md) | Canonical CLI loop, `@stub` rules, security gate, diagnostics (`E001`–`E005`). |
| **MCP Configuration** | [`.agents/mcp_config.json`](./.agents/mcp_config.json) | Local MCP stdio server definition (`psudo mcp`). |
| **Overview & Quickstart** | [`README.md`](./README.md) | Architecture pipeline, metrics, build/install, CLI quickstart. |
| **Templates Guide** | [`TEMPLATES.md`](./TEMPLATES.md) | Starter catalog vs. isolated exercise workspaces. |
| **Release History** | [`CHANGELOG.md`](./CHANGELOG.md) | SemVer changelog (`psudo` `0.10.0`, `pseudo-engine` `0.4.0`). |
| **Workspace Manifest** | [`Cargo.toml`](./Cargo.toml) | Cargo workspace (`pseudo-lang`, `pseudo-engine`, `pseudo-transpiler`). |
| **Starter Templates** | [`templates/`](./templates/) | `log_pattern_scanner.psudo`, `sensor_alert_monitor.psudo`, `quota_fee_calculator.psudo`, `tui_game_of_life.psudo`, `tui_fractal_zoom.psudo`, `tui_personal_finance.psudo` + [`.pseudo/sandbox/`](./templates/.pseudo/sandbox/) fixtures. |

---

## 3. Repository Code Manifest

### A. `pseudo-engine` ([`pseudo-engine/`](./pseudo-engine/Cargo.toml)) — Pure-Rust `TQ2_0` Inference Runtime
- `src/gguf.rs`: Zero-copy GGUF parser + `memmap2` tensor mapping.
- `src/tensor.rs`: `Tensor` storage, views, quantized block descriptors.
- `src/ops.rs`: `TQ2_0` 2-bit ternary SIMD integer matmul + SwiGLU / RMSNorm / Softmax.
- `src/kv_cache.rs`: 2-tier INT8 KV cache (`Tier 0` pinned, `Tier 1` module, `Tier 2` ephemeral runway) + `fork_sequence`.
- `src/sampler.rs`: `PdaState` (`L44`), `PirPdaConstraint` (`L61`), `Sampler` (`L783`) — decode-time grammar masking over `8,192`-token BPE vocab.
- `src/engine.rs` & `src/lib.rs`: Autoregressive Transformer forward pass, KV tier routing, and library exports.
- `src/daemon.rs`: `HealthResponse` (`L13`), `SynthRequest` (`L20`), `SynthResponse` (`L29`), `run_daemon` (`L35`), `split_addon_directive` / `splice_addon_builtin_return` (`L260–L345`), `rebind_psudo_signature` (`L350`) — local HTTP `/synth` server + addon/param repair.
- `src/pir_decompiler.rs`: `pIR` token stream -> `.psudo` decompiler.
- `src/main.rs`: Engine CLI (`--chat`, `--benchmark`, `-p`, `--daemon`, `--mcp`).

### B. `pseudo-lang` / `psudo` ([`pseudo-lang/`](./pseudo-lang/Cargo.toml)) — DSL Compiler, Capsule VFS, Checker, Transpiler, MCP & LSP
- `src/lexer.rs` & `src/tokens.rs`: Python-indented `.psudo` lexer + token spans (`//` comments, 4-space indent).
- `src/parser.rs`: `DataTypes` (`L84`), `Expression` (`L203`), `Interpret` (`L301`), `CheckedInterpret` (`L313`), `eval_builtin_inner` (`L1240`, incl. `tui_*`, `tokio_sleep_ms`, `[addons.*]` `mock_returns`), `Statment` / `Parser`.
- `src/pir_parser.rs`: `PirTokenType` (`L9`), `PirLexer` (`L89`), `PirParser` (`L402`), `parse_pir` (`L947`).
- `src/resolver.rs`: `Resolver` (`L30`, builtins list `L80–L135`) — lexical scope + addon builtin registration.
- `src/ownership.rs`: `BorrowChecker` (`L239`), `CheckedEnvironment` (`L356`) — `Copy` vs `Move` analysis.
- `src/diagnostics.rs` & `src/error_handler.rs`: Span-aware compiler/runtime errors (`E001`–`E005`).
- `src/vfs.rs`: `WorkspaceCapsule` (`L34`, `upsert_stub`, `upsert_test`, `wire_entrypoint`, `merge_delta`), `resolve_capsule_and_vpath`, `pack_directory_to_capsule`, `unpack_capsule_to_directory` — sealed `.psudo.bin` container.
- `src/blackbox.rs`: `synthesize_stubs_in_source`, `ensure_daemon_running`, `post_daemon_synth`, `synthesize_stubs_in_capsule`.
- `src/embedded_model.rs`: Baked-in `pseudo-33m-tq2_0.gguf` extraction.
- `src/codegen.rs`: `transpile_psudo_to_rust` (`L21`), `build_rust_binary_from_capsule` (`L163`), `build_rust_binary_artifact` (`L436`), `get_tui_rust_runtime` (`L817`, `ratatui` + `crossterm` headless/interactive widgets), `PsudoVal` (`L1291`). Shared `CARGO_TARGET_DIR` build cache + zero-`.unwrap()`/`.expect()` enforcement.
- `src/contract_gen.rs`: Behavioral contract YAML/JSON emitter (`params_used`, `branches`, `returns`, `guards`, `side_effects`, `sandbox_io`, `dependencies`, `control_flow`).
- `src/config.rs`: `AddonSpec` (`L11:C1-L27:C2`, `mock_returns`), `ProjectConfig` (`L31:C1-L39:C2`), `PSUDO_CAPSULE_TOML` / `psudo.toml` loader.
- `src/crdt.rs`, `src/zettel.rs`, `src/projector.rs`, `src/context.rs`, `src/dag.rs`: SymbolMap CRDT, Zettel memory, `<150t` context slices, `TaskDAG` topological sort + Mermaid/DOT export.
- `src/fs/sandbox.rs`: Isolated `.pseudo/sandbox/` I/O + size/path guards.
- `src/mcp.rs` & `src/lsp/backend.rs`: MCP stdio server (`serve_mcp_stdio`) + `tower-lsp` language server.
- `src/user_docs.rs`, `src/utils.rs` & `src/llm_cli_guide.md`: Embedded offline CLI guide (`psudo docs` / `psudo init --with-docs`) and shared utilities.
- `src/main.rs`: `psudo` CLI entry point (`init`, `vfs`, `contract`, `synth`, `check`, `test`, `run`, `build`, `benchmark`, `dag`, `watch`, `mcp`, `lsp`, `docs`).
- `tests/integration_tests.rs`: 15 compiler/borrow/TUI/addon integration tests.

### C. `pseudo-transpiler` ([`pseudo-transpiler/`](./pseudo-transpiler/Cargo.toml)) — AST/IR Lowering Crate
- `src/ir.rs`: `TsNode` intermediate representation.
- `src/from_psudo.rs` / `src/from_rust.rs` / `src/to_psudo.rs` / `src/to_rust.rs`: Bidirectional `.psudo` <-> `TsNode` <-> safe Rust lowering.

---

## 4. Build, Test & Edit Rules

```powershell
# Build release binaries
cargo build --release --manifest-path Cargo.toml

# Run unit & integration tests (MSRV 1.75+, Edition 2021, zero-panic invariant)
cargo test --manifest-path Cargo.toml --lib
cargo test --manifest-path Cargo.toml --test integration_tests
```

- **Zero-Panic Invariant:** Never introduce `.unwrap()` or `.expect()` in production runtime or transpiled output paths.
- **No Grammar Drift Without Retraining:** Do not alter `.psudo` surface tokens (`lexer.rs`) or `pIR` opcodes (`pir_parser.rs` / `sampler.rs`) unless retraining the `TQ2_0` checkpoints; add new capabilities via runtime builtins or `[addons.<name>]` in `psudo.toml`.
