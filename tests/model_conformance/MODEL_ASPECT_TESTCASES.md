# Multi-Aspect Prompt Conformance & Evaluation Catalog (`MODEL_ASPECT_TESTCASES.md`)

> **Purpose:** Canonical 8-category, 80-prompt conformance specification covering non-code model aspects: `<talk>` SVO Caveman explanations & ecosystem architecture tradeoffs, `<fim>` inline prefix/suffix completions, 1-shot `<error>` Auto-Heal (`heal@2` across syntax, arity, resolver, borrow checker `-b`, and sandbox logic traps), `<zettel>` semantic engram pager lifecycle (`+z`, `!z`, `-z`), and Hybrid `<talk>` + `<pir>` symbol-span patches (`+`, `!`, `-`).

---

## ASP-01: `<talk>` Domain Symbol, Function & Borrow-Rule Explanations

### MA-01-01: Explain calculate_total
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask how does calculate_total work?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `calculate_total, subtotal, tax`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `calculate_total`: adds `subtotal` and `tax` to produce order total.
- Parameters: (`subtotal`, `tax`) -> pure numeric addition, zero clone.
- Invariants: deterministic execution in isolated sandbox.
</talk>
```

### MA-01-02: Explain rate_limiter
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask invariants of rate_limiter`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `rate_limiter, limit, bool`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `rate_limiter`: checks request count against `limit` threshold.
- Returns `bool` (`true` below limit, `false` when throttled).
- Invariants: O(1) comparison, zero heap allocation.
</talk>
```

### MA-01-03: Explain mask_token Borrow Rules
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask how does borrow inference handle stats in @redactor::record_line vs line in @redactor::mask_token?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `mask_token, record_line, borrow`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `mask_token`: reads `line` immutably without escape -> shared borrow `&str`.
- `record_line`: mutates `stats` counter in place -> exclusive mutable borrow `&mut`.
</talk>
```

### MA-01-04: Explain clamp_val Boundary Behavior
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask walk through clamp_val boundary guards`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `clamp_val, low, high`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `clamp_val`: bounds input `val` to inclusive `[low, high]` interval.
- Returns `low` when `val < low`, `high` when `val > high`, else `val`.
</talk>
```

### MA-01-05: Explain jwt_validator Freshness Check
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask contract for jwt_validator`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `jwt_validator, exp_ts, now_ts`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `jwt_validator`: verifies token expiration timestamp `now_ts < exp_ts`.
- Rejects expired or equal timestamps deterministically without clock drift.
</talk>
```

### MA-01-06: Explain token_bucket Deduction
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask what does token_bucket do when cost exceeds tokens?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `token_bucket, tokens, cost`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `token_bucket`: deducts `cost` only when `tokens >= cost`.
- Preserves unchanged `tokens` balance on insufficient capacity; never goes negative.
</talk>
```

### MA-01-07: Explain retry_backoff Schedule
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask explain logic for retry_backoff`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `retry_backoff, base_ms, attempt`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `retry_backoff`: scales `base_ms` interval by retry `attempt` multiplier.
- Prevents thundering-herd retries against upstream endpoints.
</talk>
```

### MA-01-08: Explain circuit_breaker Trip Condition
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask when does circuit_breaker trip?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `circuit_breaker, failures, threshold`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `circuit_breaker`: trips open when consecutive `failures >= threshold`.
- Fast-fails downstream calls until health probe resets counter.
</talk>
```

### MA-01-09: Explain StepCounter OOP Dispatch
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask explain StepCounter class method dispatch`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `StepCounter, next_val, step_sz`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `StepCounter`: stateful class exposing `next_val(inst, curr, step_sz)`.
- Computes `curr + step_sz` via bound method call without heap cloning.
</talk>
```

### MA-01-10: Explain Sandboxed Config File Helper
- **Category**: `ASP-01`
- **Aspect**: `talk`
- **Prompt**: `/ask how does write_and_read_cfg handle file paths safely?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `sandbox, writeFile, readFile`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- Confines `writeFile` and `readFile` paths strictly under `.pseudo/sandbox/`.
- Blocks `..` path traversal and absolute host paths before I/O execution.
</talk>
```

---

## ASP-02: `<talk>` Ecosystem Architecture, Quantization & Design Tradeoffs

### MA-02-01: Static Borrow Checker Architecture
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how does borrow checking work?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `BorrowChecker, move, borrow`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `BorrowChecker` enforces static move semantics and single-writer XOR multi-reader borrow rules.
- Rejects use-after-move at verification time with zero runtime lock overhead.
</talk>
```

### MA-02-02: Multi-Tier KV Cache Layout
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how does the 2-tier kv cache work?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `Tier 0, Tier 1, Tier 2`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `Tier 0` (`0..150t`) pins immutable core grammar and `CORE` Zettels.
- `Tier 1` (`150..800t`) caches warm module AST and paged Zettels; `Tier 2` (`800..1200t`) holds ephemeral turn runway.
</talk>
```

### MA-02-03: System-Only pIR Codec Purpose
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask what is pIR and why is it hidden from the editor?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `pIR, Polish, AST`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `pIR` is a token-compressed Polish-prefix intermediate patch codec isomorphic to `.psudo` AST.
- Cuts generation tokens 45-60% under PDA masking while human editors view `.psudo` exclusively.
</talk>
```

### MA-02-04: 1-Shot Auto-Heal Trap Flow
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how does 1-shot auto-heal work?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `error, Tier, repair`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- Verification failure wraps compiler diagnostics in `<error>` and forks warm `Tier 1` KV state in `<1ms`.
- Model emits a single PDA-masked repair patch verified in sandbox before staging diff.
</talk>
```

