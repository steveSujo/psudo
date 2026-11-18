---
name: psudo-orchestrator
description: >
  Guide for using the `psudo` CLI to architect, synthesize, verify, and compile
  `.psudo` programs as an orchestrator. Use when creating `.psudo` apps,
  managing `psudo.bin` capsules, declaring `@stub` directives, running `psudo synth`,
  or debugging `.psudo` build/check failures.
---

# psudo-orchestrator

Act as the **Outer Orchestrator** when building applications with `psudo`. Never hand-code complex internal algorithms when `@stub` synthesis can generate them, and never read or edit `psudo.bin` directly with binary, hex, or ad-hoc scripts.

## 1. Division of Responsibility

- **Outer Orchestrator (You):** Decompose the problem into small, atomic functions, design dataflow and `.pseudo/sandbox/` fixtures, declare signatures and `@stub("...")` directives, wire entrypoint glue and `test "..."` assertions, and run verification/build gates.
- **Inner Engine (`psudo synth`):** Local 2-bit ternary model (`TQ2_0`, `100.7M` primary / `33.6M` fallback) executing under pushdown-automaton (PDA) grammar masking. It implements `@stub` bodies as verified `.psudo` / `pIR`.

Keep stubs **atomic**. `.psudo` and `pIR` are token-compressed wire formats scoped for a 47 MB model, so prefer small single-purpose functions (arithmetic, threshold guards, counters, JSON/KV/file helpers) over monolithic multi-step directives.

---

## 2. Core Orchestrator Loop

### Step 1: Initialize or Inspect Capsule
```bash
# New workspace (templates: blank, log, sensor, quota, life, fractal, finance)
psudo init <DIR> --template <blank|log|sensor|quota|life|fractal|finance> --with-docs

# Inspect behavioral contracts (YAML/JSON, no raw code leakage) and virtual files
psudo contract <CAPSULE|FILE> [--json|--yaml] [--symbol <NAME>] [-o <FILE>]
psudo vfs list <CAPSULE>
psudo vfs show <CAPSULE> [VPATH] -h
```
- `psudo init <DIR> --template blank` creates a minimal `psudo.bin` without extra files.
- Non-blank templates (`log`, `sensor`, `quota`, `life`, `fractal`, `finance`) scaffold `.pseudo/sandbox/` fixtures alongside `psudo.bin`. Non-`.psudo` files like `psudo.toml` print directly with `psudo vfs show`.

### Step 2: Inspect Behavioral Contracts
```bash
psudo contract <CAPSULE|FILE> [--json|--yaml] [--symbol <NAME>] [-o <FILE>]
```
Use `psudo contract` to audit preconditions, guards, mutations, and sandbox I/O without leaking raw implementation bodies.

> **Raw `.psudo` Output Security Gate:** `psudo vfs show --all` and `psudo synth --print` do not print raw `.psudo` code unless the user explicitly provides `--reveal-code "<answer>"` (answering the CLI security question: *"What jumped over the lazy dog?"*). Never attempt to bypass this gate; use `psudo contract` and `psudo vfs show -h` instead.

### Step 3: Declare Atomic Stubs
```bash
psudo vfs stub <CAPSULE> <FN> [PARAMS...] --directive "<concise atomic behavior>"
```
Write directives as concrete, single-responsibility rules (inputs, edge cases, return value). Example:
```bash
psudo vfs stub psudo.bin calculate_fee amount tier --directive "return amount * 0.05 if tier == 'pro' else amount * 0.1"
```

### Step 4: Synthesize and Inspect Bodies
```bash
# Synthesize all unresolved @stub functions and persist into capsule
psudo synth <CAPSULE> --write --print

# Targeted one-shot iteration on a single symbol
psudo synth <CAPSULE> --symbol <FN> --directive "<updated behavior>" --write --print
```
Flags:
- `--write`: persist synthesized code into `psudo.bin`.
- `--print`: print synthesized `.psudo` bodies and resolver diagnostics.
- `--allow-empty`: exit 0 when zero `@stub` symbols remain.
- `--model, -m <PATH>`: specify an explicit GGUF model path.

