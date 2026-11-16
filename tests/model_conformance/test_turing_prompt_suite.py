#!/usr/bin/env python3
"""
PyTest Suite: Turing-Complete Prompt Catalog & Release Conformance
"""

import sys
from pathlib import Path
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness
from turing_suite import load_prompt_testcases


@pytest.fixture(scope="module")
def harness():
    return ModelConformanceHarness(require_release=True)


def test_markdown_catalog_has_10_categories_and_100_prompts(harness):
    md_path = harness.repo_root / "tests/model_conformance/PROMPT_TESTCASES.md"
    testcases = load_prompt_testcases(md_path)
    
    assert len(testcases) >= 100, f"Expected >= 100 testcases, got {len(testcases)}"
    
    categories = set(tc.category for tc in testcases)
    assert len(categories) == 10, f"Expected exactly 10 categories, got {len(categories)}: {categories}"

    for cat in [f"CAT-{i:02d}" for i in range(1, 11)]:
        cat_cases = [tc for tc in testcases if tc.category == cat]
        assert len(cat_cases) >= 10, f"Category {cat} has only {len(cat_cases)} testcases (expected >= 10)"


def test_all_100_oracles_pass_in_release_interpreter(harness):
    md_path = harness.repo_root / "tests/model_conformance/PROMPT_TESTCASES.md"
    testcases = load_prompt_testcases(md_path)

    failed = []
    for tc in testcases:
        code_pir, stdout_pir, _ = harness.execute_pir(tc.reference_pir, test_call=tc.test_call_pir)
        clean_pir = stdout_pir.strip().splitlines()[-1] if stdout_pir.strip() else ""

        psudo_code = harness.decompile_pir_to_psudo(tc.reference_pir)
        code_psudo, stdout_psudo, _ = harness.execute_psudo(psudo_code, test_call=tc.test_call_psudo)
        clean_psudo = stdout_psudo.strip().splitlines()[-1] if stdout_psudo.strip() else ""

        if code_pir != 0 or tc.expected_stdout not in clean_pir or code_psudo != 0 or tc.expected_stdout not in clean_psudo:
            failed.append(tc.tc_id)

    assert not failed, f"Oracles failed for testcase IDs: {failed}"


def test_representative_guided_inference_across_categories(harness):
    md_path = harness.repo_root / "tests/model_conformance/PROMPT_TESTCASES.md"
    testcases = load_prompt_testcases(md_path)

    # Pick 1 testcase per category
    sample_cases = []
    for cat in [f"CAT-{i:02d}" for i in range(1, 11)]:
        matches = [tc for tc in testcases if tc.category == cat]
        if matches:
            sample_cases.append(matches[0])

    for tc in sample_cases:
        emitted, metrics = harness.run_engine_inference(tc.signature_hint, raw_prompt=True, temp=0.0, max_tokens=96)
        assert emitted, f"Engine emitted nothing for {tc.tc_id}"
        assert metrics.get("tps", 0.0) >= 45.0, f"TPS too low for {tc.tc_id}: {metrics.get('tps')}"
