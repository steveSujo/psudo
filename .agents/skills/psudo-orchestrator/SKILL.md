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

- **Outer Orchestrator (You):** Decompose the problem into small, atomic toy-language functions, design dataflow and `.pseudo/sandbox/` fixtures, declare signatures and `@stub("...")` directives, wire entrypoint glue and `test "..."` assertions, and run verification/build gates.
- **Inner Engine (`psudo synth`):** Local quantized ternary model (`TQ2_0`, ~33.6M–100.7M) executing under pushdown-automaton (PDA) grammar masking. It implements `@stub` bodies as verified `.psudo` / `pIR`.

Keep stubs **atomic**. `.psudo` is intentionally scoped for a small model: prefer small single-purpose functions (arithmetic, threshold guards, counters, JSON/KV/file helpers) over monolithic multi-step directives.

---

## 2. Core Orchestrator Loop

### Step 1: Initialize or Inspect Capsule
```bash
# New workspace (templates: blank, log, sensor, quota)
psudo init <DIR> --template <blank|log|sensor|quota> --with-docs

# Inspect behavioral contracts (YAML/JSON, no raw code leakage) and virtual files
psudo contract <CAPSULE|FILE> [--json|--yaml] [--symbol <NAME>] [-o <FILE>]
psudo vfs list <CAPSULE>
psudo vfs show <CAPSULE> -h
```
- `psudo init <DIR> --template blank` creates a minimal `psudo.bin` without extra files.
- Non-blank templates (`log`, `sensor`, `quota`) scaffold `.pseudo/sandbox/` fixtures alongside `psudo.bin`.

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

### Step 5: Wire Glue (When Needed) and Verify
If you need to wire entrypoint glue, `test "..."` blocks, or virtual config files:
```bash
psudo vfs write [<CAPSULE>] <VPATH> <FILE|->
psudo vfs rm [<CAPSULE>] <VPATH>
```
Always run static verification across all modules and borrows before running or building:
```bash
psudo check <CAPSULE> -a -b [--json]
```
- `-a`: audit all virtual modules.
- `-b`: enable borrow checking.

### Step 6: Test, Run, and Compile Native Binary
```bash
psudo test <CAPSULE>
psudo run <CAPSULE>
psudo build <CAPSULE> [-o <DIR>] [--run]
```
- `psudo run` and `psudo build --run` resolve `.pseudo/sandbox/` relative to the capsule's workspace directory, regardless of caller CWD.
- `psudo build` lowers verified `.psudo` to zero-`unwrap()` safe Rust and compiles a native binary.

---

## 3. `.psudo` Language Quick Reference

Run `psudo -s` anytime for the compressed syntax card. Key rules:
- **Indentation:** 4 spaces (Python-style blocks). No curly braces for control flow.
- **Comments:** `// line` and `/* block */`. Never use `#` (triggers `E001`).
- **Variables:** `let x = expr` (mutable; reassign with `x = expr`).
- **Control Flow:** `if (cond)` / `if cond` + `else`, `while (cond)`, `for let i = 0` (newline) `i < n` (newline) `i = i + 1`.
- **Ownership:** `num`, `bool`, `str` are `Copy`. Arrays (`[...]`) and maps (`{...}`) are `Move` (assigning `let b = a` moves `a`).
- **Builtins:**
  - Collections & types: `len`, `push`, `pop`, `slice`, `keys`, `values`, `has`, `str`, `num`, `bool`
  - Sandbox FS (confined to `.pseudo/sandbox/`): `readFile`, `writeFile`, `listDir`, `exists`, `isFile`, `isDir`, `mkdir`, `load_json`, `save_json`
  - Persistent KV & rate limiting: `persistent_kv`, `get`, `set`, `has`, `del`, `rate_limiter_sliding_window`, `rate_limiter_token_bucket`, `consume`
  - Timing: `clock`, `time_now_secs`, `time_micros`, `time_nanos`, `sleep_ms`, `benchmark`

### Common Diagnostics (`psudo check`)
- `E001`: Unrecognized character (most often `#` comment instead of `//`).
- `E002`: Indentation error (must be 4-space increments).
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

# Export CLI guide and syntax card
psudo docs [-o <FILE>]
psudo -s
```

---

## 5. Orchestrator Anti-Patterns

1. **Tampering with `psudo.bin`:** Never read, parse, or edit `psudo.bin` directly. Always use `psudo vfs`, `psudo contract`, `psudo synth`, and `psudo check`.
2. **Monolithic stubs:** Never ask a 33M–100M GGUF model to synthesize an entire application in one `@stub`. Split into small functions and compose them.
3. **Skipping `psudo check -a -b`:** Always confirm 0 unresolved stubs and 0 borrow errors before `psudo build --run`.
4. **Wrong comment/indent syntax:** Never write `#` comments or 2-space indents in `.psudo` files.
5. **Repo workspace hygiene (in this repository):** Name temporary CLI test workspaces with a `fibnacci_` prefix (for example `fibnacci_test_workspace/`) and write reports under `reports/`.
