#!/usr/bin/env python3
"""
Multi-Aspect Prompt Conformance Suite & Scorecard Runner
Parses tests/model_conformance/MODEL_ASPECT_TESTCASES.md (80 testcases across 8 categories)
and validates release-binary execution across talk, fim, auto_heal, zettel, and hybrid_patch aspects.
"""

import argparse
import json
import re
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Tuple, Dict, Any

sys.path.insert(0, str(Path(__file__).resolve().parent))
from conformance_harness import ModelConformanceHarness


@dataclass
class AspectTestCase:
    tc_id: str
    title: str
    category: str
    aspect: str
    prompt: str
    dialect: str
    check_borrows: bool
    required_keywords: List[str]
    test_call_pir: str
    test_call_psudo: str
    expected_stdout: str
    fim_prefix: str = ""
    fim_suffix: str = ""
    corrupted: str = ""
    context_pir: str = ""
    oracle: str = ""


def load_aspect_testcases(md_path: Path) -> List[AspectTestCase]:
    content = md_path.read_text(encoding="utf-8")
    sections = re.split(r"\n###\s+", content)
    testcases = []

    for section in sections[1:]:
        lines = section.splitlines()
        header = lines[0]
        id_match = re.search(r"(MA-\d{2}-\d{2})", header)
        if not id_match:
            continue
        tc_id = id_match.group(1)
        title = header.split(":", 1)[1].strip() if ":" in header else header

        category = ""
        aspect = "talk"
        prompt = ""
        dialect = "prose"
        check_borrows = False
        required_keywords = []
        test_call_pir = ""
        test_call_psudo = ""
        expected_stdout = ""

        for line in lines:
            if "- **Category**" in line:
                m = re.search(r"`(ASP-\d{2})`", line)
                if m:
                    category = m.group(1)
            elif "- **Aspect**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    aspect = m.group(1).strip()
            elif "- **Prompt**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    prompt = m.group(1)
            elif "- **Dialect**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    dialect = m.group(1).strip()
            elif "- **Check Borrows**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    check_borrows = m.group(1).strip().lower() == "true"
            elif "- **Required Keywords**" in line:
                m = re.search(r"`(.*?)`", line)
                if m:
                    required_keywords = [k.strip() for k in m.group(1).split(",") if k.strip()]
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

        # Extract fenced code blocks
        fim_prefix_match = re.search(r"```fim_prefix[^\n]*\n(.*?)```", section, re.DOTALL)
        fim_prefix = fim_prefix_match.group(1) if fim_prefix_match else ""

        fim_suffix_match = re.search(r"```fim_suffix[^\n]*\n(.*?)```", section, re.DOTALL)
        fim_suffix = fim_suffix_match.group(1) if fim_suffix_match else ""

        corrupted_match = re.search(r"```corrupted\s*\n?(.*?)```", section, re.DOTALL)
        corrupted = corrupted_match.group(1).strip() if corrupted_match else ""

        context_match = re.search(r"```context_pir\s*\n?(.*?)```", section, re.DOTALL)
        context_pir = context_match.group(1).strip() if context_match else ""

        oracle_match = re.search(r"```oracle[^\n]*\n(.*?)```", section, re.DOTALL)
        oracle = oracle_match.group(1).strip("\r\n") if oracle_match else ""

        if not category:
            category = f"ASP-{tc_id.split('-')[1]}"

        testcases.append(
            AspectTestCase(
                tc_id=tc_id,
                title=title,
                category=category,
                aspect=aspect,
                prompt=prompt,
                dialect=dialect,
                check_borrows=check_borrows,
                required_keywords=required_keywords,
                test_call_pir=test_call_pir,
                test_call_psudo=test_call_psudo,
                expected_stdout=expected_stdout,
                fim_prefix=fim_prefix,
                fim_suffix=fim_suffix,
                corrupted=corrupted,
                context_pir=context_pir,
                oracle=oracle,
            )
        )

    categories_found = set(tc.category for tc in testcases)
    if len(categories_found) < 8:
        raise ValueError(f"Expected at least 8 categories, found {len(categories_found)}: {categories_found}")
    if len(testcases) < 80:
        raise ValueError(f"Expected at least 80 testcases, found {len(testcases)}")

    return testcases


