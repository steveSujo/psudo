# psudo

`psudo` is an experimental toy programming language (`.psudo`), a compact intermediate representation (`pIR`), a safe Rust transpiler, and a pure-Rust 1.58-bit ternary (`TQ2_0`) inference engine.

---

## Why I built this

The main reason for this project was to explore an idea that had been bugging me for quite some time: **how do I save my token usage with cloud LLM providers while coding, and actually run a useful coding model locally on my own laptop?**

My daily machine has a dedicated **NVIDIA GTX 1660 Ti laptop GPU with 4 GB of VRAM**. Running a standard 7B–14B coding model locally on 4 GB VRAM while keeping an IDE, browser, and compiler open is a frustrating experience—you either run out of VRAM, page into system RAM, or watch tokens crawl. At the same time, using frontier cloud LLMs to write every single loop and helper function burns through token quotas fast.

So I wondered: *what if I separated architectural planning from low-level code implementation, and delegated the actual function bodies to a tiny local model?*

The project evolved in four steps:

1. **A minimal surface language (`.psudo`):** First, I wrote a small, Python-indented programming language with almost zero boilerplate tokens.
2. **Piggybacking on Rust (`pseudo-transpiler`):** A brand-new toy language has zero ecosystem maturity. To make `.psudo` actually useful for building real CLI tools, I wrote a transpiler that lowers verified, borrow-checked `.psudo` code into safe Rust (with zero `.unwrap()` or `.expect()` calls) and compiles it into native binaries via `cargo`.
3. **The Cactus & Needle spark:** Then I came across [Cactus](https://github.com/cactus-compute/cactus) and [Needle](https://github.com/cactus-compute/needle) ([Hugging Face](https://huggingface.co/Cactus-Compute/needle3))—specifically their 2-bit quantized on-device agentic models (`29M–121M` parameters in a single `8–29 MB` binary) that trade away general chat ability to beat models 10x their size on tool calling and structured extraction using grammar constraints.
4. **"What if?":** What if I trained a tiny `100M`-parameter ternary model (`TQ2_0`, 1.58-bit weights) whose *only* job is to synthesize `.psudo` function stubs locally, while I (or my main cloud LLM) act strictly as the orchestrator?

This repository is the artifact of that endeavor.

---

## How it works (`pIR`, the 100M Model & Distillation)

Instead of asking a small model to generate verbose surface code, the model is trained to emit **pIR (Pseudo Intermediate Representation)**—a compact Polish-prefix / S-expression variant of `.psudo`.

For example, `fn` becomes `f`, `let` becomes `v`, `return` becomes `r`, `if` becomes `?`, and `while` becomes `w`:

```text
// Surface .psudo (what humans read)
fn clamp_val(val, lo, hi)
    if val < lo
        return lo
    if val > hi
        return hi
    return val

// Wire pIR (what the 100M model emits)
f clamp_val (val lo hi) [
  ? (< val lo) [ r lo ]
  ? (> val hi) [ r hi ]
  r val
]
```

Trading away human readability in `pIR` buys two huge advantages for a tiny model:
- **45–60% fewer tokens** per function body.
- **Pushdown Automaton (PDA) Grammar Masking:** Because `pIR` is a strict prefix S-expression grammar over an `8,192`-token BPE vocabulary, `pseudo-engine` runs a stack-allocated PDA (`PirPdaConstraint`) that masks out any syntactically invalid token *before* sampling. The model cannot emit a syntax error.

### The Models (`100M` vs `33M`)

- **`pseudo-100m-tq2_0.gguf` (`100.7M` parameters, `47.0 MB`) — Primary Model:**
  - Architecture: `d_model = 768`, `14` layers, `12` attention heads (`4` KV heads), `FFN = 2048`, `vocab = 8,192`, `max_seq_len = 512`, `TQ2_0` ternary weights (`{-1, 0, 1}`).
  - Built for synthesizing atomic ~15-line `.psudo` functions (guards, arithmetic, loops, JSON/KV helpers).
- **`pseudo-33m-tq2_0.gguf` (`33.6M` parameters, `23.2 MB`) — Rough Test / Fallback Checkpoint:**
  - Architecture: `d_model = 512`, `8` layers, `8` heads (`4` KV heads).
  - My earlier test model. It is still embedded inside the `psudo` binary as a built-in smoke-test fallback, but you should use `pseudo-100m-tq2_0.gguf` for actual synthesis.

### Training & Corpus Specification

I trained the models from scratch on a corpus distilled from **[`Qwen/Qwen2.5-Coder-7B-Instruct-AWQ`](https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-AWQ)** (base: [`Qwen/Qwen2.5-Coder-7B-Instruct`](https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct)) paired with a synthetic generator checked by the compiler:
- **Teacher & Verifier Loop:** Qwen 2.5 Coder 7B (served via vLLM) and the generator produce candidate functions, which are type-checked, borrow-checked, and sandbox-tested through headless `psudo` before being compiled to canonical `pIR` and packed into `uint16` shards.
- **Corpus Breakdown (`~7,800` verified trajectories):**
  - `6,800` `<pir>` synthesis records across 6 buckets: arithmetic with zero-divisor guards, multi-branch conditionals, bounded `while` loops, recursion/helper calls, template primitives, and 15-line JSON/KV pipelines.
  - `450` `<error>` 1-shot auto-heal repair trajectories (compiler error -> fixed `<pir>` patch).
  - `300` `<zettel>` short-term memory notes.
  - `250` `<clarify>` ambiguity records (when a stub directive is underspecified).

---

## Quickstart & Onboarding

### 1. Prerequisites
- **Rust toolchain** (`1.75+`, `cargo` on your `PATH`).
- Clone this repo (no C/C++ compiler or CUDA setup required—`pseudo-engine` is 100% pure Rust using integer-only ternary arithmetic and `memmap2`).

### 2. Build the CLI
```bash
cargo build --release
```
The compiled binary is at `./target/release/psudo` (`target\release\psudo.exe` on Windows).

### 3. Run the Included Example Apps
The `examples/` folder ships three pre-built workspace capsules (`psudo.bin`) with mock sandbox data in `.pseudo/sandbox/`:

```bash
# 1. JSON Server Log Incident Filter
./target/release/psudo check examples/log-incident-filter/psudo.bin -a -b
./target/release/psudo build examples/log-incident-filter/psudo.bin --run

# 2. IoT Temperature Sensor Alert Monitor
./target/release/psudo build examples/sensor-alert-monitor/psudo.bin --run

# 3. API Tiered Quota & Billing Calculator
./target/release/psudo build examples/quota-fee-calculator/psudo.bin --run
```

---

## Using `psudo` as an Orchestrator (Step-by-Step Workflow)

In `pseudo`, your code lives inside a single-file binary workspace capsule (`psudo.bin`). You (or your outer LLM agent) act as the **Orchestrator**: you declare function signatures and `@stub("...")` directives, and let `psudo synth` write the implementation.

### Step 1: Initialize a New Workspace
```bash
# Templates available: blank, log, sensor, quota
./target/release/psudo init my-app --template blank --with-docs
```

### Step 2: Inspect Behavioral Contracts & Stubs
To save outer-LLM tokens, `psudo` never dumps raw source code by default. Instead, inspect the behavioral contract (`YAML` or `JSON`) or stub signatures:
```bash
./target/release/psudo contract my-app/psudo.bin
./target/release/psudo vfs show my-app/psudo.bin -h
```

### Step 3: Declare Atomic `@stub` Functions
Break your logic into small, single-purpose functions and add them as stubs:
```bash
./target/release/psudo vfs stub my-app/psudo.bin compute_fee units rate free_tier \
  --directive "if units <= free_tier return 0, else return (units - free_tier) * rate"
```

### Step 4: Synthesize with the Local 100M Model
```bash
./target/release/psudo synth my-app/psudo.bin \
  --model ./pseudo-100m-tq2_0.gguf \
  --write
```
Need to tweak a constant or requirement later? Re-synthesize a single symbol in one shot:
```bash
./target/release/psudo synth my-app/psudo.bin \
  --symbol compute_fee \
  --directive "if units <= free_tier return 0, else return (units - free_tier) * rate + 10" \
  --model ./pseudo-100m-tq2_0.gguf \
  --write
```

### Step 5: Verify Borrows & Compile to Native Rust
```bash
# Static type check, borrow check (-b), and unresolved stub audit (-a)
./target/release/psudo check my-app/psudo.bin -a -b

# Transpile to zero-unwrap() safe Rust, compile via cargo, and run
./target/release/psudo build my-app/psudo.bin --run
```

### CLI Reference & Easter Egg Security Key
- Run `psudo --help` or `psudo docs` for the full command list, and `psudo -s` for the compressed `.psudo` syntax card.
- **Why is raw `.psudo` code locked?** When I tested outer LLM agents (Claude, Cursor, etc.) with this CLI, they kept trying to read the `.psudo` files and hand-write the algorithms themselves instead of delegating to `psudo synth`. So `psudo vfs show --all` and `psudo synth --print` are locked behind a security key prompt:
  - **Question:** *"What jumped over the lazy dog?"*
  - **Passphrase:** `--reveal-code "the slow pink elephant"`
- **Using with AI Coding Agents:** This repo includes [`.agents/skills/psudo-orchestrator/SKILL.md`](.agents/skills/psudo-orchestrator/SKILL.md) and an MCP server (`psudo mcp`) so your coding agent knows how to drive the CLI as an orchestrator out of the box.

---

## Rough Edges, Weird Quirks & Honest Expectations

**This is not a polished product.** It is an exploration of an idea with plenty of rough edges. If you try it out, please keep your expectations calibrated:

1. **It's a 100M ternary model—keep stubs atomic:** Do not ask `@stub` to *"write an HTTP server with JWT auth"*. A `100M` 1.58-bit model only works when you decompose tasks into small toy-language building blocks (bounds checks, arithmetic, clamping, simple array/JSON loops, KV reads/writes).
2. **Grammar masking != semantic intelligence:** The PDA grammar mask forces 100% syntactically valid `pIR`, which means if you give the model an out-of-distribution prompt it doesn't understand, the mask will still force it to close its brackets and emit a valid (but logically useless) fallback expression.
3. **Strict 4-space indentation & `//` comments:** `.psudo` uses Python-style indentation, but the lexer strictly enforces 4-space increments (`E002` on 2 spaces or tabs) and C-style `//` comments (`#` comments will fail with `E001`).
4. **The sealed `psudo.bin` workflow feels weird at first:** Storing source files inside a signed binary capsule (`psudo.bin`) and hiding raw code behind `--reveal-code "the slow pink elephant"` is intentionally awkward for human editing—it exists purely as guardrails to keep outer LLM orchestrators honest.
5. **Bimodal CPU decode speeds:** On CPU, `pseudo-engine --benchmark` can swing wildly between fast runs (`~4,000+ tok/s` when `memmap2` pages are hot) and slow runs (`~50 tok/s` on cold pages or background OS scheduling).
6. **Windows-first quirks:** Most development and testing happened on Windows 11. Rebuilding while `psudo.exe` is running will hit Windows file-lock errors, and Linux/macOS paths have had far less mileage.

Expect bugs! I'll be slowly polishing things over time as I keep experimenting with the language and the local model.

---

## Repository Layout

| Path | Description |
|---|---|
| [`pseudo-lang/`](pseudo-lang/) | `.psudo` lexer, parser, static borrow checker, VFS capsule (`psudo.bin`), contract generator, and `psudo` CLI |
| [`pseudo-engine/`](pseudo-engine/) | Pure-Rust zero-copy GGUF parser, `TQ2_0` ternary matmul kernels, 3-tier KV cache, and PDA grammar sampler |
| [`pseudo-transpiler/`](pseudo-transpiler/) | Lowering pass from verified `.psudo` AST to safe, zero-`unwrap()` Rust |
| [`examples/`](examples/) | Three working template capsules (`log-incident-filter`, `sensor-alert-monitor`, `quota-fee-calculator`) |
| [`templates/`](templates/) | Source `.psudo` templates and `.pseudo/sandbox/` JSON fixtures used by `psudo init --template` |
| [`.agents/skills/`](.agents/skills/) | Agent skill (`psudo-orchestrator`) for LLM harnesses |
