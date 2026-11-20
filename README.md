# psudo

A **100M-parameter 2-bit ternary (`TQ2_0`, 47.0 MB) coding model** and pure-Rust runtime built to synthesize local functions inside a **4 GB VRAM** budget, with a 33.6M (23.2 MB) fallback baked into the binary.

![Demo](demo.gif)

## At a Glance

| Dimension | Spec |
| --- | --- |
| **Model Checkpoints** | [`pseudo-100m-tq2_0.gguf`](pseudo-100m-tq2_0.gguf) (`47.0 MB`, `100.7M` params) + [`pseudo-33m-tq2_0.gguf`](pseudo-33m-tq2_0.gguf) (`23.2 MB`, `33.6M` params) + [`tokenizer.json`](tokenizer.json) (`8,192` BPE) |
| **Hardware Target** | `< 4 GB` VRAM edge/consumer budget (no cloud GPU reliance) |
| **Runtime Stack** | Zero-dependency pure Rust: `memmap2` GGUF loader, `TQ2_0` ternary matmuls, 2-tier INT8 KV cache, and `PirPdaConstraint` logit mask |
| **Wire Format** | Polish-prefix `pIR` cutting token count by `45-60%` with `0%` decode-time syntax errors |
| **Output Target** | Borrow-checked safe Rust with zero `.unwrap()` or `.expect()` calls |

## 30-Second Quickstart

Both quantized `.gguf` weights ship in this repo and compile with plain `cargo` (Rust `1.75+`, no Python, C++, or CUDA toolchains):

```bash
git clone https://github.com/steveSujo/psudo.git
cd psudo
cargo build --release
./target/release/psudo demo
```

To run a bundled capsule end-to-end or synthesize your own stub:

```bash
# Run a pre-built capsule with sandbox fixtures
./target/release/psudo build examples/log-incident-filter/psudo.bin --run

# Orchestrator loop: declare a stub, synthesize locally with the 100M GGUF, test, and build
./target/release/psudo init my-app --template blank && cd my-app
../target/release/psudo vfs stub clamp_val val lo hi --directive "clamp val between lo and hi"
../target/release/psudo synth --write
../target/release/psudo check -a && ../target/release/psudo build --run
```

## How It Works

Asking a 47 MB model to emit verbose syntax burns half its context window on keywords and punctuation. Wrapping it in tight systems scaffolding fixes that, so that every token goes toward logic instead.

```mermaid
flowchart LR
    A["@stub Spec (psudo.bin)"] --> B["pseudo-engine (100M TQ2_0)"]
    B --> C["PirPdaConstraint (pIR Mask)"]
    C --> D["Borrow Checker & Sandbox"]
    D --> E["Safe Rust (0 unwrap)"]
```

- **2-Bit Ternary Inference (`pseudo-engine`):** Weights map directly from disk via `memmap2` and run through pure-Rust `TQ2_0` ternary kernels (`{-1, 0, 1}`) with a 2-tier INT8 KV cache, which removes Python, C++, and CUDA from the stack.
- **Token Compression & PDA Logit Masking:** The model emits a Polish-prefix wire format (`pIR`) that cuts sequence length by `45-60%`, while `PirPdaConstraint` masks out invalid BPE tokens before sampling so that syntax errors drop to `0%`.
- **Isolated Capsule & Safe Lowering:** All source stays inside one binary file (`psudo.bin`) so outer agents stay at the orchestrator level, and verified stubs lower straight into safe Rust with zero `.unwrap()` or `.expect()` calls.

## Before / After Example

An orchestrator declares a `@stub`, the 100M ternary model emits compact `pIR` under the PDA mask, and the transpiler lowers the verified body into safe Rust:

```text
// 1. Input @stub (declared in psudo.bin)
@stub "clamp val between lo and hi"
fn clamp_val(val, lo, hi)

// 2. Wire pIR (emitted by pseudo-100m-tq2_0.gguf under PDA mask)
f clamp_val (val lo hi) [ ? (< val lo) [ r lo ] ? (> val hi) [ r hi ] r val ]
```

```rust
// 3. Lowered Safe Rust (borrow-checked, 0 unwrap)
pub fn clamp_val(val: i64, lo: i64, hi: i64) -> i64 {
    if val < lo { return lo; }
    if val > hi { return hi; }
    val
}
```

## Honest Scope

- **Atomic functions, not whole services:** A 47 MB 2-bit model fits ~15-line single-purpose stubs (guards, clamping, math, simple loops, JSON/KV helpers), not multi-file architectures.
- **Grammar masking guarantees syntax, not reasoning:** `PirPdaConstraint` enforces valid `pIR` structure at decode time, so an out-of-distribution prompt still gives valid syntax with wrong logic until sandbox tests catch it.
- **Wire format, not a general-purpose language:** `.psudo` and `pIR` exist only as token-compression scaffolding around the 47 MB model, for the same reason `psudo.bin` hides raw source so outer agents don't bypass local synthesis.

## Workspace Layout

```text
psudo/
├── pseudo-engine/          # Pure-Rust GGUF parser, TQ2_0 kernels, INT8 KV cache, PDA sampler, --daemon
├── pseudo-lang/            # Lexer, parser, borrow checker, psudo.bin capsule VFS, and psudo CLI
├── pseudo-transpiler/      # Deterministic lowering from verified .psudo to zero-unwrap Rust
├── examples/               # Pre-built capsules (log-incident-filter, sensor-alert-monitor, quota-fee-calculator)
├── templates/              # Starter templates and .pseudo/sandbox/ fixtures for `psudo init`
├── pseudo-100m-tq2_0.gguf  # Primary 100.7M-param 2-bit ternary weights (47.0 MB)
├── pseudo-33m-tq2_0.gguf   # Embedded 33.6M-param 2-bit ternary fallback (23.2 MB)
├── tokenizer.json          # 8,192-token BPE vocabulary
└── demo.gif                # CLI walkthrough capture
```
