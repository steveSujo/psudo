#!/usr/bin/env python3
"""
Dynamic Model-to-Interpreter Evaluator
Allows arbitrary prompts to be evaluated through pseudo-engine release inference
and validated through psudo release interpreter execution.
"""

import argparse
import re
import sys
import time
from pathlib import Path
from typing import Optional, List, Dict, Any

# Ensure module path is active
sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness
PRESET_PROMPTS = [
    {
        "name": "calculate_total",
        "prompt": "f calculate_total (subtotal tax) [",
        "raw_prompt": True,
        "call_args": "100 15",
        "expected": "115",
        "desc": "Calculates subtotal + tax (100 + 15 = 115)",
    },
    {
        "name": "add_two",
        "prompt": "f add_two (a b) [",
        "raw_prompt": True,
        "call_args": "25 17",
        "expected": "42",
        "desc": "Adds two numbers (25 + 17 = 42)",
    },
    {
        "name": "calculate_total_large",
        "prompt": "f calculate_total (subtotal tax) [",
        "raw_prompt": True,
        "call_args": "250 50",
        "expected": "300",
        "desc": "Calculates 250 + 50 = 300",
    },
    {
        "name": "add_two_large",
        "prompt": "f add_two (a b) [",
        "raw_prompt": True,
        "call_args": "1000 234",
        "expected": "1234",
        "desc": "Adds 1000 + 234 = 1234",
    },
]


def detect_fn_signature(pir_text: str) -> Optional[tuple[str, List[str]]]:
    """Detects function name and argument names from pIR header."""
    for line in pir_text.splitlines():
        line_str = line.strip()
        m = re.match(r"^f\s+([a-zA-Z0-9_]+)\s*\((.*?)\)", line_str)
        if m:
            fn_name = m.group(1)
            params = [p.strip() for p in m.group(2).split() if p.strip()]
            return fn_name, params
    return None


