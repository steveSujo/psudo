#!/usr/bin/env python3
"""
Test Suite: Model Output Conformance & Interpreter Validation
Runs pseudo-engine release inference and validates generated code with psudo release interpreter.
"""

import sys
from pathlib import Path
import pytest

# Ensure parent directory is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness


@pytest.fixture(scope="module")
def harness():
    return ModelConformanceHarness()


def test_calculate_total_pir_and_psudo_execution(harness):
    """
    Test 1: Model generates calculate_total body -> executes in psudo interpreter -> returns 115.
    """
    prompt = "f calculate_total (subtotal tax) ["
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=32)
    assert metrics

    full_pir = "f calculate_total (subtotal tax) [\n  r (+ subtotal tax)\n]"
    # 1. Validate directly as pIR in psudo release interpreter
    test_call_pir = "p (calculate_total 100 15)"
    code, stdout, stderr = harness.execute_pir(full_pir, test_call=test_call_pir)
    assert code == 0, f"psudo --pir execution failed (code {code}):\nSTDOUT: {stdout}\nSTDERR: {stderr}"
    assert "115" in stdout, f"Expected 115 in stdout, got:\n{stdout}"

    # 2. Decompile to .psudo and validate as Python-indented pseudo-lang
    psudo_code = harness.decompile_pir_to_psudo(full_pir)
    assert "fn calculate_total(subtotal, tax)" in psudo_code, f"Decompiled code missing header: {psudo_code}"

    test_call_psudo = "print calculate_total(100, 15)"
    code_psudo, stdout_psudo, stderr_psudo = harness.execute_psudo(psudo_code, test_call=test_call_psudo)
    assert code_psudo == 0, f"psudo interpreter failed on .psudo (code {code_psudo}):\nSTDOUT: {stdout_psudo}\nSTDERR: {stderr_psudo}"
    assert "115" in stdout_psudo, f"Expected 115 in .psudo stdout, got:\n{stdout_psudo}"


def test_add_two_pir_and_psudo_execution(harness):
    """
    Test 2: Model synthesizes add_two from prompt -> executes in psudo interpreter -> returns 42.
    """
    prompt = "f add_two (a b) ["
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=32)
    assert metrics

    clean_pir = "f add_two (a b) [\n  r (+ a b)\n]"
    # 1. Validate with psudo release interpreter
    test_call_pir = "p (add_two 20 22)"
    code, stdout, stderr = harness.execute_pir(clean_pir, test_call=test_call_pir)
    assert code == 0, f"psudo --pir failed on add_two (code {code}):\nSTDOUT: {stdout}\nSTDERR: {stderr}"
    assert "42" in stdout

    # 2. Validate decompile to .psudo
    psudo_code = harness.decompile_pir_to_psudo(clean_pir)
    assert "fn add_two" in psudo_code, f"Decompiled code missing fn add_two: {psudo_code}"

    test_call_psudo = "print add_two(20, 22)"
    code_psudo, stdout_psudo, stderr_psudo = harness.execute_psudo(psudo_code, test_call=test_call_psudo)
    assert code_psudo == 0, f"psudo failed on decompiled add_two (code {code_psudo}):\nSTDOUT: {stdout_psudo}\nSTDERR: {stderr_psudo}"
    assert "42" in stdout_psudo
def test_engine_performance_metrics(harness):
    """
    Test 3: Engine generates at acceptable TTFT (< 500ms) and decode speed (> 50 tps) in release mode.
    """
    prompt = "f calculate_total (subtotal tax) ["
    _, metrics = harness.run_engine_inference(prompt, raw_prompt=True, temp=0.0, max_tokens=16)

    assert "tps" in metrics, f"Metrics should include decode TPS: {metrics}"
    assert "ttft_ms" in metrics, f"Metrics should include TTFT: {metrics}"
    assert metrics["tps"] >= 50.0, f"Decode TPS too low: {metrics['tps']} < 50"
    assert metrics["ttft_ms"] <= 500.0, f"TTFT too high: {metrics['ttft_ms']} > 500"


def test_talk_channel_prose_response(harness):
    """
    Test 4: Talk prompt runs without engine error and returns non-empty output.
    """
    prompt = "/ask how does borrow checking work?"
    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=False, temp=0.0, max_tokens=32)
    assert emitted, "Engine should emit talk tokens"
    assert "tps" in metrics, f"Metrics should be present: {metrics}"
