#!/usr/bin/env python3
"""
CLI Test Runner for Model Conformance & Interpreter Validation Suite
Executes model inference on pseudo-engine release and validates with psudo release.
"""

import sys
import time
from pathlib import Path

# Add script directory to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness


def print_banner(title: str):
    print("\n" + "=" * 65)
    print(f"  {title}")
    print("=" * 65)


def run_suite():
    print_banner("pseudo-engine -> psudo Interpreter Conformance Suite")
    harness = ModelConformanceHarness()
    print(f"[*] Engine Binary:      {harness.engine_bin}")
    print(f"[*] Interpreter Binary: {harness.psudo_bin}")
    print(f"[*] Model GGUF:         {harness.model_path}")

    tests = [
        ("Calculate Total (pIR & .psudo execution)", test_calculate_total),
        ("Add Two (Synthesis & Interpreter check)", test_add_two),
        ("Performance SLA (TTFT < 500ms, TPS >= 50)", test_performance),
        ("Talk Channel Prose Output", test_talk_channel),
    ]

    passed = 0
    failed = 0
    start_total = time.time()

    for name, fn in tests:
        print(f"\n[RUN] {name}...")
        try:
            fn(harness)
            print(f"      \033[92m✓ PASS\033[0m")
            passed += 1
        except Exception as e:
            print(f"      \033[91m✗ FAIL: {e}\033[0m")
            failed += 1

    total_time = time.time() - start_total
    print("\n" + "-" * 65)
    print(f"Summary: {passed} passed, {failed} failed in {total_time:.2f}s")
    print("-" * 65)

    if failed > 0:
        sys.exit(1)
    else:
        print("\033[92mAll model output conformance tests PASSED successfully!\033[0m\n")


def test_calculate_total(harness: ModelConformanceHarness):
    prompt = "f calculate_total (subtotal tax) ["
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=32)
    print(f"      Inference: {metrics.get('tps', 0.0):.1f} tok/s | TTFT: {metrics.get('ttft_ms', 0.0):.1f} ms")
    full_pir = harness.extract_complete_block(f"{prompt}\n{emitted}")
    
    # Check pIR execution
    code, stdout, stderr = harness.execute_pir(full_pir, test_call="p (calculate_total 100 15)")
    if code != 0 or "115" not in stdout:
        raise AssertionError(f"pIR execution failed: code={code}, stdout={stdout}, stderr={stderr}")
    print(f"      pIR execution: verified return 115")

    # Check .psudo decompilation & execution
    psudo_code = harness.decompile_pir_to_psudo(full_pir)
    code_p, stdout_p, stderr_p = harness.execute_psudo(psudo_code, test_call="print calculate_total(100, 15)")
    if code_p != 0 or "115" not in stdout_p:
        raise AssertionError(f".psudo execution failed: code={code_p}, stdout={stdout_p}, stderr={stderr_p}")
    print(f"      .psudo execution: verified return 115")


def test_add_two(harness: ModelConformanceHarness):
    prompt = "f add_two (a b) ["
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=32)
    print(f"      Inference: {metrics.get('tps', 0.0):.1f} tok/s | TTFT: {metrics.get('ttft_ms', 0.0):.1f} ms")
    clean_pir = harness.extract_complete_block(f"{prompt}\n{emitted}")
    code, stdout, stderr = harness.execute_pir(clean_pir, test_call="p (add_two 20 22)")
    if code != 0:
        raise AssertionError(f"pIR add_two failed: code={code}, stderr={stderr}")
    psudo_code = harness.decompile_pir_to_psudo(clean_pir)
    code_p, stdout_p, stderr_p = harness.execute_psudo(psudo_code, test_call="print add_two(20, 22)")
    if code_p != 0:
        raise AssertionError(f".psudo add_two failed: code={code_p}, stderr={stderr_p}")
    print(f"      Interpreter: successfully executed add_two in pIR and .psudo")


def test_performance(harness: ModelConformanceHarness):
    prompt = "f calculate_total (subtotal tax) ["
    _, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=16)
    tps = metrics.get("tps", 0.0)
    ttft = metrics.get("ttft_ms", 999.0)
    print(f"      Measured: {tps:.1f} tok/s, TTFT: {ttft:.1f} ms")
    if tps < 50.0:
        raise AssertionError(f"TPS {tps:.1f} < 50.0")
    if ttft > 500.0:
        raise AssertionError(f"TTFT {ttft:.1f}ms > 500.0ms")


def test_talk_channel(harness: ModelConformanceHarness):
    prompt = "/ask how does borrow checking work?"
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=False, temp=0.0, max_tokens=32)
    if not emitted:
        raise AssertionError("Empty emitted talk response")
    print(f"      Prose output verified ({len(emitted)} chars, {metrics.get('tps', 0.0):.1f} tok/s)")


if __name__ == "__main__":
    run_suite()