### MA-02-05: Sandbox Isolation & Mock Fixtures
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how does sandbox security work?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `sandbox, mock, psudo.toml`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- Isolates filesystem operations inside `.pseudo/sandbox/` with explicit read/write allowlists.
- Redirects external `std.http` and `std.sqlite` calls to deterministic `psudo.toml` mock fixtures.
</talk>
```

### MA-02-06: TQ2_0 Ternary Quantization
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask what is TQ2_0 quantization?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `TQ2_0, ternary, scale`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `TQ2_0` packs 1.58-bit ternary weights `{-1, 0, 1}` at 4 weights per byte with per-row gamma scale.
- Executes pure integer sign-addition matmuls without floating-point weight multiplies.
</talk>
```

### MA-02-07: Operational Token Ceiling & Amber Gate
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask what is the token ceiling and amber threshold?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `1200, 800, Zettel`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- Operational KV budget caps at `1200` tokens (`1536` physical window).
- Crossing `800` tokens triggers silent Amber `Zettel` compaction (`!z`) back to Green.
</talk>
```

### MA-02-08: Incremental Span Splicing
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how does AstToPsudo preserve comments during patching?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `AstToPsudo, span, comment`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `AstToPsudo` splices verified patches solely over target `[start_byte..end_byte]` symbol spans.
- Preserves leading `//` doc comments and untouched sibling definitions byte-identical.
</talk>
```

### MA-02-09: Sub-Millisecond Zettel Retrieval
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask how are Zettel engrams ranked and retrieved?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `SQLite, FTS5, 160`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `SQLite` `FTS5` + 1-hop `DEPENDS`/`REFINES`/`TRAP_FIX` edges rank candidate Zettels on CPU in `<0.5ms`.
- Packs top 5-8 engrams within a hard `160` token `Tier 1` sub-budget.
</talk>
```

### MA-02-10: Topological Sort Cycle Tradeoff
- **Category**: `ASP-02`
- **Aspect**: `talk`
- **Prompt**: `/ask Kahn in-degree queue vs current DFS @graph::topo_sort for cycle diagnostics?`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `DFS, Kahn, cycle`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- `DFS` reports exact back-edge `cycle` path in single pass; risks stack depth only above 4k nodes.
- `Kahn` queue avoids recursion but requires an extra pass to reconstruct cycle edges; keep `DFS` for CLI DAGs.
</talk>
```

---

## ASP-03: `<fim>` Function Signatures, Parameter Lists & Inline Call Arguments

### MA-03-01: Complete .psudo 2-Param Signature
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn add_pair(<fim_suffix>)\n    return pa + pb`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `pa, pb`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print add_pair(19, 23)`
- **Expected Stdout**: `42`
```fim_prefix
fn add_pair(
```
```fim_suffix
)
    return pa + pb
```
```oracle
pa, pb
```

### MA-03-02: Complete .psudo 3-Param Signature
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn clamp_num(<fim_suffix>)\n    if val < low\n        return low\n    if val > high\n        return high\n    return val`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `val, low, high`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print clamp_num(140, 0, 100)`
- **Expected Stdout**: `100`
```fim_prefix
fn clamp_num(
```
```fim_suffix
)
    if val < low
        return low
    if val > high
        return high
    return val
```
```oracle
val, low, high
```

### MA-03-03: Complete pIR Function Parameter Tuple
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f calc_checkout (<fim_suffix>) [\n  r (+ subtotal (+ tax ship))\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `subtotal, tax, ship`
- **Test Call (pIR)**: `p (calc_checkout 100 15 8)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `123`
```fim_prefix
f calc_checkout (
```
```fim_suffix
) [
  r (+ subtotal (+ tax ship))
]
```
```oracle
subtotal tax ship
```

### MA-03-04: Complete .psudo Helper Call Arguments
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn mul_add(pa, pb, pc)\n    return pa * pb + pc\n\nfn calc_invoice(qty, unit_px, fee)\n    return mul_add(<fim_suffix>)`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `qty, unit_px, fee`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print calc_invoice(4, 25, 12)`
- **Expected Stdout**: `112`
```fim_prefix
fn mul_add(pa, pb, pc)
    return pa * pb + pc

fn calc_invoice(qty, unit_px, fee)
    return mul_add(
```
```fim_suffix
)
```
```oracle
qty, unit_px, fee
```

