# pseudo-engine

Pure Rust, zero-copy inference engine and CLI for ternary-quantized (`TQ2_0`, BitNet-style) GGUF models. Part of the [psudo](../README.md) ecosystem.

## Modules (`src/`)

| File | Role |
|---|---|
| `gguf.rs` | Zero-copy GGUF parser and tensor map (`memmap2`) |
| `tensor.rs`, `ops.rs` | Tensors, SIMD/integer ternary matmul (`matmul_tq2_0`) |
| `engine.rs` | Autoregressive transformer pipeline |
| `kv_cache.rs` | 2-tier forked KV cache (`fork_sequence` in <1 ms) |
| `sampler.rs` | Temperature/top-k/top-p/repetition sampling with PDA grammar mask (`PirPdaConstraint`) and channels (`Talk`, `Fim`, `Zettel`, `Pir`) |
| `pir_decompiler.rs` | Converts streamed pIR into readable output |
| `main.rs` | CLI, including MCP stdio server mode |

KV tiers: Tier 0 (`0..150t`) pinned invariants; Tier 1 (`150..~800t`) module context; Tier 2 (`~800..1200t`) ephemeral per-call runway.

## Build and run

```bash
cargo build --release --manifest-path pseudo-engine/Cargo.toml

# Interactive chat
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf --chat --color
# One-shot
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf -p "fn calculate_total() {"
# Benchmark
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf --benchmark
# MCP server over stdio (model stays hot)
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- --mcp
```

## CLI options

| Flag | Default | Description |
|---|---|---|
| `-m, --model` | `pseudo-100m-tq2_0.gguf` | GGUF model path |
| `-t, --tokenizer` | none | Tokenizer file |
| `-p, --prompt` | none | One-shot prompt |
| `--chat` | off | REPL with persistent KV cache |
| `-c, --ctx-size` | 1536 | Context size |
| `--temp` | 0.2 | Temperature |
| `--top-p` | 0.95 | Nucleus sampling |
| `--top-k` | 40 | Top-k |
| `--rep-penalty` | 1.1 | Repetition penalty |
| `--max-tokens` | 512 | Generation limit |
| `--threads` | 4 | Worker threads |
| `--pir-mode` | off | Constrain output to pIR |
| `--raw-prompt` | off | Skip prompt templating |
| `--color` | off | Colored output |
| `--benchmark` | off | TTFT, decode TPS, memory |
| `--mcp` | off | Serve MCP on stdio |

## Tests

```bash
cargo test --manifest-path pseudo-engine/Cargo.toml
```

Suites in `tests/`: `engine_tests.rs`, `mcp_stdio_tests.rs`, `subsystem_integration_tests.rs`. Model conformance: [../tests/model_conformance](../tests/model_conformance).

## Rules

No `unwrap()`/`expect()` in production paths; zero-copy weight mapping; integer-only ternary accumulation. See [AGENTS.md](AGENTS.md), [EXPECTATIONS.md](EXPECTATIONS.md), [DEV_PLAN.md](DEV_PLAN.md).
