#!/usr/bin/env python3
"""validate_environment.py — environment.json schema 校验（M0-T1 自动化测试）。

用法：
  python validate_environment.py --file <environment.json>   # 校验真实文件
  python validate_environment.py --selftest                  # 内置好/坏样例自测

退出码：0 = 通过；1 = 校验失败；2 = 自测失败。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

SCHEMA_PATH = Path(__file__).resolve().parent / "schema.json"


def load_schema() -> dict:
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


def resolve(data: dict, dotted: str):
    """按 dotted path 取值；路径不存在返回 (False, None)。"""
    node = data
    for part in dotted.split("."):
        if isinstance(node, dict) and part in node:
            node = node[part]
        else:
            return False, None
    return True, node


def type_ok(value, expected: str) -> bool:
    table = {
        "string": str,
        "number": (int, float),
        "bool": bool,
        "object": dict,
        "array": list,
    }
    t = table[expected]
    if expected == "number" and isinstance(value, bool):  # bool 是 int 子类，排除
        return False
    if expected == "bool" and not isinstance(value, bool):
        return False
    return isinstance(value, t)


def validate(data: dict, schema: dict) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    for path, rule in schema["fields"].items():
        found, value = resolve(data, path)
        if not found:
            if rule.get("required"):
                errors.append(f"MISSING: {path}")  # required = 键必须存在
            continue
        if value is None:
            if not rule.get("nullable"):
                errors.append(f"NULL: {path} (not nullable)")  # nullable = 值可为 null
            continue
        if not type_ok(value, rule["type"]):
            errors.append(f"TYPE: {path} expected {rule['type']}, got {type(value).__name__}")
    return errors, warnings


def run_selftest() -> int:
    schema = load_schema()

    good = {
        "schema_version": "1.0",
        "tool": {"name": "verify-environment", "version": "1.0", "engine": "PowerShell 7"},
        "run_id": "G0-20261111-120000-ab12",
        "generated_at": "2026-11-11T12:00:00+08:00",
        "machine": {
            "hostname": "TEST",
            "os": {"caption": "c", "version": "10.0", "build": "26200", "ubr": 1,
                   "display_version": "25H2", "edition": "e", "edition_id": "id", "meets_22000": True},
            "cpu": {"name": "CPU", "cores": 4, "threads": 8},
            "gpu": [{"name": "GPU", "driver_version": "1", "driver_date": "2026-01-01"}],
            "ram_gb": {"total": 32.0, "free": 16.0, "volatile": True},
            "disk": [{"drive": "C:", "volume": "v", "total_gb": 1.0, "free_gb": 1.0, "volatile": True}],
        },
        "toolchain": {
            "git": "git version 2.56", "python": "3.14", "cmake": "cmake version 4.2",
            "rust": {"rustc": "rustc 1.95", "cargo": "cargo 1.95",
                     "rustup_toolchain": "stable-x86_64-pc-windows-msvc", "default_host": "x86_64-pc-windows-msvc"},
            "visual_studio": {"instances": []},
            "windows_sdks": ["10.0.26100"],
            "wdk": {"installed": False, "km_headers": False, "vsix": []},
            "gstreamer": {"installed": True, "variant": "msvc", "version_output": "GStreamer 1.28.3",
                          "root": "C:\\gstreamer", "mingw_root_detected": None, "gst_launch_path": "C:\\x\\gst-launch-1.0.exe"},
        },
        "devices": {"cameras": [], "audio": [], "privacy_hint": {"camera": "Allow", "microphone": "Allow"}},
        "security_software": {"av_products": [], "firewall": [], "adapters_up": [], "note": "n"},
        "accounts": {"local_users_enabled": ["u"], "current_shell_is_admin": False},
        "unconfirmed_facts": [],
        "probes_failed": [],
    }

    bad_cases: list[tuple[str, dict]] = []
    import copy
    b = copy.deepcopy(good); del b["machine"]["os"]["build"]
    bad_cases.append(("missing required machine.os.build", b))
    b = copy.deepcopy(good); b["machine"]["os"]["ubr"] = "not-a-number"
    bad_cases.append(("wrong type machine.os.ubr", b))
    b = copy.deepcopy(good); b["toolchain"]["gstreamer"]["installed"] = "yes"
    bad_cases.append(("wrong type toolchain.gstreamer.installed", b))
    b = copy.deepcopy(good); b["machine"]["gpu"] = "should-be-array"
    bad_cases.append(("wrong type machine.gpu", b))

    ok, _ = validate(good, schema)
    if ok:
        print("[FAIL] selftest: good sample unexpectedly rejected:", ok)
        return 2
    print("[PASS] selftest: good sample accepted")

    for name, case in bad_cases:
        errs, _ = validate(case, schema)
        if not errs:
            print(f"[FAIL] selftest: bad case NOT rejected: {name}")
            return 2
        print(f"[PASS] selftest: bad case rejected ({name}) -> {errs[0]}")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if "--selftest" in args:
        return run_selftest()
    if "--file" in args:
        path = Path(args[args.index("--file") + 1])
        data = json.loads(path.read_text(encoding="utf-8"))
        errors, _ = validate(data, load_schema())
        for e in errors:
            print(f"[FAIL] {e}")
        print(f"{'VALID' if not errors else 'INVALID'}: {path} ({len(errors)} errors)")
        return 1 if errors else 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
