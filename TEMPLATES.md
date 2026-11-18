# Templates & The Exercise

On a 4 GB VRAM laptop, you don't ask a `100M` 2-bit ternary model to write a 400-line terminal renderer from scratch. That burns the entire context window on layout boilerplate and ANSI escape codes.

Instead, every template in `psudo` splits the problem in two:

1. **The Exercise (what the `100M` model writes):** Small, 10–15 line pure logic and data-pipeline stubs—bounds checks, Conway's cell rules, Mandelbrot escape loops, budget clamping, and `load_json` -> `persistent_kv` aggregation.
2. **The Addon Builtin Stubs (what the Rust transpiler handles):** Functions like `tui_dashboard`, `tui_grid`, `tui_fractal`, or `tokio_sleep_ms` that turn the `persistent_kv` state produced by those stubs into a native `ratatui` + `crossterm` binary (or `#[tokio::main]` async Rust) with zero `.psudo` syntax changes and zero model retraining.

---

## Available Templates

### Data & CLI Templates
- **`log`** (`log_incident_filter.psudo`): Filters JSON server logs for `5xx` incidents and logs counts into `persistent_kv`.
- **`sensor`** (`sensor_alert_monitor.psudo`): Clamps IoT temperature readings, flags threshold alerts, and stores alert state.
- **`quota`** (`quota_fee_calculator.psudo`): Computes tiered API usage fees with free-tier guards.

### Interactive TUI Templates (`ratatui` + `crossterm`)
- **`life`** (`tui_game_of_life.psudo` — *Conway's Game of Life*):
  - **Model exercise:** `next_cell_state(alive, neighbors)` + `seed_life_grid(path)` loading `.pseudo/sandbox/life_seed.json`.
  - **Run:** `psudo init my-life --template life && psudo build my-life/psudo.bin --run`
- **`fractal`** (`tui_fractal_zoom.psudo` — *2D Mandelbrot Explorer*):
  - **Model exercise:** `mandelbrot_escape(cr, ci, max_iter)` + `init_fractal_viewport(path)` loading `.pseudo/sandbox/fractal_view.json`.
  - **Run:** `psudo init my-fractal --template fractal && psudo build my-fractal/psudo.bin --run`
- **`finance`** (`tui_personal_finance.psudo` — *Personal Finance Dashboard*):
  - **Model exercise:** `clamp_budget_pct(spent, budget)` + `summarize_finances(path, monthly_budget)` aggregating `.pseudo/sandbox/finance.json`.
  - **Run:** `psudo init my-finance --template finance && psudo build my-finance/psudo.bin --run`

---

## Extending with Custom Addon Builtin Stubs (`psudo.toml`)

You can define your own transpiler + daemon addons in `psudo.toml` without changing `.psudo` syntax or retraining the model:

```toml
[addons.tokio]
builtins = ["tokio_sleep_ms"]
cargo_deps = ['tokio = { version = "1", features = ["full"] }']
rust_main_attr = "#[tokio::main]"
rust_main_async = true
```

When a `.psudo` module calls any function listed in `builtins`, `psudo build` injects the Cargo dependencies and Rust attributes (`#[tokio::main] pub async fn main()`) into `.pseudo/build/` automatically.
