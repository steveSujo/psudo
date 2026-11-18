# 0. Fundamental Architecture & Division of Responsibility

When working with `psudo`, there is a strict division of labor between the **Outer LLM** (you, the AI developer assistant) and the **Inner Engine** (the local GGUF ternary model `pseudo-100m-tq2_0.gguf` / `pseudo-33m-tq2_0.gguf` running via `pseudo-engine`):
- **Outer LLM**: Acts as the system architect. You define dataflow, specify function signatures, declare natural language `@stub` directives, and author unit test assertions (`test "..."`).
- **Inner Engine**: Acts as the code synthesizer. It materializes pIR token streams under strict pushdown automaton (PDA) grammar masking and auto-heals sandbox diagnostics. **You should never write complex internal algorithmic code by hand when natural language stubs can express them.**

---

# 1. Domain Concepts

- **`capsule` (`psudo.bin`)**: Single-binary workspace container. Holds all project code, virtual files, and configuration. Pass `psudo.bin` as path to CLI commands.
- **`stub` (`@stub`)**: Function declaration with natural language directive:

  ```pseudo
  fn calculate_discount(price, tier)
      @stub("apply 10% discount if tier == 'gold' else 5%")
  ```

  Outer LLM writes stubs; local GGUF engine implements them.

---

# 2. CLI Commands Reference
- `psudo init <DIR> [--template blank|log|sensor|quota|life|fractal|finance] [--with-docs]` -> Create new project workspace (`psudo.bin` + `.pseudo/sandbox/`).

- `psudo contract <CAPSULE|FILE> [--json|--yaml] [--symbol <NAME>] [-o <FILE>]` -> Generate behavioral contracts from .pseudo source files or capsules (token-compressed, no code leakage; includes `params_used`, `branches`, `returns`).
- `psudo vfs show <CAPSULE> [VPATH] -h` -> View function signatures and `@stub` directives in readable syntax (non-`.psudo` files like `psudo.toml` print directly). (Full `.psudo` printing via `--all` or `synth --print` requires `--reveal-code "<answer>"` to the CLI security question.)
- `psudo vfs stub <CAPSULE> <FN> [PARAMS...] --directive "<TEXT>"` -> Add or update function stub in capsule.
- `psudo vfs test [<CAPSULE>] <TEST_NAME> --call "<EXPR>" --expect "<VAL>" [--assert "<COND>"]` -> Declare or update `test "..."` block in capsule.
- `psudo vfs wire [<CAPSULE>] [--entry <FN>] [--call "<STMT>" ...] [--stdin] [--return "<EXPR>"]` -> Wire entrypoint function (`main_task`) to call synthesized functions (use `--stdin` to avoid shell quote escaping).
- `psudo vfs write [<CAPSULE>] <VPATH> <FILE|->` -> Write or update virtual file inside capsule.
- `psudo vfs list <CAPSULE>` -> List virtual files inside capsule.
- `psudo vfs rm [<CAPSULE>] <VPATH>` -> Remove virtual file from capsule.
- `psudo synth <CAPSULE> [--write]` -> Synthesize stubs into working code. Use `--write` to save into capsule.
- `psudo check <CAPSULE> [--all-modules] [-b]` -> Verify syntax, static types, borrow rules, and check all modules for unresolved stubs.
- `psudo test <CAPSULE>` -> Run in-capsule tests (`test "..."` blocks).
- `psudo run <CAPSULE> [--headless] [--call "<EXPR>"]` -> Run script via interpreter (resolves `.pseudo/sandbox/` relative to capsule workspace) or invoke a single expression.
- `psudo build <CAPSULE> --run [--headless]` -> Compile to native binary and execute (use `--headless` or `PSUDO_TUI_HEADLESS=1` for non-interactive ASCII snapshots).
- `psudo benchmark <CAPSULE>` -> Profile compile time and runtime latency.
- `psudo docs [-o <FILE>]` -> Emit CLI guide and domain glossary.
- `psudo -s` -> Print compressed language syntax reference.

## 2.1 TUI Builtins & `[addons.*]` Extensibility
- **TUI Builtins (`ratatui` + `crossterm`):** `tui_metric(label, val)`, `tui_gauge(label, pct)`, `tui_sparkline(label, nums)`, `tui_table(title, rows)`, `tui_log(title, lines)`, `tui_canvas(title, matrix_2d)`, `tui_barchart(title, labels, values)`, `tui_row(widgets)`, `tui_col(widgets)`, `tui_dashboard(title, source)`, `tui_render(title, root)`, `tui_grid(title, source)`, `tui_fractal(title, source)`.
- **Async Timing:** `tokio_sleep_ms(ms)` automatically lowers `fn main()` to `#[tokio::main] pub async fn main()`.
- **Custom Addons (`psudo.toml`):** Declare `[addons.<name>]` with `builtins`, `cargo_deps`, `rust_main_attr`, `rust_main_async`, `rust_file`, and optional `mock_returns` (for interpreter runs). Keep addon `.rs` files UTF-8 without BOM.

---

# 3. Recommended 6-Step Development Loop

1. **Inspect Contracts**: `psudo contract psudo.bin` (see symbols, guards, side effects, and sandbox I/O without code leakage).
2. **Inspect Stubs**: `psudo vfs show psudo.bin -h` (view function signatures and active `@stub` directives).
3. **Declare Stubs**: `psudo vfs stub psudo.bin <name> <params...> --directive "<what it does>"`
4. **Synthesize Code**: `psudo synth psudo.bin --write` (generates implementations).
5. **Verify**: `psudo check psudo.bin --all-modules` (checks types, stubs, and borrows).
6. **Test & Run**: `psudo test psudo.bin` followed by `psudo build psudo.bin --run`.

---

# 4. Iterating on Stubs & Refining Logic

When requirements evolve or logic needs tuning (for example, a constant `x` was `10` and now must be `20`, or a function needs additional validation):
- **Method 1 (VFS Stub Update + Re-Synthesis)**:
  ```bash
  psudo vfs stub psudo.bin calculate_total items --directive "calculate total and apply 20 percent tax instead of 10"
  psudo synth psudo.bin --write
  ```
- **Method 2 (Targeted One-Shot Re-Synthesis)**:
  ```bash
  psudo synth psudo.bin --symbol calculate_total --directive "calculate total and apply 20 percent tax instead of 10" --write
  ```

---

# 5. Anti-Patterns & Common AI Mistakes

- **Anti-Pattern 1: Binary Tampering**: Trying to read, parse, or edit `psudo.bin` directly with Python scripts, regex, or hex/binary editors. `psudo.bin` is a sealed cryptographic VFS capsule. *Fix: Always interact via `psudo vfs`, `psudo synth`, and `psudo check`.*
- **Anti-Pattern 2: Manual Algorithmic Coding Without GGUF**: Manually writing hundreds of lines of complex internal algorithmic code by hand when natural language `@stub` directives can be synthesized instantly via `psudo synth --write`.
- **Anti-Pattern 3: Manual VFS Packing Over-Engineering**: Manually assembling temporary directories and calling `psudo vfs pack` repeatedly instead of directly modifying stubs or editing `.psudo` files.