def verify_single_aspect_oracle(harness: ModelConformanceHarness, tc: AspectTestCase) -> Tuple[bool, str]:
    try:
        if tc.aspect == "talk":
            ok, err = harness.validate_talk_output(tc.oracle, tc.required_keywords)
            return ok, err

        elif tc.aspect == "zettel":
            if tc.dialect == "pir":
                assert "<zettel>" not in tc.oracle and "</zettel>" not in tc.oracle, "Zettel-conditioned pIR oracle must not contain <zettel> tags"
                full_pir = f"{tc.context_pir}\n{tc.oracle}" if tc.context_pir else tc.oracle
                code, stdout, stderr = harness.execute_pir(full_pir, test_call=tc.test_call_pir, check_borrows=tc.check_borrows)
                clean_out = stdout.strip().splitlines()[-1] if stdout.strip() else ""
                if code != 0 or "error" in stderr.lower() or (tc.expected_stdout and tc.expected_stdout not in clean_out):
                    return False, f"Zettel-conditioned pIR execution failed (code {code}):\nStdout: {stdout}\nStderr: {stderr}"
                return True, ""
            else:
                ok, err = harness.validate_zettel_output(tc.oracle, tc.required_keywords)
                return ok, err
        elif tc.aspect == "fim":
            spliced = harness.splice_fim(tc.fim_prefix, tc.oracle, tc.fim_suffix)
            if tc.dialect == "pir":
                code, stdout, stderr = harness.execute_pir(spliced, test_call=tc.test_call_pir, check_borrows=tc.check_borrows)
            else:
                code, stdout, stderr = harness.execute_psudo(spliced, test_call=tc.test_call_psudo, check_borrows=tc.check_borrows)
            clean_out = stdout.strip().splitlines()[-1] if stdout.strip() else ""
            if code != 0 or "error" in stderr.lower() or tc.expected_stdout not in clean_out:
                return False, f"FIM execution failed (code {code}):\nStdout: {stdout}\nStderr: {stderr}"
            return True, ""

        elif tc.aspect == "auto_heal":
            # Phase 1: Negative test on corrupted source
            if tc.dialect == "pir":
                c_code, c_out, c_err = harness.execute_pir(tc.corrupted, test_call=tc.test_call_pir, check_borrows=tc.check_borrows)
            else:
                c_code, c_out, c_err = harness.execute_psudo(tc.corrupted, test_call=tc.test_call_psudo, check_borrows=tc.check_borrows)
            c_clean = c_out.strip().splitlines()[-1] if c_out.strip() else ""
            negative_pass = (c_code != 0 or "error" in c_err.lower() or (tc.expected_stdout and tc.expected_stdout not in c_clean))
            if not negative_pass:
                return False, f"Auto-heal negative check failed: corrupted source unexpectedly passed execution without error."

            # Phase 2: Positive test on oracle (healed) source
            if tc.dialect == "pir":
                p_code, p_out, p_err = harness.execute_pir(tc.oracle, test_call=tc.test_call_pir, check_borrows=tc.check_borrows)
                p_clean = p_out.strip().splitlines()[-1] if p_out.strip() else ""
                if p_code != 0 or "error" in p_err.lower() or (tc.expected_stdout and tc.expected_stdout not in p_clean):
                    return False, f"Auto-heal pIR positive check failed (code {p_code}):\nStdout: {p_out}\nStderr: {p_err}"

                # Also verify decompiled .psudo
                psudo_code = harness.decompile_pir_to_psudo(tc.oracle)
                if tc.test_call_psudo:
                    s_code, s_out, s_err = harness.execute_psudo(psudo_code, test_call=tc.test_call_psudo, check_borrows=tc.check_borrows)
                    s_clean = s_out.strip().splitlines()[-1] if s_out.strip() else ""
                    if s_code != 0 or "error" in s_err.lower() or (tc.expected_stdout and tc.expected_stdout not in s_clean):
                        return False, f"Auto-heal decompiled .psudo positive check failed (code {s_code}):\nStdout: {s_out}\nStderr: {s_err}"
            else:
                s_code, s_out, s_err = harness.execute_psudo(tc.oracle, test_call=tc.test_call_psudo, check_borrows=tc.check_borrows)
                s_clean = s_out.strip().splitlines()[-1] if s_out.strip() else ""
                if s_code != 0 or "error" in s_err.lower() or (tc.expected_stdout and tc.expected_stdout not in s_clean):
                    return False, f"Auto-heal .psudo positive check failed (code {s_code}):\nStdout: {s_out}\nStderr: {s_err}"
            return True, ""

        elif tc.aspect == "hybrid_patch":
            if tc.dialect == "prose":
                return harness.validate_talk_output(tc.oracle, tc.required_keywords)

            talk_part, pir_part = harness.split_hybrid_output(tc.oracle)
            if talk_part:
                ok, err = harness.validate_talk_output(f"<talk>{talk_part}</talk>", [])
                if not ok:
                    return False, f"Hybrid talk validation failed: {err}"

            # Check required keywords in oracle
            lower_oracle = tc.oracle.lower()
            for kw in tc.required_keywords:
                if kw.lower() not in lower_oracle:
                    return False, f"Hybrid patch missing required keyword '{kw}'"

            if pir_part.strip().startswith("- "):
                dec = harness.decompile_pir_to_psudo(pir_part)
                if "CRDT tombstone" not in dec:
                    return False, f"Tombstone patch did not decompile to CRDT tombstone: {dec}"
                return True, ""

            full_pir = f"{tc.context_pir}\n{pir_part}" if tc.context_pir else pir_part
            # Strip patch verb header lines like '+ src/...', '! src/...', '- src/...'
            clean_pir_lines = []
            for line in full_pir.splitlines():
                s = line.strip()
                if s.startswith("+ ") or s.startswith("! ") or s.startswith("- "):
                    continue
                clean_pir_lines.append(line)
            cleaned_pir_code = "\n".join(clean_pir_lines)

            p_code, p_out, p_err = harness.execute_pir(cleaned_pir_code, test_call=tc.test_call_pir, check_borrows=tc.check_borrows)
            p_clean = p_out.strip().splitlines()[-1] if p_out.strip() else ""
            if p_code != 0 or "error" in p_err.lower() or (tc.expected_stdout and tc.expected_stdout not in p_clean):
                return False, f"Hybrid patch pIR execution failed (code {p_code}):\nStdout: {p_out}\nStderr: {p_err}"
            return True, ""

        return False, f"Unknown aspect type '{tc.aspect}'"
    except Exception as e:
        return False, f"Exception during verification of {tc.tc_id}: {e}"