If synthesis surfaces `<clarify>...</clarify>`, narrow the function signature or make the `--directive` more explicit before retrying.

### Step 5: Wire Glue, Declare Tests, and Verify
Wire `main_task`, declare `test "..."` blocks, or update virtual config files:
```bash
psudo vfs wire <CAPSULE> [--entry <FN>] [--call "<STMT>" ...] [--stdin] [--return "<EXPR>"]
psudo vfs test <CAPSULE> <TEST_NAME> --call "<EXPR>" --expect "<VAL>" [--assert "<COND>"]
psudo vfs write [<CAPSULE>] <VPATH> <FILE|->
psudo vfs rm [<CAPSULE>] <VPATH>
```
Always run static verification across all modules and borrows before running or building:
```bash
psudo check <CAPSULE> -a -b [--json]
```
- `-a`: audit all virtual modules.
- `-b`: enable borrow checking.
- Use `vfs wire --stdin` on Windows PowerShell to avoid shell quote stripping.

### Step 6: Test, Run, and Compile Native Binary
```bash
psudo test <CAPSULE>
psudo run <CAPSULE> [--headless] [--call "<EXPR>"]
psudo build <CAPSULE> [-o <DIR>] [--run] [--headless]
```
- `psudo run` and `psudo build --run` resolve `.pseudo/sandbox/` relative to the capsule's workspace directory, regardless of caller CWD.
- Pass `--headless` (or `PSUDO_TUI_HEADLESS=1`) for deterministic ASCII snapshots of `ratatui` widgets.
- `psudo build` lowers verified `.psudo` to zero-`unwrap()` safe Rust and compiles a native binary.
- Built-in TUI & async widgets: `tui_metric`, `tui_gauge`, `tui_sparkline`, `tui_table`, `tui_log`, `tui_canvas`, `tui_barchart`, `tui_row`, `tui_col`, `tui_dashboard`, `tui_render`, `tui_grid`, `tui_fractal`, `tokio_sleep_ms`.
- Custom Rust addons live in `psudo.toml` (`[addons.<name>]`: `builtins`, `cargo_deps`, `rust_main_attr`, `rust_main_async`, `rust_file`, `mock_returns`; keep `.rs` files UTF-8 without BOM).

---

## 3. Syntax Visibility & Common Diagnostics

Do not print or inline `.psudo` syntax rules automatically. If `.psudo` syntax needs to be displayed (or before running `psudo -s`), ask the user a question first to confirm they want the syntax shown.

### Common Diagnostics (`psudo check`)
- `E001`: Unrecognized character.
- `E002`: Indentation error.
- `E003`: Unclosed string literal.
- `E004`: Unresolved symbol or leftover `@stub`.
- `E005`: Borrow checker conflict (use-after-move on array/map).

---

## 4. Multi-Agent, DAG, and Reference Commands

```bash
# CRDT conflict-free merge of two capsules
psudo vfs merge <CAPSULE_A> <CAPSULE_B> -o <MERGED_CAPSULE>

# Export task dependency DAG
psudo dag export <CAPSULE> --format <mermaid|dot> [-o <FILE>]

# Benchmark and model telemetry
psudo benchmark <CAPSULE>
psudo mcp info [--model <PATH>]

# Export CLI guide
psudo docs [-o <FILE>]
```

---

## 5. Orchestrator Anti-Patterns

1. **Tampering with `psudo.bin`:** Never read, parse, or edit `psudo.bin` directly. Always use `psudo vfs`, `psudo contract`, `psudo synth`, and `psudo check`.
2. **Monolithic stubs:** Never ask a 33M–100M GGUF model to synthesize an entire application in one `@stub`. Split into small functions and compose them.
3. **Skipping `psudo check -a -b`:** Always confirm 0 unresolved stubs and 0 borrow errors before `psudo build --run`.
