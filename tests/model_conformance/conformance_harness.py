#!/usr/bin/env python3
"""
Model Conformance Test Harness
Connects release binary inference (pseudo-engine) with release binary execution (psudo interpreter).
"""

import os
import re
import sys
import subprocess
import tempfile
from pathlib import Path
from typing import Optional, Tuple, List


class ModelConformanceHarness:
    """Harness that drives pseudo-engine and psudo release binaries."""

    def __init__(self, repo_root: Optional[Path] = None, require_release: bool = True):
        self.repo_root = repo_root or self._find_repo_root()
        self.engine_bin = self._locate_binary("pseudo-engine", ["pseudo-engine/target/release"], require_release)
        self.psudo_bin = self._locate_binary("psudo", ["pseudo-lang/target/release"], require_release)
        self.model_path = self._locate_model()

    def _find_repo_root(self) -> Path:
        curr = Path(__file__).resolve().parent
        for _ in range(5):
            if (curr / "pseudo-engine").is_dir() and (curr / "pseudo-lang").is_dir():
                return curr
            curr = curr.parent
        return Path.cwd()

    def _locate_binary(self, name: str, subdirs: list[str], require_release: bool) -> Path:
        exe_ext = ".exe" if sys.platform == "win32" else ""
        if require_release:
            for s in subdirs:
                cand = self.repo_root / s / f"{name}{exe_ext}"
                if cand.is_file():
                    return cand.resolve()
                cand_no_ext = self.repo_root / s / name
                if cand_no_ext.is_file():
                    return cand_no_ext.resolve()
            raise FileNotFoundError(f"Required release binary {name} not found. Run cargo build --release.")

        # Fallback search (release first, then debug)
        all_subdirs = subdirs + [s.replace("release", "debug") for s in subdirs]
        for s in all_subdirs:
            cand = self.repo_root / s / f"{name}{exe_ext}"
            if cand.is_file():
                return cand.resolve()
            cand_no_ext = self.repo_root / s / name
            if cand_no_ext.is_file():
                return cand_no_ext.resolve()

        raise FileNotFoundError(f"Could not locate {name} binary under {self.repo_root}")

    def _locate_model(self) -> Path:
        candidates = [
            self.repo_root / "pseudo-3.2b-tq2_0.gguf",
            self.repo_root / "pseudo-colab" / "models" / "pseudo-3.2b-tq2_0.gguf",
        ]
        for c in candidates:
            if c.is_file():
                return c.resolve()
        raise FileNotFoundError(f"Could not locate pseudo-3.2b-tq2_0.gguf under {self.repo_root}")

    def run_engine_inference(
        self,
        prompt: str,
        raw_prompt: bool = False,
        temp: float = 0.0,
        max_tokens: int = 96,
        timeout: int = 30,
    ) -> Tuple[str, dict]:
        """
        Executes pseudo-engine release binary on prompt and extracts emitted tokens.
        Returns (emitted_text, metrics_dict).
        """
        cmd = [
            str(self.engine_bin),
            "-m",
            str(self.model_path),
            "-p",
            prompt,
            "--temp",
            str(temp),
            "--max-tokens",
            str(max_tokens),
        ]
        if raw_prompt:
            cmd.append("--raw-prompt")
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=self.repo_root)
        if res.returncode != 0:
            raise RuntimeError(f"pseudo-engine failed (code {res.returncode}):\nSTDOUT: {res.stdout}\nSTDERR: {res.stderr}")

        stdout = res.stdout

        # Extract output after "Output:"
        output_match = re.search(r"Output:\s*(.*?)(?:\n\s*\d+\.\d+\s+tps|\Z)", stdout, re.DOTALL)
        emitted = output_match.group(1).strip() if output_match else ""
        if ".psudo Decompiled View" in emitted:
            emitted = emitted.split(".psudo Decompiled View")[0].strip()
        # Extract TPS and TTFT
        metrics = {}
        tps_match = re.search(r"(\d+\.\d+)\s+tps\s*\|\s*TTFT:\s*(\d+\.\d+)\s+ms(?:\s*\|\s*conf:\s*(\d+\.\d+))?", stdout)
        if tps_match:
            metrics["tps"] = float(tps_match.group(1))
            metrics["ttft_ms"] = float(tps_match.group(2))
            if tps_match.group(3):
                metrics["conf"] = float(tps_match.group(3))

        return emitted, metrics

    def execute_pir(self, pir_code: str, test_call: Optional[str] = None, timeout: int = 15, check_borrows: bool = False) -> Tuple[int, str, str]:
        """
        Executes pIR code using psudo release binary with --pir flag.
        """
        full_code = pir_code.strip()
        if test_call:
            formatted_call = test_call.replace("\\n", "\n")
            full_code += f"\n{formatted_call}\n"
        with tempfile.NamedTemporaryFile("w", suffix=".pir", delete=False, encoding="utf-8") as f:
            f.write(full_code)
            tmp_path = Path(f.name)

        try:
            cmd = [str(self.psudo_bin), "--pir"]
            if check_borrows:
                cmd.append("-b")
            cmd.append(str(tmp_path))
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=self.repo_root)
            return res.returncode, res.stdout, res.stderr
        finally:
            if tmp_path.exists():
                tmp_path.unlink()

    def execute_psudo(self, psudo_code: str, test_call: Optional[str] = None, timeout: int = 15, check_borrows: bool = False) -> Tuple[int, str, str]:
        """
        Executes Python-indented .psudo code using psudo release binary.
        """
        full_code = psudo_code.strip()
        if test_call:
            formatted_call = test_call.replace("\\n", "\n")
            full_code += f"\n\n{formatted_call}\n"
        with tempfile.NamedTemporaryFile("w", suffix=".psudo", delete=False, encoding="utf-8") as f:
            f.write(full_code)
            tmp_path = Path(f.name)

        try:
            cmd = [str(self.psudo_bin)]
            if check_borrows:
                cmd.append("-b")
            cmd.append(str(tmp_path))
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, cwd=self.repo_root)
            return res.returncode, res.stdout, res.stderr
        finally:
            if tmp_path.exists():
                tmp_path.unlink()
    @staticmethod
    def extract_complete_block(text: str) -> str:
        """Extracts text up to the balanced closing ']' bracket of the block."""
        res = []
        depth = 0
        started = False
        for line in text.splitlines():
            res.append(line)
            for ch in line:
                if ch == "[":
                    depth += 1
                    started = True
                elif ch == "]":
                    depth -= 1
            if started and depth == 0:
                break
        return "\n".join(res)

    @staticmethod
    def extract_all_pir_definitions(text: str) -> str:
        """Extracts all top-level balanced f [...] and c [...] definitions."""
        lines = text.splitlines()
        blocks = []
        cur_block = []
        depth = 0
        collecting = False

        for line in lines:
            stripped = line.strip()
            if not collecting and (stripped.startswith("f ") or stripped.startswith("c ")):
                collecting = True
                cur_block = [line]
                for ch in line:
                    if ch == "[":
                        depth += 1
                    elif ch == "]":
                        depth -= 1
                if collecting and depth == 0:
                    blocks.append("\n".join(cur_block))
                    collecting = False
                    cur_block = []
                continue

            if collecting:
                cur_block.append(line)
                for ch in line:
                    if ch == "[":
                        depth += 1
                    elif ch == "]":
                        depth -= 1
                if depth == 0:
                    blocks.append("\n".join(cur_block))
                    collecting = False
                    cur_block = []

        if blocks:
            return "\n\n".join(blocks)
        return ModelConformanceHarness.extract_complete_block(text)

    @staticmethod
    def decompile_pir_to_psudo(pir_source: str) -> str:
        """
        Recursive token-stream decompiler converting pIR code into idiomatic Python-indented .psudo code.
        """
        all_tombstone = True
        for line in pir_source.splitlines():
            s = line.strip()
            if not s or s.startswith("<pir>") or s.startswith("</pir>"):
                continue
            if not s.startswith("- "):
                all_tombstone = False
                break
        if all_tombstone:
            tomb_line = next((l.strip() for l in pir_source.splitlines() if l.strip().startswith("- ")), "")
            return f"// CRDT tombstone: {tomb_line}\n"

        clean_lines = []
        for line in pir_source.splitlines():
            s = line.strip()
            if not s or s.startswith("!") or s.startswith("+ ") or s.startswith("- ") or s.startswith("<pir>") or s.startswith("</pir>") or s.startswith("//"):
                continue
            clean_lines.append(line)
        cleaned = "\n".join(clean_lines)
        return ModelConformanceHarness._structural_decompile(cleaned)

    @staticmethod
    def splice_fim(fim_prefix: str, completion: str, fim_suffix: str) -> str:
        clean_comp = completion.replace("<fim>", "").replace("</fim>", "").strip("\r\n")
        prefix = fim_prefix
        tp = prefix.rstrip("\r\n")
        if any(tp.rstrip().endswith(ch) for ch in ["(", "[", "{", ",", "+", "=>", "=", "return"]):
            prefix = tp.rstrip() + (" " if not tp.rstrip().endswith(("(", "[", "{")) else "")
        suffix = fim_suffix
        ts = suffix.lstrip("\r\n")
        if any(ts.startswith(ch) for ch in [")", "]", "}", ","]):
            suffix = ts
        elif suffix.strip() and not suffix.startswith("\n") and not suffix.startswith("\r\n") and "\n" not in clean_comp and not any(tp.rstrip().endswith(ch) for ch in ["(", "[", "{", ",", "+", "=>", "=", "return"]):
            suffix = "\n" + suffix
        return f"{prefix}{clean_comp}{suffix}"

    @staticmethod
    def split_hybrid_output(wire_text: str) -> Tuple[str, str]:
        talk_match = re.search(r"<talk>(.*?)</talk>", wire_text, re.DOTALL)
        pir_match = re.search(r"<pir>(.*?)</pir>", wire_text, re.DOTALL)
        talk_part = talk_match.group(1).strip() if talk_match else ""
        pir_part = pir_match.group(1).strip() if pir_match else wire_text
        return talk_part, pir_part

    @staticmethod
    def validate_talk_output(text: str, required_keywords: List[str]) -> Tuple[bool, str]:
        clean = text.replace("<talk>", "").replace("</talk>", "").strip()
        if not clean:
            return False, "Talk output is empty"
        if "<pir>" in clean or "</pir>" in clean or "! src/" in clean or "<ctrl_" in clean:
            return False, f"Talk output contains forbidden leakage: {clean}"
        lower = clean.lower()
        for kw in required_keywords:
            if kw.lower() not in lower:
                return False, f"Talk output missing required keyword '{kw}': {clean}"
        return True, ""

    @staticmethod
    def validate_zettel_output(text: str, required_keywords: List[str], max_words: int = 40) -> Tuple[bool, str]:
        clean = text.replace("<zettel>", "").replace("</zettel>", "").strip()
        if not clean:
            return False, "Zettel output is empty"
        if "<pir>" in clean or "</pir>" in clean or "<ctrl_" in clean:
            return False, f"Zettel output contains forbidden code/control leakage: {clean}"

        lower = clean.lower()
        is_noop = any(kw.lower() == "noop" for kw in required_keywords)
        if is_noop and (clean.upper() == "NOOP" or "noop" in lower):
            if any(v in clean for v in ["+z", "!z", "-z", "engram:"]):
                return False, f"NOOP zettel output must not contain engram write verbs: {clean}"
            return True, ""

        for line in clean.splitlines():
            l_trimmed = line.strip()
            if not l_trimmed:
                continue
            if not (l_trimmed.startswith("+z") or l_trimmed.startswith("!z") or l_trimmed.startswith("-z") or l_trimmed.startswith("engram:")):
                return False, f"Zettel output line does not start with valid engram verb (+z, !z, -z, engram:): {l_trimmed}"

        words = clean.split()
        if len(words) > max_words * 3:
            return False, f"Zettel output too long ({len(words)} tokens > max {max_words * 3}): {clean}"

        for kw in required_keywords:
            if kw.lower() not in lower:
                return False, f"Zettel output missing required keyword '{kw}': {clean}"
        return True, ""
    @staticmethod
    def _structural_decompile(pir_text: str) -> str:
        lines = [line.rstrip() for line in pir_text.splitlines() if line.strip()]
        out_lines = []
        indent = 0
        i = 0

        while i < len(lines):
            line = lines[i].strip()
            if not line or line in ["<pir>", "</pir>"]:
                i += 1
                continue

            # Class definition: c ClassName [
            c_match = re.match(r"^c\s+([a-zA-Z0-9_]+)\s*\[?$", line)
            if c_match:
                c_name = c_match.group(1)
                out_lines.append(f"{'    ' * indent}class {c_name}")
                indent += 1
                i += 1
                continue

            # Function definition: f name (params) [
            fn_match = re.match(r"^f\s+([a-zA-Z0-9_]+)\s*\((.*?)\)\s*\[?$", line)
            if fn_match:
                name, raw_params = fn_match.groups()
                params = [p.strip() for p in raw_params.split() if p.strip()]
                out_lines.append(f"{'    ' * indent}fn {name}({', '.join(params)})")
                indent += 1
                i += 1
                continue

            # If statement: ? cond [
            if_match = re.match(r"^\?\s+(.+?)\s*\[$", line)
            if if_match:
                cond = ModelConformanceHarness._convert_polish_expr(if_match.group(1))
                out_lines.append(f"{'    ' * indent}if {cond}")
                indent += 1
                i += 1
                # Check if there is an else block following this
                continue

            if line == "] [":
                indent = max(0, indent - 1)
                out_lines.append(f"{'    ' * indent}else")
                indent += 1
                i += 1
                continue
            elif line == "]":
                indent = max(0, indent - 1)
                i += 1
                while i < len(lines) and not lines[i].strip():
                    i += 1
                if i < len(lines) and lines[i].strip() == "[":
                    while i < len(lines) and not lines[i].strip():
                        i += 1
                    i += 1 # consume [
                    out_lines.append(f"{'    ' * indent}else")
                    indent += 1
                continue
            # Variable assignment: v name expr
            var_match = re.match(r"^v\s+([a-zA-Z0-9_]+)\s+(.+)$", line)
            if var_match:
                name, expr = var_match.groups()
                psudo_expr = ModelConformanceHarness._convert_polish_expr(expr)
                out_lines.append(f"{'    ' * indent}let {name} = {psudo_expr}")
                i += 1
                continue

            # Reassignment: = name expr
            assign_match = re.matchr = re.match(r"^=\s+([a-zA-Z0-9_]+)\s+(.+)$", line) if hasattr(re, 'match') else None
            # Fallback regex for assignment
            assign_match = re.match(r"^=\s+([a-zA-Z0-9_]+)\s+(.+)$", line)
            if assign_match:
                name, expr = assign_match.groups()
                psudo_expr = ModelConformanceHarness._convert_polish_expr(expr)
                out_lines.append(f"{'    ' * indent}{name} = {psudo_expr}")
                i += 1
                continue

            # While loop: w cond [
            while_match = re.match(r"^w\s+(.+?)\s*\[$", line)
            if while_match:
                cond = ModelConformanceHarness._convert_polish_expr(while_match.group(1))
                out_lines.append(f"{'    ' * indent}while {cond}")
                indent += 1
                i += 1
                continue

            # Return statement: r expr
            ret_match = re.match(r"^r\s*(.*)$", line)
            if ret_match:
                expr = ret_match.group(1).strip()
                if expr:
                    psudo_expr = ModelConformanceHarness._convert_polish_expr(expr)
                    out_lines.append(f"{'    ' * indent}return {psudo_expr}")
                else:
                    out_lines.append(f"{'    ' * indent}return")
                i += 1
                continue

            # Print statement: p expr
            p_match = re.match(r"^p\s+(.+)$", line)
            if p_match:
                expr = p_match.group(1).strip()
                psudo_expr = ModelConformanceHarness._convert_polish_expr(expr)
                out_lines.append(f"{'    ' * indent}print {psudo_expr}")
                i += 1
                continue

            # Method call: m inst method (args)
            m_match = re.match(r"^m\s+([a-zA-Z0-9_]+)\s+([a-zA-Z0-9_]+)\s*\((.*?)\)$", line)
            if m_match:
                inst, method, raw_args = m_match.groups()
                args_str = ", ".join(raw_args.split())
                out_lines.append(f"{'    ' * indent}{inst}.{method}({args_str})")
                i += 1
                continue

            # Builtin file operations: @wt, @rd, @mk
            wt_match = re.match(r"^@wt\s+([a-zA-Z0-9_\"']+)\s+(.+)$", line)
            if wt_match:
                path, content = wt_match.groups()
                out_lines.append(f"{'    ' * indent}writeFile({path}, {ModelConformanceHarness._convert_polish_expr(content)})")
                i += 1
                continue

            mk_match = re.match(r"^@mk\s+([a-zA-Z0-9_\"']+)$", line)
            if mk_match:
                p = mk_match.group(1)
                out_lines.append(f"{'    ' * indent}mkdir({p})")
                i += 1
                continue

            # Generic fallback
            out_lines.append(f"{'    ' * indent}{line}")
            i += 1

        return "\n".join(out_lines)

    @staticmethod
    def _convert_polish_expr(expr: str) -> str:
        """Helper to convert Polish-prefix expressions into infix form recursively."""
        expr = expr.strip()
        if not expr:
            return ""

        # Builtin helper wrappers: (@rd path) -> readFile(path), (@ex path) -> exists(path), etc.
        builtin_m = re.match(r"^\(@(rd|wt|ls|ex|if|id|mk)\s*(.*?)\)$", expr)
        if builtin_m:
            b_code, b_args = builtin_m.groups()
            map_name = {
                "rd": "readFile",
                "wt": "writeFile",
                "ls": "listDir",
                "ex": "exists",
                "if": "isFile",
                "id": "isDir",
                "mk": "mkdir",
            }.get(b_code, b_code)
            args_converted = ModelConformanceHarness._convert_polish_expr(b_args) if b_args else ""
            return f"{map_name}({args_converted})"

        # Binary or function call: (op_or_func arg1 arg2 ...)
        if expr.startswith("(") and expr.endswith(")"):
            inner = expr[1:-1].strip()
            # Split tokens respecting quotes and nested parens
            parts = ModelConformanceHarness._split_expr_tokens(inner)
            if len(parts) >= 1:
                op_or_func = parts[0]
                args = parts[1:]
                converted_args = [ModelConformanceHarness._convert_polish_expr(a) for a in args]
                if op_or_func in ["+", "-", "*", "/", ">", "<", ">=", "<=", "==", "!="]:
                    if len(args) == 2:
                        l_conv = ModelConformanceHarness._convert_polish_expr(args[0])
                        r_conv = ModelConformanceHarness._convert_polish_expr(args[1])
                        return f"({l_conv} {op_or_func} {r_conv})"
                    elif len(args) == 1 and op_or_func == "-":
                        return f"-{ModelConformanceHarness._convert_polish_expr(args[0])}"
                    elif len(args) > 2 and op_or_func in ["+", "*"]:
                        res = ModelConformanceHarness._convert_polish_expr(args[0])
                        for a in args[1:]:
                            res = f"({res} {op_or_func} {ModelConformanceHarness._convert_polish_expr(a)})"
                        return res
                # Function call
                return f"{op_or_func}({', '.join([ModelConformanceHarness._convert_polish_expr(a) for a in args])})"

        return expr

    @staticmethod
    def _split_expr_tokens(text: str) -> List[str]:
        tokens = []
        curr = []
        depth = 0
        in_str = False

        for ch in text:
            if ch == '"':
                in_str = not in_str
                curr.append(ch)
            elif in_str:
                curr.append(ch)
            elif ch == '(':
                depth += 1
                curr.append(ch)
            elif ch == ')':
                depth -= 1
                curr.append(ch)
            elif ch.isspace() and depth == 0 and not in_str:
                if curr:
                    tokens.append("".join(curr))
                    curr = []
            else:
                curr.append(ch)
        if curr:
            tokens.append("".join(curr))
        return tokens