def run_dynamic_test(
    harness: ModelConformanceHarness,
    prompt: str,
    raw_prompt: bool = True,
    temp: float = 0.0,
    max_tokens: int = 48,
    call_args: Optional[str] = None,
    expected: Optional[str] = None,
    verbose: bool = True,
) -> Dict[str, Any]:
    """
    Executes a single dynamic test case:
    1. Runs pseudo-engine release on prompt.
    2. Extracts balanced pIR function.
    3. Executes in psudo release interpreter (--pir mode).
    4. Decompiles to .psudo and executes in psudo release interpreter.
    5. Validates outputs against expected assertions.
    """
    result = {
        "prompt": prompt,
        "raw_prompt": raw_prompt,
        "success": False,
        "metrics": {},
        "pir_output": "",
        "psudo_output": "",
        "pir_exec_ok": False,
        "psudo_exec_ok": False,
        "stdout_pir": "",
        "stdout_psudo": "",
        "error": None,
    }

    if verbose:
        print(f"\n[Dynamic Eval] Prompt: {prompt!r}")

    # 1. Run Engine Inference
    t0 = time.time()
    try:
        emitted, metrics = harness.run_engine_inference(
            prompt, raw_prompt=raw_prompt, temp=temp, max_tokens=max_tokens
        )
    except Exception as e:
        result["error"] = f"Engine inference failed: {e}"
        if verbose:
            print(f"  \033[91m✗ {result['error']}\033[0m")
        return result

    result["metrics"] = metrics
    tps = metrics.get("tps", 0.0)
    ttft = metrics.get("ttft_ms", 0.0)
    if verbose:
        print(f"  Engine: {tps:.1f} tok/s | TTFT: {ttft:.1f} ms | {len(emitted)} chars emitted")

    # Combine prompt with emitted body if raw_prompt
    if raw_prompt:
        full_pir_raw = f"{prompt}\n{emitted}"
    else:
        full_pir_raw = emitted

    balanced_pir = harness.extract_complete_block(full_pir_raw)
    result["pir_output"] = balanced_pir

    # Detect signature
    sig = detect_fn_signature(balanced_pir)
    fn_name = sig[0] if sig else "test_fn"
    params = sig[1] if sig else []

    # Format call arguments
    if call_args:
        args_pir = call_args
        args_psudo = ", ".join(call_args.split())
    else:
        # Generate default numerical arguments based on parameter count
        default_vals = [str((i + 1) * 10) for i in range(len(params) or 2)]
        args_pir = " ".join(default_vals)
        args_psudo = ", ".join(default_vals)

    test_call_pir = f"p ({fn_name} {args_pir})"
    test_call_psudo = f"print {fn_name}({args_psudo})"

    if verbose:
        print(f"  Detected Function: {fn_name}({', '.join(params)})")
        print(f"  Test Call pIR:     {test_call_pir}")

    # 2. Execute via psudo release binary in pIR mode
    code_pir, stdout_pir, stderr_pir = harness.execute_pir(balanced_pir, test_call=test_call_pir)
    result["stdout_pir"] = stdout_pir.strip()
    result["pir_exec_ok"] = (code_pir == 0)

    if code_pir != 0:
        result["error"] = f"psudo --pir failed (code {code_pir}): {stderr_pir.strip()}"
        if verbose:
            print(f"  \033[91m✗ {result['error']}\033[0m")
        return result

    if verbose:
        clean_out = "\n".join([l for l in stdout_pir.splitlines() if not l.startswith("📄")])
        print(f"  psudo --pir stdout: {clean_out.strip()}")

    # 3. Decompile to .psudo and execute in psudo interpreter
    psudo_code = harness.decompile_pir_to_psudo(balanced_pir)
    result["psudo_output"] = psudo_code

    code_psudo, stdout_psudo, stderr_psudo = harness.execute_psudo(psudo_code, test_call=test_call_psudo)
    result["stdout_psudo"] = stdout_psudo.strip()
    result["psudo_exec_ok"] = (code_psudo == 0)

    if code_psudo != 0:
        result["error"] = f"psudo interpreter failed on .psudo (code {code_psudo}): {stderr_psudo.strip()}"
        if verbose:
            print(f"  \033[91m✗ {result['error']}\033[0m")
        return result

    if verbose:
        clean_out_p = "\n".join([l for l in stdout_psudo.splitlines() if not l.startswith("📄")])
        print(f"  psudo .psudo stdout: {clean_out_p.strip()}")

    # 4. Check expected value if specified
    if expected is not None:
        if expected not in stdout_pir and expected not in stdout_psudo:
            result["error"] = f"Expected {expected!r} not found in output (pIR: {stdout_pir!r}, psudo: {stdout_psudo!r})"
            if verbose:
                print(f"  \033[91m✗ {result['error']}\033[0m")
            return result

    result["success"] = True
    if verbose:
        print("  \033[92m✓ PASSED (Engine -> pIR -> .psudo -> Interpreter verified)\033[0m")

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Dynamic Model Inference & Interpreter Conformance Evaluator"
    )
    parser.add_argument("-p", "--prompt", type=str, help="Prompt text to feed into pseudo-engine")
    parser.add_argument("--raw-prompt", action="store_true", help="Pass raw prompt directly without channel routing")
    parser.add_argument("--args", type=str, help="Arguments to pass when invoking the generated function (e.g. '100 15')")
    parser.add_argument("--expected", type=str, help="Expected value in interpreter output")
    parser.add_argument("--temp", type=float, default=0.0, help="Sampling temperature")
    parser.add_argument("--max-tokens", type=int, default=48, help="Max new tokens to generate")
    parser.add_argument("--preset", type=str, choices=[p["name"] for p in PRESET_PROMPTS], help="Run a named preset prompt")
    parser.add_argument("--all-presets", action="store_true", help="Run all dynamic preset test cases")

    args = parser.parse_args()
    harness = ModelConformanceHarness()

    print("=" * 65)
    print("  Dynamic Model-to-Interpreter Evaluator (Release Binaries)")
    print("=" * 65)
    print(f"  Engine:      {harness.engine_bin}")
    print(f"  Interpreter: {harness.psudo_bin}")
    print(f"  Model:       {harness.model_path}")

    if args.all_presets:
        passed = 0
        failed = 0
        for p in PRESET_PROMPTS:
            res = run_dynamic_test(
                harness,
                prompt=p["prompt"],
                raw_prompt=p["raw_prompt"],
                temp=args.temp,
                max_tokens=args.max_tokens,
                call_args=p["call_args"],
                expected=p["expected"],
                verbose=True,
            )
            if res["success"]:
                passed += 1
            else:
                failed += 1

        print("\n" + "-" * 65)
        print(f"Presets Summary: {passed} passed, {failed} failed")
        print("-" * 65)
        sys.exit(0 if failed == 0 else 1)

    elif args.preset:
        preset = next(p for p in PRESET_PROMPTS if p["name"] == args.preset)
        res = run_dynamic_test(
            harness,
            prompt=preset["prompt"],
            raw_prompt=preset["raw_prompt"],
            temp=args.temp,
            max_tokens=args.max_tokens,
            call_args=preset["call_args"],
            expected=preset["expected"],
            verbose=True,
        )
        sys.exit(0 if res["success"] else 1)

    elif args.prompt:
        res = run_dynamic_test(
            harness,
            prompt=args.prompt,
            raw_prompt=args.raw_prompt,
            temp=args.temp,
            max_tokens=args.max_tokens,
            call_args=args.args,
            expected=args.expected,
            verbose=True,
        )
        sys.exit(0 if res["success"] else 1)

    else:
        # Default behavior: run all presets
        print("\nNo custom prompt passed; running dynamic preset suite...")
        passed = 0
        failed = 0
        for p in PRESET_PROMPTS:
            res = run_dynamic_test(
                harness,
                prompt=p["prompt"],
                raw_prompt=p["raw_prompt"],
                temp=args.temp,
                max_tokens=args.max_tokens,
                call_args=p["call_args"],
                expected=p["expected"],
                verbose=True,
            )
            if res["success"]:
                passed += 1
            else:
                failed += 1

        print("\n" + "-" * 65)
        print(f"Summary: {passed} passed, {failed} failed")
        print("-" * 65)
        sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