def run_live_channel_probes(harness: ModelConformanceHarness, testcases: List[AspectTestCase]) -> List[Dict[str, Any]]:
    print("\n[*] Running Live Multi-Channel Engine Probes...")
    probes_run = []

    # Pick 1 representative testcase per category ASP-01..ASP-08
    category_map = {}
    for tc in testcases:
        if tc.category not in category_map:
            category_map[tc.category] = tc

    for cat in sorted(category_map.keys()):
        tc = category_map[cat]
        print(f"  -> Probing [{cat}] ({tc.aspect}): '{tc.prompt[:50]}...'")
        raw_prompt = tc.aspect in ["talk", "zettel"]
        try:
            emitted, metrics = harness.run_engine_inference(tc.prompt, raw_prompt=raw_prompt, temp=0.0, max_tokens=64)
            tps = metrics.get("tps", 0.0)
            ttft = metrics.get("ttft_ms", 0.0)

            # Invariants
            assert emitted, "Engine emitted empty text"
            assert "<ctrl_" not in emitted, "Engine leaked control token"

            if tc.aspect != "zettel":
                assert "<zettel>" not in emitted and "</zettel>" not in emitted, f"Model leaked trigger-happy <zettel> on non-zettel channel [{cat}]"

            if tc.aspect == "talk":
                assert "<talk>" in emitted or len(emitted) > 5, "Talk emission invalid"
            elif tc.aspect == "zettel":
                assert "<zettel>" in emitted or "+z" in emitted or "!z" in emitted or "-z" in emitted or "engram:" in emitted or "NOOP" in emitted, "Zettel emission invalid"
            elif tc.aspect == "fim":
                tokens_count = len(emitted.split())
                assert tokens_count <= 64, f"FIM token count {tokens_count} exceeded 64"
            elif tc.aspect == "auto_heal":
                assert "f " in emitted or "r " in emitted, "Auto-heal should emit pIR code"
            probes_run.append({
                "category": cat,
                "tc_id": tc.tc_id,
                "aspect": tc.aspect,
                "tps": tps,
                "ttft_ms": ttft,
                "status": "PASS"
            })
            print(f"     [PASS] TPS: {tps:.1f} | TTFT: {ttft:.1f} ms")
        except Exception as e:
            probes_run.append({
                "category": cat,
                "tc_id": tc.tc_id,
                "aspect": tc.aspect,
                "error": str(e),
                "status": "FAIL"
            })
            print(f"     [FAIL] {e}")

    return probes_run


