#!/usr/bin/env python3
"""
Turing-Complete Prompt Conformance Suite & Scorecard Runner
Parses tests/model_conformance/PROMPT_TESTCASES.md (100 testcases across 10 categories)
and validates release-binary execution across verify-oracles, natural, and guided modes.
"""

import argparse
import json
import re
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import List, Optional, Dict, Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness


@dataclass
class PromptTestCase:
    tc_id: str
    title: str
    category: str
    prompt: str
    signature_hint: str
    test_call_pir: str
    test_call_psudo: str
    expected_stdout: str
    reference_pir: str


def load_prompt_testcases(md_path: Path) -> List[PromptTestCase]:
    content = md_path.read_text(encoding="utf-8")
    sections = re.split(r"\n###\s+", content)
    testcases = []

    for section in sections[1:]:
        lines = section.splitlines()
        header = lines[0]
        id_match = re.search(r"(TC-\d{2}-\d{2})", header)
        if not id_match:
            continue
        tc_id = id_match.group(1)
        title = header.split(":", 1)[1].strip() if ":" in header else header

        category = ""
        prompt = ""
        signature_hint = ""
        test_call_pir = ""
        test_call_psudo = ""
        expected_stdout = ""
        reference_pir = ""

        for line in lines:
            if "- **Category**" in line:
                m = re.search(r"`(CAT-\d{2})`", line)
                if m:
                    category = m.group(1)
            elif "- **Prompt**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    prompt = m.group(1)
            elif "- **Signature Hint**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    signature_hint = m.group(1)
            elif "- **Test Call (pIR)**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    test_call_pir = m.group(1)
            elif "- **Test Call (.psudo)**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    test_call_psudo = m.group(1)
            elif "- **Expected Stdout**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    expected_stdout = m.group(1)

        # Extract reference pIR code block
        pir_block_match = re.search(r"```pir\s*(.*?)```", section, re.DOTALL)
        if pir_block_match:
            reference_pir = pir_block_match.group(1).strip()

        if not category:
            category = f"CAT-{tc_id.split('-')[1]}"

        testcases.append(
            PromptTestCase(
                tc_id=tc_id,
                title=title,
                category=category,
                prompt=prompt,
                signature_hint=signature_hint,
                test_call_pir=test_call_pir,
                test_call_psudo=test_call_psudo,
                expected_stdout=expected_stdout,
                reference_pir=reference_pir,
            )
        )

    categories_found = set(tc.category for tc in testcases)
    if len(categories_found) < 10:
        raise ValueError(f"Expected at least 10 categories, found {len(categories_found)}: {categories_found}")
    if len(testcases) < 100:
        raise ValueError(f"Expected at least 100 testcases, found {len(testcases)}")

    return testcases


