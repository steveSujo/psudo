# pseudo-engine

Pure-Rust, zero-copy inference runtime for 2-bit ternary (`TQ2_0`) GGUF models, built as the core engine of [psudo](../README.md).

I wrote this in pure Rust with `memmap2`, integer-only ternary accumulation (`matmul_tq2_0`), and an INT8 2-tier KV cache so that a 47.0 MB (100.7M) model runs locally inside a 4 GB VRAM budget without Python, C++, or CUDA toolchains.

## Modules (`src/`)

| File | Role |
| --- | --- |
| `gguf.rs` | Zero-copy GGUF parser and tensor map (`memmap2`) |
| `tensor.rs`, `ops.rs` | Tensors and integer-only ternary matmul (`matmul_tq2_0`) |
| `engine.rs` | Autoregressive transformer pipeline |
| `kv_cache.rs` | 2-tier forked INT8 KV cache (`fork_sequence` in <1 ms) |
| `sampler.rs` | Sampling with pushdown automaton (`PirPdaConstraint`) logit mask and channels (`Talk`, `Fim`, `Zettel`, `Pir`) |
| `pir_decompiler.rs` | Converts streamed Polish-prefix `pIR` into readable `.psudo` |
| `main.rs` | CLI and persistent `--mcp` / `--daemon` stdio server |

KV tiers: Tier 0 (`0..150t`) holds pinned invariants, Tier 1 (`150..~800t`) holds module context, and Tier 2 (`~800..1200t`) acts as the ephemeral per-call runway.

## Build and run

```bash
cargo build --release --manifest-path pseudo-engine/Cargo.toml

# Interactive chat
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf --chat --color
# One-shot
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf -p "fn calculate_total() {"
# Benchmark (TTFT, decode TPS, memory)
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- -m ./pseudo-100m-tq2_0.gguf --benchmark
# MCP server over stdio (model stays hot)
cargo run --release --manifest-path pseudo-engine/Cargo.toml -- --mcp
```

## CLI options

| Flag | Default | Description |
| --- | --- | --- |
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
| `--pir-mode` | off | Constrain output to `pIR` |
| `--raw-prompt` | off | Skip prompt templating |
| `--color` | off | Colored output |
| `--benchmark` | off | TTFT, decode TPS, memory |
| `--mcp` | off | Serve MCP on stdio |

## Tests

```bash
cargo test --manifest-path pseudo-engine/Cargo.toml
```

Suites in `tests/`: `engine_tests.rs`, `mcp_stdio_tests.rs`, `subsystem_integration_tests.rs`. Model conformance: [../tests/model_conformance](../tests/model_conformance).

## Design Rules

No `.unwrap()` or `.expect()` in production paths, zero-copy weight mapping via `memmap2`, and integer-only ternary accumulation.