def run_aspect_suite(mode: str = "full", target_category: Optional[str] = None, target_id: Optional[str] = None, limit: Optional[int] = None):
    harness = ModelConformanceHarness(require_release=True)
    md_path = harness.repo_root / "tests/model_conformance/MODEL_ASPECT_TESTCASES.md"
    testcases = load_aspect_testcases(md_path)

    if target_category:
        testcases = [tc for tc in testcases if tc.category == target_category]
    if target_id:
        testcases = [tc for tc in testcases if tc.tc_id == target_id]
    if limit:
        testcases = testcases[:limit]

    print("=" * 70)
    print(f"  Multi-Aspect Prompt Conformance Suite (Mode: {mode})")
    print(f"  Loaded {len(testcases)} testcases from MODEL_ASPECT_TESTCASES.md")
    print("=" * 70)

    results = []

    if mode in ["verify-oracles", "full"]:
        print("\n[*] Running Aspect Oracle & Trap Verification (80 testcases)...")
        passed = 0
        failed = 0

        for tc in testcases:
            ok, err = verify_single_aspect_oracle(harness, tc)
            if ok:
                passed += 1
                status = "\033[92m✓ PASS\033[0m"
            else:
                failed += 1
                status = f"\033[91m✗ FAIL: {err}\033[0m"
            print(f"[{tc.tc_id}] ({tc.category} - {tc.aspect}) {tc.title} -> {status}")
            results.append({"tc_id": tc.tc_id, "passed": ok, "error": err})

        print("-" * 70)
        print(f"  Aspect Oracle Verification: {passed}/{passed + failed} PASSED ({(passed / (passed + failed) * 100):.1f}%)")
        print("=" * 70)

        if failed > 0:
            print(f"\nFAILURE: {failed} aspect oracles failed verification.")
            sys.exit(1)

    if mode in ["live-channels", "full"]:
        probes = run_live_channel_probes(harness, testcases)
        failed_probes = [p for p in probes if p["status"] == "FAIL"]
        print("=" * 70)
        print(f"  Live Model Channels: {len(probes) - len(failed_probes)}/{len(probes)} PASSED ({(len(probes) - len(failed_probes))/len(probes)*100:.1f}%)")
        print("=" * 70)
        if failed_probes:
            print(f"\n[MODEL CAPABILITY GAP] {len(failed_probes)} live channel probe(s) failed on current model checkpoint:")
            for p in failed_probes:
                print(f"  * {p['category']} ({p['aspect']}): {p.get('error', 'Invalid channel emission')}")
            print("Leaving model failures active to track model improvement in the next distillation iteration.")

    print("\nMulti-Aspect Conformance Suite Completed Successfully!")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Multi-Aspect Prompt Conformance & Evaluation Runner")
    parser.add_argument("--mode", choices=["verify-oracles", "live-channels", "full"], default="full")
    parser.add_argument("--category", type=str, default=None, help="Target category (e.g., ASP-01)")
    parser.add_argument("--id", type=str, default=None, help="Target testcase ID (e.g., MA-01-01)")
    parser.add_argument("--limit", type=int, default=None, help="Limit number of testcases")
    args = parser.parse_args()

    run_aspect_suite(mode=args.mode, target_category=args.category, target_id=args.id, limit=args.limit)