def run_turing_suite(mode: str = "full", target_category: Optional[str] = None, target_id: Optional[str] = None, limit: Optional[int] = None):
    harness = ModelConformanceHarness(require_release=True)
    md_path = harness.repo_root / "tests/model_conformance/PROMPT_TESTCASES.md"
    testcases = load_prompt_testcases(md_path)

    if target_category:
        testcases = [tc for tc in testcases if tc.category == target_category]
    if target_id:
        testcases = [tc for tc in testcases if tc.tc_id == target_id]
    if limit:
        testcases = testcases[:limit]

    print("=" * 70)
    print(f"  Turing-Complete Prompt Conformance Suite (Mode: {mode})")
    print(f"  Loaded {len(testcases)} testcases from PROMPT_TESTCASES.md")
    print("=" * 70)

    results = []

    # 1. Verify Oracles Mode
    if mode in ["verify-oracles", "full"]:
        print("\n[*] Running Verify-Oracles Mode (Direct execution of reference pIR & .psudo)...")
        oracle_success = 0
        oracle_total = len(testcases)

        for tc in testcases:
            # Test pIR execution
            code_pir, stdout_pir, _ = harness.execute_pir(tc.reference_pir, test_call=tc.test_call_pir)
            clean_pir_out = stdout_pir.strip().splitlines()[-1] if stdout_pir.strip() else ""

            # Test .psudo execution
            psudo_code = harness.decompile_pir_to_psudo(tc.reference_pir)
            code_psudo, stdout_psudo, _ = harness.execute_psudo(psudo_code, test_call=tc.test_call_psudo)
            clean_psudo_out = stdout_psudo.strip().splitlines()[-1] if stdout_psudo.strip() else ""

            pir_ok = (code_pir == 0 and tc.expected_stdout in clean_pir_out)
            psudo_ok = (code_psudo == 0 and tc.expected_stdout in clean_psudo_out)

            if pir_ok and psudo_ok:
                oracle_success += 1
                status = "\033[92m✓ PASS\033[0m"
            else:
                status = f"\033[91m✗ FAIL (pIR={pir_ok}, psudo={psudo_ok})\033[0m"

            print(f"  [{tc.tc_id}] {tc.title[:45]:<45} -> {status}")
            results.append({
                "tc_id": tc.tc_id,
                "category": tc.category,
                "mode": "verify-oracles",
                "success": pir_ok and psudo_ok,
            })

        print(f"\nOracle Verification: {oracle_success}/{oracle_total} PASSED ({oracle_success/oracle_total*100:.1f}%)")
        if mode == "verify-oracles" and oracle_success < oracle_total:
            sys.exit(1)

    # 2. Natural & Guided LLM Inference Modes
    if mode in ["natural", "guided", "full"]:
        eval_modes = ["natural", "guided"] if mode == "full" else [mode]
        for eval_mode in eval_modes:
            print(f"\n[*] Running LLM Inference Mode: {eval_mode.upper()}...")
            success_count = 0
            total_count = len(testcases)
            tps_list = []
            ttft_list = []

            for tc in testcases:
                prompt = tc.prompt if eval_mode == "natural" else tc.signature_hint
                raw_flag = (eval_mode == "guided")

                try:
                    emitted, metrics = harness.run_engine_inference(prompt, raw_prompt=raw_flag, temp=0.0, max_tokens=96)
                    tps = metrics.get("tps", 0.0)
                    ttft = metrics.get("ttft_ms", 0.0)
                    tps_list.append(tps)
                    ttft_list.append(ttft)

                    full_raw = f"{prompt}\n{emitted}" if raw_flag else emitted
                    extracted = harness.extract_all_pir_definitions(full_raw)

                    # Validate pIR execution
                    code_pir, stdout_pir, _ = harness.execute_pir(extracted, test_call=tc.test_call_pir)
                    clean_pir = stdout_pir.strip().splitlines()[-1] if stdout_pir.strip() else ""
                    match_ok = (code_pir == 0 and tc.expected_stdout in clean_pir)

                    if match_ok:
                        success_count += 1
                        res_status = "\033[92m✓ PASS\033[0m"
                    else:
                        res_status = f"\033[91m✗ FAIL\033[0m"

                    print(f"  [{tc.tc_id}] {tc.title[:40]:<40} | {tps:5.1f} tps | {res_status}")
                except Exception as e:
                    print(f"  [{tc.tc_id}] {tc.title[:40]:<40} | \033[91m✗ ERROR: {e}\033[0m")

            avg_tps = sum(tps_list) / max(1, len(tps_list))
            avg_ttft = sum(ttft_list) / max(1, len(ttft_list))
            print(f"\n{eval_mode.capitalize()} Mode Summary: {success_count}/{total_count} PASSED | Avg TPS: {avg_tps:.1f} | Avg TTFT: {avg_ttft:.1f}ms")

    print("\n" + "=" * 70)
    print("  Turing Conformance Suite Completed Successfully!")
    print("=" * 70)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Turing-Complete Prompt Conformance Runner")
    parser.add_argument("--mode", type=str, default="full", choices=["verify-oracles", "natural", "guided", "full"])
    parser.add_argument("--category", type=str, help="Target category (e.g. CAT-01)")
    parser.add_argument("--id", type=str, help="Target testcase ID (e.g. TC-01-01)")
    parser.add_argument("--limit", type=int, help="Limit number of testcases")
    args = parser.parse_args()

    run_turing_suite(mode=args.mode, target_category=args.category, target_id=args.id, limit=args.limit)
