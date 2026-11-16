pub fn llm_cli_docs() -> &'static str {
    include_str!("llm_cli_guide.md")
}
//DEPRECATED: for the above fun
// pub fn end_user_llm_agents_md(project_name: &str) -> String {
//     format!(
//         r#"> **WORKSPACE ISOLATION RULE**: This is a standalone end-user `.psudo` application project (`{project_name}`). Ignore all parent-directory compiler source files. You are an application developer writing `.psudo` code using the connected `psudo` MCP tools and `psudo` CLI as a black box.
//
// # AGENTS.md - End-User Application AI Developer Guide
//
// ## Core Purpose
// You are an expert autonomous AI developer authoring applications in `.psudo`, a gradually-typed, Python-indented declarative language. You interact with the `psudo` toolchain via connected Model Context Protocol (MCP) tools (`psudo_context`, `psudo_check`, `psudo_synth`, `psudo_build`, `gguf_generate`, `psudo_ask`, `psudo_fim`, `psudo_zettel`) and CLI commands.
//
// ---
//
// - **Indentation**: 4-space indentation defines code blocks (Python-style). No curly braces `{{}}` required for control flow blocks.
// - **Functions**:
//   ```psudo
//   fn calculate_total(subtotal, tax)
//       return subtotal + tax
//   ```
// - **Control Flow**:
//   - `if cond` / `else`
//   - `while cond`
//   - `for let i = 0` (newline) `i < n` (newline) `i = i + 1`
// - **Collections**:
//   - Arrays: `let items = [10, 20, 30]`, `push(items, 40)`, `items[0]`.
//   - Maps (Objects): `let obj = {{name: "Invoice", total: 150}}`, `obj["total"]`.
// - **Builtins**: `len(x)`, `str(x)`, `num(x)`, `bool(x)`, `clock()`, `push(arr, x)`, `pop(arr)`, `slice(arr, start, end)`, `keys(obj)`, `values(obj)`, `has(obj, key)`, `readFile(path)`, `writeFile(path, content)`, `exists(path)`, `isFile(path)`, `isDir(path)`, `listDir(path)`, `mkdir(path)`.
//
// ---
//
// ## 2. Ownership & Borrow Rules
// - **Copy vs. Move**: Numbers, booleans, and strings are `Copy`. Arrays (`[...]`) and maps (`{{...}}`) follow `Move` semantics (reassigning `let b = a` transfers ownership and invalidates subsequent reads of `a`).
// - **Mutability**: Variables declared with `let` are mutable (`let total = 0`, `total = total + 10`).
//
// ---
//
// ## 3. Black-Box Stub Synthesis (`@stub`)
// When implementing complex business logic, you can write a stub directive inside any function body:
// ```psudo
// fn calculate_tiered_discount(amount, tier)
//     @stub("apply 10% discount if tier == 'gold' else 5%")
// ```
// Call MCP tool `psudo_synth` (`file_path: "src/main.psudo", target_symbol: "calculate_tiered_discount", write: true`) to synthesize and verify the implementation automatically.
//
// ---
//
// ## 4. 5-Step Autonomous Development Loop
// 1. **Context Inspection**: Call MCP `psudo_context` (`file_path: "src/main.psudo"`) to inspect symbols, borrow modes, and dataflow in `<150` tokens.
// 2. **Code Authoring**: Write `.psudo` code and `@stub` hooks in `src/main.psudo`.
// 3. **Stub Synthesis**: Call MCP `psudo_synth` (`write: true`) to resolve `@stub` directives into verified `.psudo`.
// 4. **Static Verification**: Call MCP `psudo_check` (`file_path: "src/main.psudo", check_borrows: true`) to verify static typing and borrow rules.
// 5. **Native Build & Run**: Call MCP `psudo_build` (`file_path: "src/main.psudo", run: true`) to compile into zero-`unwrap()` safe Rust and execute the native binary.
// "#,
//         project_name = project_name
//     )
// }
//
// pub fn end_user_human_guide_md(project_name: &str) -> String {
//     format!(
//         r#"# USER_GUIDE.md - End-User Developer Manual (`{project_name}`)
//
// Welcome to your isolated `.psudo` application project! This workspace is pre-configured with Model Context Protocol (MCP) integration so your AI harness (Claude Code, Cursor, OMP, etc.) can directly author, check, synthesize, and compile your `.psudo` code into production-ready safe Rust binaries.
//
// ---
//
// ## 1. Quick Start
// 1. **Open Workspace**: Open this folder (`{project_name}`) in your AI harness editor.
// 2. **MCP Server**: Your harness will automatically launch `psudo mcp` via `.mcp.json`.
// 3. **Ask your LLM**: Give your AI assistant prompts such as:
//    > *"Analyze src/main.psudo using psudo_context, implement the missing @stub functions with psudo_synth, verify static borrows with psudo_check, and compile and execute the native binary with psudo_build."*
//
// ---
//
// ## 2. CLI Commands
// You can also run toolchain commands directly from your terminal:
// - `psudo context src/main.psudo` — Display token-compressed AST and dataflow digest.
// - `psudo check src/main.psudo -b` — Run static type and borrow analysis.
// - `psudo synth src/main.psudo --write` — Materialize `@stub` directives.
// - `psudo run src/main.psudo` — Interpret script in REPL/file mode.
// - `psudo build src/main.psudo --run` — Transpile to zero-unwrap safe Rust, compile binary via cargo, and execute.
// - `psudo mcp` — Start MCP stdio server.
// "#,
//         project_name = project_name
//     )
// }
//
// DEPRECATED:
// pub fn starter_invoice_psudo() -> &'static str {
//     r#"// Invoice & Sandbox Receipt Processing Application
// fn calculate_subtotal(qty, unit_price)
//     @stub("multiply qty by unit_price")
//
// fn calculate_tax(subtotal, tax_rate)
//     @stub("calculate tax amount as (subtotal * tax_rate) / 100")
//
// fn process_order(items)
//     let total_amount = 0
//     let i = 0
//     while i < len(items)
//         let item = items[i]
//         let sub = calculate_subtotal(item["qty"], item["price"])
//         total_amount = total_amount + sub
//         i = i + 1
//     return total_amount
//
// let order_items = [
//     {item: "Widget A", qty: 3, price: 25},
//     {item: "Widget B", qty: 2, price: 50}
// ]
//
// let gross_total = process_order(order_items)
// let tax = calculate_tax(gross_total, 8)
// let final_total = gross_total + tax
//
// print "=== INVOICE SUMMARY ==="
// print "Gross Total:"
// print gross_total
// print "Tax (8%):"
// print tax
// print "Final Total:"
// print final_total
//
// // Sandbox File I/O demonstration
// writeFile("receipt.txt", "Invoice Processed Successfully. Total: " + str(final_total))
// let saved_receipt = readFile("receipt.txt")
// print "Saved Receipt Content:"
// print saved_receipt
// "#
// }
#[allow(non_snake_case)]
pub fn starter_helloWorld() -> &'static str {
    r#"// Hello World .psudo Application Entrypoint
fn main_task()
    return "Hello, world!"

let msg = main_task()
print msg
"#
}