### MA-03-05: Complete pIR Nested Arithmetic Arguments
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f rect_perim (width height) [\n  r (* 2 (<fim_suffix>))\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `width, height`
- **Test Call (pIR)**: `p (rect_perim 9 11)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `40`
```fim_prefix
f rect_perim (width height) [
  r (* 2 (
```
```fim_suffix
))
]
```
```oracle
+ width height
```

### MA-03-06: Complete .psudo Object Literal Argument
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn format_tag(meta, count)\n    return meta.env + ":" + str(count)\n\nfn build_prod_tag(total)\n    return format_tag(<fim_suffix>)`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `prod, total`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print build_prod_tag(7)`
- **Expected Stdout**: `prod:7`
```fim_prefix
fn format_tag(meta, count)
    return meta.env + ":" + count

fn build_prod_tag(total)
    return format_tag(
```
```fim_suffix
)
```
```oracle
{env: "prod"}, total
```

### MA-03-07: Complete .psudo String Concatenation Call
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn make_route(ver, slug)\n    return "/api/" + <fim_suffix>`
- **Check Borrows**: `false`
- **Required Keywords**: `ver, slug`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print make_route("v2", "users")`
- **Expected Stdout**: `/api/v2/users`
```fim_prefix
fn make_route(ver, slug)
    return "/api/" + 
```
```fim_suffix

```
```oracle
ver + "/" + slug
```

### MA-03-08: Complete pIR Class Method Call Arguments
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>c Adder [\n  f run_add (inst pa pb) [\n    r (+ pa pb)\n  ]\n]\nf invoke_adder (x_val y_val) [\n  v ad (Adder)\n  r (m ad run_add (<fim_suffix>))\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `x_val, y_val`
- **Test Call (pIR)**: `p (invoke_adder 33 17)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `50`
```fim_prefix
c Adder [
  f run_add (inst pa pb) [
    r (+ pa pb)
  ]
]
f invoke_adder (x_val y_val) [
  v ad (Adder)
  r (m ad run_add (
```
```fim_suffix
))
]
```
```oracle
x_val y_val
```

### MA-03-09: Complete .psudo Array Indexing Expression
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn sum_ends(items)\n    let last_idx = len(items) - 1\n    return <fim_suffix>`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `items, last_idx`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print sum_ends([5, 10, 20, 35])`
- **Expected Stdout**: `40`
```fim_prefix
fn sum_ends(items)
    let last_idx = len(items) - 1
    return items[0] + items[last_idx]
```
```fim_suffix

```
```oracle
items[0] + items[last_idx]
```

### MA-03-10: Complete pIR Comparison Guard Condition
- **Category**: `ASP-03`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f can_spend (bal cost) [\n  ? (<fim_suffix>) [\n    r (- bal cost)\n  ]\n  r bal\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `bal, cost`
- **Test Call (pIR)**: `p (can_spend 100 45)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `55`
```fim_prefix
f can_spend (bal cost) [
  ? (
```
```fim_suffix
) [
    r (- bal cost)
  ]
  r bal
]
```
```oracle
>= bal cost
```

---

## ASP-04: `<fim>` Function Bodies, Loop Accumulators & Sandbox Spans

### MA-04-01: Complete .psudo Arrow Function Expression
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn net_margin(rev, cost) => <fim_suffix>`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `rev, cost`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print net_margin(250, 90)`
- **Expected Stdout**: `160`
```fim_prefix
fn net_margin(rev, cost) => 
```
```fim_suffix

```
```oracle
rev - cost
```

### MA-04-02: Complete .psudo Discount Body
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn disc_price(price, pct)\n    <fim_suffix>\n    return price - cut`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `cut, price, pct`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print disc_price(200, 25)`
- **Expected Stdout**: `150`
```fim_prefix
fn disc_price(price, pct)
    
```
```fim_suffix

    return price - cut
```
```oracle
    let cut = (price * pct) / 100
```
### MA-04-03: Complete pIR While Loop Accumulator Step
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f sum_to (limit_n) [\n  v acc 0\n  v idx 1\n  w (<= idx limit_n) [\n    <fim_suffix>\n    = idx (+ idx 1)\n  ]\n  r acc\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `acc, idx`
- **Test Call (pIR)**: `p (sum_to 8)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `36`
```fim_prefix
f sum_to (limit_n) [
  v acc 0
  v idx 1
  w (<= idx limit_n) [
    
```
```fim_suffix

    = idx (+ idx 1)
  ]
  r acc
]
```
```oracle
= acc (+ acc idx)
```

### MA-04-04: Complete .psudo Recursive Step
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn fact_rec(num_n)\n    if num_n <= 1\n        return 1\n    return <fim_suffix>`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `fact_rec, num_n`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print fact_rec(5)`
- **Expected Stdout**: `120`
```fim_prefix
fn fact_rec(num_n)
    if num_n <= 1
        return 1
    return num_n * fact_rec(num_n - 1)
```
```fim_suffix

```
```oracle
num_n * fact_rec(num_n - 1)
```

### MA-04-05: Complete pIR Absolute Difference Else Branch
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f abs_gap (pa pb) [\n  ? (> pa pb) [\n    r (- pa pb)\n  ] [\n    <fim_suffix>\n  ]\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `pb, pa`
- **Test Call (pIR)**: `p (abs_gap 14 39)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `25`
```fim_prefix
f abs_gap (pa pb) [
  ? (> pa pb) [
    r (- pa pb)
  ] [
    
```
```fim_suffix

  ]
]
```
```oracle
r (- pb pa)
```

### MA-04-06: Complete pIR Sandbox Write-Then-Read
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f roundtrip_note (msg) [\n  @wt "fim_ma04_06.txt" msg\n  r (<fim_suffix>)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `fim_ma04_06.txt`
- **Test Call (pIR)**: `p (roundtrip_note "ready")`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `ready`
```fim_prefix
f roundtrip_note (msg) [
  @wt "fim_ma04_06.txt" msg
  r (
```
```fim_suffix
)
]
```
```oracle
@rd "fim_ma04_06.txt"
```

### MA-04-07: Complete .psudo Safe Division Result Guard
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn safe_div(num_a, div_b)\n    if div_b == 0\n        return <fim_suffix>\n    return ok(num_a / div_b)`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `err, div_zero`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print isError(safe_div(10, 0))`
- **Expected Stdout**: `true`
```fim_prefix
fn safe_div(num_a, div_b)
    if div_b == 0
        return 
```
```fim_suffix

    return ok(num_a / div_b)
```
```oracle
err("div_zero")
```

### MA-04-08: Complete .psudo Power Loop Update
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn pow_int(base_v, exp_v)\n    let acc = 1\n    let step = 0\n    while step < exp_v\n        acc = acc * base_v\n        <fim_suffix>\n    return acc</fim_prefix>`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `acc, base_v`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print pow_int(2, 6)`
- **Expected Stdout**: `64`
```fim_prefix
fn pow_int(base_v, exp_v)
    let acc = 1
    let step = 0
    while step < exp_v
```
```fim_suffix

    return acc
```
```oracle
        acc = acc * base_v
        step = step + 1
```
### MA-04-09: Complete pIR HTTP 2xx Range Check
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>f is_http_2xx (code) [\n  ? (>= code 200) [\n    r (<fim_suffix>)\n  ]\n  r false\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `code, 300`
- **Test Call (pIR)**: `p (is_http_2xx 204)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `true`
```fim_prefix
f is_http_2xx (code) [
  ? (>= code 200) [
    r (
```
```fim_suffix
)
  ]
  r false
]
```
```oracle
< code 300
```

### MA-04-10: Complete .psudo Linear Backoff Formula
- **Category**: `ASP-04`
- **Aspect**: `fim`
- **Prompt**: `<fim_prefix>fn calc_backoff(base_ms, attempt)\n    let mult = attempt * 2\n    return <fim_suffix>`
- **Dialect**: `psudo`
- **Check Borrows**: `false`
- **Required Keywords**: `base_ms, mult`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: `print calc_backoff(150, 3)`
- **Expected Stdout**: `900`
```fim_prefix
fn calc_backoff(base_ms, attempt)
    let mult = attempt * 2
    return 
```
```fim_suffix

```
```oracle
base_ms * mult
```

---

## ASP-05: 1-Shot Auto-Heal (`<error>`): Syntax, Delimiter & Arity Traps (`E_SYNTAX`, `E_ARITY`)

### MA-05-01: Heal add_two Missing Parameter Arity Trap
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/add_two.psudo:1:1] E_ARITY: fn add_two expects 2 args, got 1</error>\nf add_two (pa) [\n  r pa\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `add_two, pa, pb`
- **Test Call (pIR)**: `p (add_two 18 24)`
- **Test Call (.psudo)**: `print add_two(18, 24)`
- **Expected Stdout**: `42`
```corrupted
f add_two (pa) [
  r pa
]
```
```oracle
f add_two (pa pb) [
  v s (+ pa pb)
  r s
]
```

### MA-05-02: Heal calculate_total Missing Tax Parameter
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/calculate_total.psudo:1:1] E_ARITY: fn calculate_total expects 2 args, got 1</error>\nf calculate_total (subtotal) [\n  r 0\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `calculate_total, subtotal, tax`
- **Test Call (pIR)**: `p (calculate_total 200 30)`
- **Test Call (.psudo)**: `print calculate_total(200, 30)`
- **Expected Stdout**: `230`
```corrupted
f calculate_total (subtotal) [
  r 0
]
```
```oracle
f calculate_total (subtotal tax) [
  v total (+ subtotal tax)
  r total
]
```

### MA-05-03: Heal add_three 3-Arg Arity Trap
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/add_three.psudo:1:1] E_ARITY: fn add_three expects 3 args, got 2</error>\nf add_three (pa pb) [\n  r (+ pa pb)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `add_three, pa, pb, pc`
- **Test Call (pIR)**: `p (add_three 10 20 30)`
- **Test Call (.psudo)**: `print add_three(10, 20, 30)`
- **Expected Stdout**: `60`
```corrupted
f add_three (pa pb) [
  r (+ pa pb)
]
```
```oracle
f add_three (pa pb pc) [
  v s1 (+ pa pb)
  v s2 (+ s1 pc)
  r s2
]
```

### MA-05-04: Heal Unclosed Function Bracket E_SYNTAX
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/multiply.psudo:3:1] E_SYNTAX: Expected ']' at end of function block</error>\nf multiply (pa pb) [\n  r (* pa pb)`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `multiply, pa, pb`
- **Test Call (pIR)**: `p (multiply 7 9)`
- **Test Call (.psudo)**: `print multiply(7, 9)`
- **Expected Stdout**: `63`
```corrupted
f multiply (pa pb) [
  r (* pa pb)
```
```oracle
f multiply (pa pb) [
  r (* pa pb)
]
```

### MA-05-05: Heal Unclosed Prefix Expression Paren E_SYNTAX
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/rect_area.psudo:2:18] E_SYNTAX: Expected ')' after arguments</error>\nf compute_perim (width height) [\n  r (* 2 (+ width height)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `compute_perim, width, height`
- **Test Call (pIR)**: `p (compute_perim 8 12)`
- **Test Call (.psudo)**: `print compute_perim(8, 12)`
- **Expected Stdout**: `40`
```corrupted
f compute_perim (width height) [
  r (* 2 (+ width height)
]
```
```oracle
f compute_perim (width height) [
  r (* 2 (+ width height))
]
```

### MA-05-06: Heal Binary Operator Single-Operand Trap
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/ApplyTax.psudo:2:8] E_SYNTAX: Operator '*' expects 2 arguments, got 1</error>\nf double_num (val) [\n  r (* val)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `double_num, val`
- **Test Call (pIR)**: `p (double_num 27)`
- **Test Call (.psudo)**: `print double_num(27)`
- **Expected Stdout**: `54`
```corrupted
f double_num (val) [
  r (* val)
]
```
```oracle
f double_num (val) [
  r (* val 2)
]
```

### MA-05-07: Heal Missing If-Branch Block Bracket
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/rate_limiter.psudo:2:18] E_SYNTAX: Expected '[' for if branch</error>\nf rate_limiter (req limit) [\n  ? (< req limit)\n    r true\n  r false\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `rate_limiter, req, limit`
- **Test Call (pIR)**: `p (rate_limiter 4 10)`
- **Test Call (.psudo)**: `print rate_limiter(4, 10)`
- **Expected Stdout**: `true`
```corrupted
f rate_limiter (req limit) [
  ? (< req limit)
    r true
  r false
]
```
```oracle
f rate_limiter (req limit) [
  ? (< req limit) [
    r true
  ]
  r false
]
```

### MA-05-08: Heal Helper Callsite Arity Trap
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/sync.psudo:5:12] E_ARITY: fn calc_fee expects 2 args, got 1</error>\nf calc_fee (amt pct) [\n  r (/ (* amt pct) 100)\n]\nf total_with_fee (amt) [\n  r (+ amt (calc_fee amt))\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `calc_fee, total_with_fee, amt`
- **Test Call (pIR)**: `p (total_with_fee 200)`
- **Test Call (.psudo)**: `print total_with_fee(200)`
- **Expected Stdout**: `220`
```corrupted
f calc_fee (amt pct) [
  r (/ (* amt pct) 100)
]
f total_with_fee (amt) [
  r (+ amt (calc_fee amt))
]
```
```oracle
f calc_fee (amt pct) [
  r (/ (* amt pct) 100)
]
f total_with_fee (amt) [
  r (+ amt (calc_fee amt 10))
]
```

### MA-05-09: Heal Missing Parameter List Parens E_SYNTAX
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/square.psudo:1:10] E_SYNTAX: Expected '(' after function name</error>\nf square num_x [\n  r (* num_x num_x)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `square, num_x`
- **Test Call (pIR)**: `p (square 11)`
- **Test Call (.psudo)**: `print square(11)`
- **Expected Stdout**: `121`
```corrupted
f square num_x [
  r (* num_x num_x)
]
```
```oracle
f square (num_x) [
  r (* num_x num_x)
]
```

### MA-05-10: Heal clamp_val 3-Arg Signature Trap
- **Category**: `ASP-05`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/clamp.psudo:1:1] E_ARITY: fn clamp_val expects 3 args, got 2</error>\nf clamp_val (val low) [\n  r val\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `clamp_val, val, low, high`
- **Test Call (pIR)**: `p (clamp_val -5 0 100)`
- **Test Call (.psudo)**: `print clamp_val(-5, 0, 100)`
- **Expected Stdout**: `0`
```corrupted
f clamp_val (val low) [
  r val
]
```
```oracle
f clamp_val (val low high) [
  ? (< val low) [
    r low
  ]
  ? (> val high) [
    r high
  ]
  r val
]
```

---

## ASP-06: 1-Shot Auto-Heal (`<error>`): Undefined Symbol, Borrow Checker (`-b`) & Sandbox Logic Traps

### MA-06-01: Heal Undefined Variable Parameter Typo E_RESOLVE
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/tax.psudo:2:15] E_RESOLVE: Undefined variable 'tx_rate'</error>\nf add_tax_amt (base_amt tax_rate) [\n  r (+ base_amt tx_rate)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `add_tax_amt, tax_rate`
- **Test Call (pIR)**: `p (add_tax_amt 100 18)`
- **Test Call (.psudo)**: `print add_tax_amt(100, 18)`
- **Expected Stdout**: `118`
```corrupted
f add_tax_amt (base_amt tax_rate) [
  r (+ base_amt tx_rate)
]
```
```oracle
f add_tax_amt (base_amt tax_rate) [
  r (+ base_amt tax_rate)
]
```

### MA-06-02: Heal Undeclared Accumulator Variable E_RESOLVE
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/sum.psudo:3:9] E_RESOLVE: Undefined variable 'acc_total'</error>\nf sum_up (limit_n) [\n  v idx 1\n  w (<= idx limit_n) [\n    = acc_total (+ acc_total idx)\n    = idx (+ idx 1)\n  ]\n  r acc_total\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `sum_up, acc_total`
- **Test Call (pIR)**: `p (sum_up 5)`
- **Test Call (.psudo)**: `print sum_up(5)`
- **Expected Stdout**: `15`
```corrupted
f sum_up (limit_n) [
  v idx 1
  w (<= idx limit_n) [
    = acc_total (+ acc_total idx)
    = idx (+ idx 1)
  ]
  r acc_total
]
```
```oracle
f sum_up (limit_n) [
  v acc_total 0
  v idx 1
  w (<= idx limit_n) [
    = acc_total (+ acc_total idx)
    = idx (+ idx 1)
  ]
  r acc_total
]
```

### MA-06-03: Heal Array Use-After-Move Borrow Error (`psudo -b`)
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/buf.psudo:3:7] E_BORROW: Borrow check error: Cannot use value after move</error>\nlet pkt = [10, 20, 30]\nlet moved_pkt = pkt\nprint pkt[0]`
- **Dialect**: `psudo`
- **Check Borrows**: `true`
- **Required Keywords**: `moved_pkt, pkt`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `10`
```corrupted
let pkt = [10, 20, 30]
let moved_pkt = pkt
print pkt[0]
```
```oracle
let pkt = [10, 20, 30]
let moved_pkt = pkt
print moved_pkt[0]
```

### MA-06-04: Heal Object Double-Move Borrow Error (`psudo -b`)
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/cfg.psudo:3:13] E_BORROW: Borrow check error: Cannot use value after move</error>\nlet cfg = {port: 8080}\nlet primary_cfg = cfg\nlet backup_cfg = cfg\nprint backup_cfg.port`
- **Dialect**: `psudo`
- **Check Borrows**: `true`
- **Required Keywords**: `cfg, primary_cfg`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `8080`
```corrupted
let cfg = {port: 8080}
let primary_cfg = cfg
let backup_cfg = cfg
print backup_cfg.port
```
```oracle
let cfg = {port: 8080}
let primary_cfg = cfg
print primary_cfg.port
```

### MA-06-05: Heal Class Instance Use-After-Move (`psudo --pir -b`)
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/wallet.psudo:6:3] E_BORROW: Borrow check error: Cannot use value after move</error>\nc BoxVal [\n  f read_v (inst num_x) [\n    r (* num_x 3)\n  ]\n]\nv b1 (BoxVal)\nv b2 b1\nv b3 b1\np (m b3 read_v (9))`
- **Dialect**: `pir`
- **Check Borrows**: `true`
- **Required Keywords**: `BoxVal, read_v`
- **Test Call (pIR)**: `v b1 (BoxVal)\nv b2 b1\np (m b2 read_v (9))`
- **Test Call (.psudo)**: `let b1 = BoxVal()\nlet b2 = b1\nprint b2.read_v(9)`
- **Expected Stdout**: `27`
```corrupted
c BoxVal [
  f read_v (inst num_x) [
    r (* num_x 3)
  ]
]
v b1 (BoxVal)
v b2 b1
v b3 b1
```
```oracle
c BoxVal [
  f read_v (inst num_x) [
    r (* num_x 3)
  ]
]
```

### MA-06-06: Heal Reassign-After-Move Borrow Error (`psudo -b`)
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[src/queue.psudo:3:1] E_BORROW: Borrow check error on assign: Cannot use value after move</error>\nlet batch = [4, 8]\nlet active_batch = batch\nbatch = [12, 16]\nprint active_batch[1]`
- **Dialect**: `psudo`
- **Check Borrows**: `true`
- **Required Keywords**: `batch, active_batch, next_batch`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `20`
```corrupted
let batch = [4, 8]
let active_batch = batch
batch = [12, 16]
print active_batch[1]
```
```oracle
let batch = [4, 8]
let active_batch = batch
let next_batch = [12, 16]
print active_batch[1] + next_batch[0]
```

### MA-06-07: Heal Inverted Discount Operator Sandbox Trap
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[sandbox/test_discount.psudo:4:1] E_SANDBOX: assertion failed: apply_discount(100, 15) expected 85, got 115</error>\nf apply_discount (price disc) [\n  r (+ price disc)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `apply_discount, price, disc`
- **Test Call (pIR)**: `p (apply_discount 100 15)`
- **Test Call (.psudo)**: `print apply_discount(100, 15)`
- **Expected Stdout**: `85`
```corrupted
f apply_discount (price disc) [
  r (+ price disc)
]
```
```oracle
f apply_discount (price disc) [
  r (- price disc)
]
```

### MA-06-08: Heal Off-By-One Loop Bound Sandbox Trap
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[sandbox/test_fact.psudo:6:1] E_SANDBOX: assertion failed: fact_loop(5) expected 120, got 24</error>\nf fact_loop (num_n) [\n  v prod 1\n  v idx 1\n  w (< idx num_n) [\n    = prod (* prod idx)\n    = idx (+ idx 1)\n  ]\n  r prod\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `fact_loop, prod, idx`
- **Test Call (pIR)**: `p (fact_loop 5)`
- **Test Call (.psudo)**: `print fact_loop(5)`
- **Expected Stdout**: `120`
```corrupted
f fact_loop (num_n) [
  v prod 1
  v idx 1
  w (< idx num_n) [
    = prod (* prod idx)
    = idx (+ idx 1)
  ]
  r prod
]
```
```oracle
f fact_loop (num_n) [
  v prod 1
  v idx 1
  w (<= idx num_n) [
    = prod (* prod idx)
    = idx (+ idx 1)
  ]
  r prod
]
```

### MA-06-09: Heal Missing Zero-Divisor Guard Sandbox Trap
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[sandbox/test_ratio.psudo:3:1] E_SANDBOX: safe_ratio(50, 0) expected 0, got inf</error>\nf safe_ratio (num_a div_b) [\n  r (/ num_a div_b)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `safe_ratio, div_b`
- **Test Call (pIR)**: `p (safe_ratio 50 0)`
- **Test Call (.psudo)**: `print safe_ratio(50, 0)`
- **Expected Stdout**: `0`
```corrupted
f safe_ratio (num_a div_b) [
  r (/ num_a div_b)
]
```
```oracle
f safe_ratio (num_a div_b) [
  ? (== div_b 0) [
    r 0
  ]
  r (/ num_a div_b)
]
```

### MA-06-10: Heal Quorum Strict-Greater Off-By-One Trap
- **Category**: `ASP-06`
- **Aspect**: `auto_heal`
- **Prompt**: `<error>[sandbox/test_quorum.psudo:3:1] E_SANDBOX: has_quorum(3, 3) expected true, got false</error>\nf has_quorum (votes needed) [\n  r (> votes needed)\n]`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `has_quorum, votes`
- **Test Call (pIR)**: `p (has_quorum 3 3)`
- **Test Call (.psudo)**: `print has_quorum(3, 3)`
- **Expected Stdout**: `true`
```corrupted
f has_quorum (votes needed) [
  r (> votes needed)
]
```
```oracle
f has_quorum (votes needed) [
  r (>= votes needed)
]
```

---

## ASP-07: `<zettel>` Semantic Engram Pager Lifecycle (`+z`, `!z`, `-z`) & Organic Abstention

### MA-07-01: Organic Concept Creation on Verified Module (`+z CONCEPT`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx></ctx>\n/zettel evaluate ledger::EventLedger verified`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `+z, kind:CONCEPT, ledger`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
+z id:concept_event_wal kind:CONCEPT scope:ledger links:[EventLedger,EventRecord] fact:"Append-only wal replays deterministic state transitions"
</zettel>
```

### MA-07-02: NOOP Abstention — Existing Zettel Already Matches Verified Symbol
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:sym_rate_limiter kind:SYMBOL scope:rate_limiter links:[token_bucket] fact:"rate_limiter enforces req < limit bool guard"</ctx>\n/zettel evaluate rate_limiter verified_unchanged`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `NOOP`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
NOOP
</zettel>
```

### MA-07-03: NOOP Abstention — Trivial Helper Below Usefulness Threshold
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx></ctx>\n/zettel evaluate math::add_two trivial_green_210t`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `NOOP`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
NOOP
</zettel>
```

### MA-07-04: NOOP Abstention — Unverified Code With Active Borrow Error
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx></ctx>\n/zettel evaluate pkt unverified_E_BORROW`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `NOOP`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
NOOP
</zettel>
```

### MA-07-05: Organic Auto-Heal Trap Lesson Creation (`+z EPISODE`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx></ctx>\n/zettel trap E_BORROW pkt verified_heal`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `+z, kind:EPISODE, E_BORROW`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
+z id:trap_move_pkt kind:EPISODE scope:pkt links:[] fact:"E_BORROW fixed: read moved_pkt owner after array move"
</zettel>
```

### MA-07-06: Organic Symbol Contract Delta Update (`!z SYMBOL`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:sym_find_cached_rate kind:SYMBOL scope:store::find_cached_rate links:[RateRow] fact:"queries rate by pair without ttl"</ctx>\n/zettel update store::find_cached_rate verified_delta`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `!z, kind:SYMBOL, store::find_cached_rate`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
!z id:sym_find_cached_rate kind:SYMBOL scope:store::find_cached_rate links:[RateRow] fact:"Added 60000ms max_age_ms TTL filter to find_cached_rate"
</zettel>
```

### MA-07-07: Organic Amber Threshold Context Compaction (`!z EPISODE`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:ep_http_kv_sync kind:EPISODE scope:main::sync_pair links:[store::find_cached_rate] fact:"sync_pair draft"</ctx>\n/zettel compact http_kv_sync amber 842`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `!z, kind:EPISODE, sync_pair`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
!z id:ep_http_kv_sync kind:EPISODE scope:main::sync_pair links:[store::find_cached_rate] fact:"Compacted 842t->348t: 60s TTL filter + sync_pair caller verified"
</zettel>
```

### MA-07-08: Organic Dead Symbol Tombstone (`-z SYMBOL`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:sym_legacy_serial_ids kind:SYMBOL scope:runner::legacy_serial_ids links:[] fact:"legacy id generator"</ctx>\n/zettel tombstone sym_legacy_serial_ids verified_delete`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `-z, sym_legacy_serial_ids`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
-z id:sym_legacy_serial_ids kind:SYMBOL scope:runner::legacy_serial_ids
</zettel>
```

### MA-07-09: Story Merge Episode-to-Concept Consolidation (`+z CONCEPT` + `-z EPISODE`)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:ep_http_kv_sync kind:EPISODE scope:store::find_cached_rate links:[RateRow] fact:"60s TTL verified"</ctx>\n/zettel promote http_kv_sync verified_merge`
- **Dialect**: `zettel`
- **Check Borrows**: `false`
- **Required Keywords**: `+z, kind:CONCEPT, -z, ep_http_kv_sync`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<zettel>
+z id:concept_http_kv_ttl kind:CONCEPT scope:http_kv_sync links:[ep_http_kv_sync] fact:"Story http_kv_sync verified: 60s TTL verified"
-z id:ep_http_kv_sync kind:EPISODE scope:store::find_cached_rate
</zettel>
```

### MA-07-10: Tier-1 Zettel-Conditioned Code Implementation (Zero `<zettel>` Emission)
- **Category**: `ASP-07`
- **Aspect**: `zettel`
- **Prompt**: `<ctx>+z id:concept_http_kv_ttl kind:CONCEPT scope:store links:[is_ttl_valid] fact:"Cached rate valid only when age_ms <= 60000"</ctx>\nimplement sync_rate_ttl(age_ms)`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `sync_rate_ttl, is_ttl_valid`
- **Test Call (pIR)**: `p (sync_rate_ttl 45000)`
- **Test Call (.psudo)**: ``
- **Expected Stdout**: `true`
```context_pir
f is_ttl_valid (age max_age) [
    r (<= age max_age)
]
```
```oracle
f sync_rate_ttl (age_ms) [
    r (is_ttl_valid age_ms 60000)
]
```
---

## ASP-08: Hybrid `<talk>` + `<pir>`, Symbol-Span Patch Verbs (`+`/`!`/`-`) & `@` Context Conditioning

### MA-08-01: Hybrid Explain + Replace Patch (! verb)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `add ttl_ms freshness check to @store::is_cache_fresh and explain behavior`
- **Dialect**: `hybrid`
- **Check Borrows**: `false`
- **Required Keywords**: `ttl_ms, elapsed`
- **Test Call (pIR)**: `p (is_cache_fresh 450 1000)`
- **Test Call (.psudo)**: `print is_cache_fresh(450, 1000)`
- **Expected Stdout**: `true`
```oracle
<talk>
- Added `ttl_ms` freshness comparison returning `true` while `elapsed < ttl_ms`.
</talk>
<pir>
! src/store.psudo is_cache_fresh
f is_cache_fresh (elapsed ttl_ms) [
  r (< elapsed ttl_ms)
]
</pir>
```

### MA-08-02: Hybrid Explain + Insert New Symbol (+ verb)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `insert helper @billing::calc_surcharge adding 5% platform fee and explain`
- **Dialect**: `hybrid`
- **Check Borrows**: `false`
- **Required Keywords**: `calc_surcharge, 5`
- **Test Call (pIR)**: `p (calc_surcharge 200)`
- **Test Call (.psudo)**: `print calc_surcharge(200)`
- **Expected Stdout**: `210`
```oracle
<talk>
- Added `calc_surcharge` computing 5% platform fee via integer division.
</talk>
<pir>
+ src/billing.psudo calc_surcharge
f calc_surcharge (amount) [
  v fee (/ (* amount 5) 100)
  r (+ amount fee)
]
</pir>
```

### MA-08-03: CRDT Symbol Tombstone Delete Patch (- verb)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `/code delete deprecated @runner::legacy_serial_ids via CRDT tombstone`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `- src/runner.psudo legacy_serial_ids`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<pir>
- src/runner.psudo legacy_serial_ids
</pir>
```

### MA-08-04: Context-Conditioned 1-Hop Helper Call (@math::sq_num)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `/code write hypot_sq(pa, pb) using @math::sq_num from context`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `sq_num, hypot_sq`
- **Test Call (pIR)**: `p (hypot_sq 6 8)`
- **Test Call (.psudo)**: `print hypot_sq(6, 8)`
- **Expected Stdout**: `100`
```context_pir
f sq_num (val) [
  r (* val val)
]
```
```oracle
<pir>
+ src/geom.psudo hypot_sq
f hypot_sq (pa pb) [
  r (+ (sq_num pa) (sq_num pb))
]
</pir>
```

### MA-08-05: Context-Conditioned Tax & Shipping Pipeline (@pricing::apply_disc)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `implement checkout_final(price, pct, ship) calling @pricing::apply_disc and explain`
- **Dialect**: `hybrid`
- **Check Borrows**: `false`
- **Required Keywords**: `apply_disc, ship`
- **Test Call (pIR)**: `p (checkout_final 300 20 15)`
- **Test Call (.psudo)**: `print checkout_final(300, 20, 15)`
- **Expected Stdout**: `255`
```context_pir
f apply_disc (price pct) [
  r (- price (/ (* price pct) 100))
]
```
```oracle
<talk>
- Calls `apply_disc(price, pct)` first, then adds flat `ship` fee.
</talk>
<pir>
+ src/checkout.psudo checkout_final
f checkout_final (price pct ship) [
  v discounted (apply_disc price pct)
  r (+ discounted ship)
]
</pir>
```

### MA-08-06: In-Place Span Replacement Patch (! verb on redactor)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `/code update @redactor::has_secret_prefix to check "sk_live_"`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `! src/redactor.psudo has_secret_prefix`
- **Test Call (pIR)**: `p (has_secret_prefix "sk_live_9941")`
- **Expected Stdout**: `false`
```oracle
<pir>
! src/redactor.psudo has_secret_prefix
f has_secret_prefix (tok_str) [
  r (== (slice tok_str 0 8) "sk_live_")
]
</pir>
```

### MA-08-07: Context-Conditioned Class Collaborator (@auth::PolicyGate)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `/code write check_admin_access(role_lvl) instantiating @auth::PolicyGate`
- **Dialect**: `pir`
- **Check Borrows**: `false`
- **Required Keywords**: `PolicyGate, allows`
- **Test Call (pIR)**: `p (check_admin_access 12)`
- **Test Call (.psudo)**: `print check_admin_access(12)`
- **Expected Stdout**: `true`
```context_pir
c PolicyGate [
  f allows (inst user_lvl min_lvl) [
    r (>= user_lvl min_lvl)
  ]
]
```
```oracle
<pir>
+ src/auth_flow.psudo check_admin_access
f check_admin_access (role_lvl) [
  v gate (PolicyGate)
  r (m gate allows (role_lvl 10))
]
</pir>
```

### MA-08-08: Hybrid Sandbox Config Writer + Explanation
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `write save_env_port(port_str) to "ma08_port.cfg" and explain sandbox safety`
- **Dialect**: `hybrid`
- **Check Borrows**: `false`
- **Required Keywords**: `sandbox, ma08_port.cfg`
- **Test Call (pIR)**: `p (save_env_port "9090")`
- **Test Call (.psudo)**: `print save_env_port("9090")`
- **Expected Stdout**: `9090`
```oracle
<talk>
- Writes `port_str` into `ma08_port.cfg` inside isolated `.pseudo/sandbox/` root and echoes stored value.
</talk>
<pir>
+ src/config.psudo save_env_port
f save_env_port (port_str) [
  @wt "ma08_port.cfg" port_str
  r (@rd "ma08_port.cfg")
]
</pir>
```

### MA-08-09: Red Gate Token Ceiling Rejection Guard (> 1200t)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `inline every symbol across all modules exceeding 1200 token ceiling`
- **Dialect**: `prose`
- **Check Borrows**: `false`
- **Required Keywords**: `1200, budget, ceiling`
- **Test Call (pIR)**: ``
- **Test Call (.psudo)**: ``
- **Expected Stdout**: ``
```oracle
<talk>
- Token budget ceiling exceeded: requested context exceeds `1200` operational token limit; prune `@` needles first.
</talk>
```

### MA-08-10: Hybrid Result Monad Validator + Patch (! verb)
- **Category**: `ASP-08`
- **Aspect**: `hybrid_patch`
- **Prompt**: `replace @validator::check_port_range returning Ok(port) for 1024..65535 else Err("bad_port") and explain`
- **Dialect**: `hybrid`
- **Check Borrows**: `false`
- **Required Keywords**: `Ok, err, 1024`
- **Test Call (pIR)**: `p (check_port_range 8080)`
- **Test Call (.psudo)**: `print check_port_range(8080)`
- **Expected Stdout**: `Ok(8080)`
```oracle
<talk>
- Enforces unprivileged TCP port bounds `1024..=65535`, wrapping valid ports in `Ok` and out-of-range inputs in `err`.
</talk>
<pir>
! src/validator.psudo check_port_range
f check_port_range (port_n) [
  ? (< port_n 1024) [
    r (err "bad_port")
  ]
  ? (> port_n 65535) [
    r (err "bad_port")
  ]
  r (ok port_n)
]
</pir>
```
