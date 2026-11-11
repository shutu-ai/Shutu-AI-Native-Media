#!/usr/bin/env python3
"""check_idempotency.py — 幂等性测试（M0-T1）：两次 environment.json 除 volatile 外一致。

用法：python check_idempotency.py <fileA.json> <fileB.json>
退出码：0 = 一致；1 = 有差异。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from validate_environment import load_schema  # noqa: E402


def prune_volatile(node, dotted: str, volatile_paths: set[str]):
    if dotted in volatile_paths:
        return "<volatile>"
    if isinstance(node, dict):
        return {k: prune_volatile(v, f"{dotted}.{k}".lstrip("."), volatile_paths) for k, v in node.items()}
    if isinstance(node, list):
        return [prune_volatile(v, dotted, volatile_paths) for v in node]
    return node


def main() -> int:
    a_path, b_path = Path(sys.argv[1]), Path(sys.argv[2])
    schema = load_schema()
    volatile = {p for p, r in schema["fields"].items() if r.get("volatile")}
    a = prune_volatile(json.loads(a_path.read_text(encoding="utf-8")), "", volatile)
    b = prune_volatile(json.loads(b_path.read_text(encoding="utf-8")), "", volatile)

    def diff(x, y, path=""):
        out = []
        if type(x) is not type(y):
            return [(path or "<root>", f"{type(x).__name__} vs {type(y).__name__}")]
        if isinstance(x, dict):
            for k in sorted(set(x) | set(y)):
                if k not in x:
                    out.append((f"{path}.{k}", "only in B"))
                elif k not in y:
                    out.append((f"{path}.{k}", "only in A"))
                else:
                    out += diff(x[k], y[k], f"{path}.{k}")
        elif isinstance(x, list):
            if len(x) != len(y):
                out.append((path, f"len {len(x)} vs {len(y)}"))
            else:
                for i, (xi, yi) in enumerate(zip(x, y)):
                    out += diff(xi, yi, f"{path}[{i}]")
        elif x != y:
            out.append((path, f"{x!r} vs {y!r}"))
        return out

    diffs = diff(a, b)
    for p, d in diffs:
        print(f"[DIFF] {p}: {d}")
    print(f"IDEMPOTENT (volatile={sorted(volatile)}): {'YES' if not diffs else 'NO'} ({len(diffs)} diffs)")
    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main())
