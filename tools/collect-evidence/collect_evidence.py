#!/usr/bin/env python3
"""collect_evidence.py — run 证据目录与 manifest 自动化（M0-T6；EVIDENCE_SPEC v1.0 生成器）。

子命令：
  init     --gate G0 --task M0-T6 [--runs-root artifacts] [--run-id G0-...-4hex]
           创建 artifacts/<gate>/<run-id>/（run-id 按 EVIDENCE_SPEC §1 生成：
           G<k>-yyyymmdd-hhmmss-4hex）。目标目录已存在时拒绝执行，不破坏已有 run。
  manifest --run-dir <dir> [--task M0-T6] [--commit <hex>] [--notes "..."] [--machine-json <file>]
           扫描 run 目录生成 manifest.json + SHA256SUMS.txt（排除这两个文件自身）。
           已有 manifest 时保留其元数据仅刷新 files[] → 重复执行字节级幂等。
  validate --run-dir <dir>
           校验 manifest.json：schema（manifest.schema.json）+ 格式语义
           （run-id/commit/sha256 格式、路径安全、task_ids/gate 一致性）。不读磁盘文件内容。
  verify   --run-dir <dir>
           完整性核查：先 validate，再逐文件核对存在/大小/SHA-256，发现
           MISSING / MODIFIED / EXTRA / SUMS-MISMATCH 即失败。
  --selftest
           内置合法 / 非法 / 重复执行场景自测（临时目录内运行，不触碰 artifacts/）。

退出码：0 = 通过；1 = 校验或完整性失败；2 = 用法错误或自测失败。
"""
from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

SCHEMA_PATH = Path(__file__).resolve().parent / "manifest.schema.json"
MANIFEST_NAME = "manifest.json"
SUMS_NAME = "SHA256SUMS.txt"
RUNID_RE = re.compile(r"^G\d-\d{8}-\d{6}-[0-9a-f]{4}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{7,40}$")
GATE_RE = re.compile(r"^G[0-5]$")
TASK_RE = re.compile(r"^M[0-6]-T\d+$")


# --- 小工具 -------------------------------------------------------------------

def arg_value(args: list[str], flag: str) -> str | None:
    if flag in args:
        i = args.index(flag)
        if i + 1 >= len(args):
            return ""
        return args[i + 1]
    return None


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def run_id_new(gate: str) -> str:
    stamp = datetime.now().astimezone().strftime("%Y%m%d-%H%M%S")
    return f"{gate}-{stamp}-{os.urandom(2).hex()}"


def detect_git_commit() -> str | None:
    try:
        out = subprocess.run(["git", "rev-parse", "HEAD"],
                             capture_output=True, text=True, timeout=10)
        if out.returncode == 0 and COMMIT_RE.match(out.stdout.strip()):
            return out.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        pass
    return None


# --- manifest 构建 ------------------------------------------------------------

def scan_run_files(run_dir: Path) -> list[dict]:
    """遍历 run 目录（排除 manifest/SHA256SUMS 自身），返回按 path 排序的 files[] 条目。"""
    entries: list[dict] = []
    for root, dirs, names in os.walk(run_dir):
        dirs.sort()
        for name in sorted(names):
            if name in (MANIFEST_NAME, SUMS_NAME):
                continue
            full = Path(root) / name
            entries.append({
                "path": full.relative_to(run_dir).as_posix(),
                "sha256": sha256_file(full),
                "bytes": full.stat().st_size,
                "description": "",
                "redacted": True,
                "media_content": False,
            })
    entries.sort(key=lambda e: e["path"])
    return entries


def build_manifest(run_dir: Path, overrides: dict) -> dict:
    """构建 manifest：已有 manifest 时保留元数据仅刷新 files[]（重复执行幂等）。"""
    existing: dict = {}
    if (run_dir / MANIFEST_NAME).exists():
        existing = json.loads((run_dir / MANIFEST_NAME).read_text(encoding="utf-8"))

    meta: dict = {}
    meta["run_id"] = existing.get("run_id", run_dir.name)
    meta["task_ids"] = existing.get("task_ids") or (
        [overrides["task"]] if overrides.get("task") else [])
    meta["gate"] = existing.get("gate") or overrides.get("gate") or run_dir.name.split("-")[0]
    meta["created_local"] = existing.get(
        "created_local", datetime.now().astimezone().isoformat(timespec="seconds"))
    meta["machine"] = existing.get("machine") or overrides.get("machine") or {
        "os_build": None, "cpu": None, "gpu": None, "ram_gb": None}
    if "git_commit" in existing:
        meta["git_commit"] = existing["git_commit"]
    elif overrides.get("commit") is not None:
        meta["git_commit"] = overrides["commit"]
    else:
        meta["git_commit"] = detect_git_commit()
    meta["tool_versions"] = existing.get("tool_versions", {})
    meta["system_changes"] = existing.get("system_changes", [])
    meta["not_run_tasks"] = existing.get("not_run_tasks", [])
    meta["notes"] = existing.get("notes", "")

    # 显式覆盖（同值重复执行仍幂等）
    if overrides.get("task") is not None:
        meta["task_ids"] = [overrides["task"]]
    if overrides.get("gate"):
        meta["gate"] = overrides["gate"]
    if overrides.get("commit") is not None:
        meta["git_commit"] = overrides["commit"]
    if overrides.get("notes") is not None:
        meta["notes"] = overrides["notes"]

    meta["files"] = scan_run_files(run_dir)
    return meta


def write_manifest(run_dir: Path, manifest: dict) -> None:
    with open(run_dir / MANIFEST_NAME, "w", encoding="utf-8", newline="\n") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
        f.write("\n")
    with open(run_dir / SUMS_NAME, "w", encoding="utf-8", newline="\n") as f:
        for e in manifest["files"]:
            f.write(f"{e['sha256']}  {e['path']}\n")


# --- schema 校验（fields 风格，同 verify-environment） -------------------------

def load_schema() -> dict:
    return json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))


def resolve(data: dict, dotted: str):
    node = data
    for part in dotted.split("."):
        if isinstance(node, dict) and part in node:
            node = node[part]
        else:
            return False, None
    return True, node


def type_ok(value, expected: str) -> bool:
    table = {"string": str, "number": (int, float), "bool": bool,
             "object": dict, "array": list}
    t = table[expected]
    if expected == "number" and isinstance(value, bool):
        return False
    if expected == "bool" and not isinstance(value, bool):
        return False
    return isinstance(value, t)


def validate_manifest(run_dir: Path) -> list[str]:
    """结构 + 格式语义校验（不读磁盘文件内容——磁盘一致性属 verify）。"""
    mpath = run_dir / MANIFEST_NAME
    if not mpath.is_file():
        return [f"MISSING: {MANIFEST_NAME}"]
    try:
        data = json.loads(mpath.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        return [f"JSON: {exc}"]

    errors = validate_structure(data, load_schema())
    if not RUNID_RE.match(str(data.get("run_id", ""))):
        errors.append("SEMANTIC: run_id 不符合 G<k>-yyyymmdd-hhmmss-4hex 格式")
    gate = str(data.get("gate", ""))
    if gate and not GATE_RE.match(gate):
        errors.append(f"SEMANTIC: gate 非法 {gate!r}")
    if gate and str(data.get("run_id", "")).split("-")[0] != gate:
        errors.append("SEMANTIC: gate 与 run-id 前缀不一致")
    commit = data.get("git_commit")
    if commit is not None and not COMMIT_RE.match(str(commit)):
        errors.append("SEMANTIC: git_commit 非 7~40 位 hex")
    if data.get("task_ids") == []:
        errors.append("SEMANTIC: task_ids 为空（run 关闭时必须归属任务）")
    files = data.get("files", [])
    if not files:
        errors.append("SEMANTIC: files 为空（run 关闭时必须有证据文件）")
    seen: set[str] = set()
    for e in files:
        if not isinstance(e, dict):
            errors.append("SEMANTIC: files[] 条目不是对象")
            continue
        p = e.get("path")
        if not isinstance(p, str) or not safe_rel_path(p):
            errors.append(f"SEMANTIC: path 不安全 {p!r}")
            continue
        if p in seen:
            errors.append(f"SEMANTIC: path 重复 {p}")
        seen.add(p)
        if not SHA256_RE.match(str(e.get("sha256", ""))):
            errors.append(f"SEMANTIC: sha256 非 64 hex: {p}")
        size = e.get("bytes")
        if not isinstance(size, int) or isinstance(size, bool) or size < 0:
            errors.append(f"SEMANTIC: bytes 非法: {p}")
        for flag in ("redacted", "media_content"):
            if flag in e and not isinstance(e[flag], bool):
                errors.append(f"SEMANTIC: {flag} 非 bool: {p}")
    for sc in data.get("system_changes", []):
        if isinstance(sc, dict):
            for k in ("action", "scope", "rollback"):
                if not isinstance(sc.get(k), str):
                    errors.append(f"SEMANTIC: system_changes[].{k} 缺失或非字符串")
            if not isinstance(sc.get("confirmed_by_user"), bool):
                errors.append("SEMANTIC: system_changes[].confirmed_by_user 非 bool")
        else:
            errors.append("SEMANTIC: system_changes[] 条目不是对象")
    return errors


def validate_structure(data: dict, schema: dict) -> list[str]:
    errors: list[str] = []
    for path, rule in schema["fields"].items():
        found, value = resolve(data, path)
        if not found:
            if rule.get("required"):
                errors.append(f"MISSING: {path}")
            continue
        if value is None:
            if not rule.get("nullable"):
                errors.append(f"NULL: {path} (not nullable)")
            continue
        if not type_ok(value, rule["type"]):
            errors.append(f"TYPE: {path} expected {rule['type']}, got {type(value).__name__}")
    return errors


def safe_rel_path(p: str) -> bool:
    """契约路径必须相对、正斜杠、无 .. 与盘符。"""
    if not p or p.startswith("/") or "\\" in p or ":" in p:
        return False
    parts = p.split("/")
    return ".." not in parts and all(part not in ("", ".") for part in parts)


# --- 完整性核查 ---------------------------------------------------------------

def integrity_problems(run_dir: Path) -> list[str]:
    """磁盘完整性核查：存在性、大小、哈希、EXTRA、SHA256SUMS 一致性。"""
    problems: list[str] = []
    errs = validate_manifest(run_dir)
    if errs:
        return [f"INVALID-MANIFEST: {e}" for e in errs]

    data = json.loads((run_dir / MANIFEST_NAME).read_text(encoding="utf-8"))
    listed = {e["path"]: e for e in data["files"]}
    for p, e in listed.items():
        full = run_dir / p
        if not full.is_file():
            problems.append(f"MISSING: {p}")
        elif full.stat().st_size != e["bytes"] or sha256_file(full) != e["sha256"]:
            problems.append(f"MODIFIED: {p}")
    for root, dirs, names in os.walk(run_dir):
        dirs.sort()
        for name in sorted(names):
            if name in (MANIFEST_NAME, SUMS_NAME):
                continue
            rel = (Path(root) / name).relative_to(run_dir).as_posix()
            if rel not in listed:
                problems.append(f"EXTRA: {rel}")

    sums_path = run_dir / SUMS_NAME
    if not sums_path.is_file():
        problems.append(f"MISSING: {SUMS_NAME}")
        return problems
    sums: dict[str, str] = {}
    for line in sums_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        parts = line.split("  ", 1)
        if len(parts) != 2 or not SHA256_RE.match(parts[0]):
            problems.append(f"SUMS-BADLINE: {line[:72]}")
            continue
        sums[parts[1]] = parts[0]
    expect = {e["path"]: e["sha256"] for e in data["files"]}
    if sums != expect:
        only_sums = sorted(set(sums) - set(expect))
        only_man = sorted(set(expect) - set(sums))
        diff = sorted(p for p in set(sums) & set(expect) if sums[p] != expect[p])
        problems.append(f"SUMS-MISMATCH: sums_only={only_sums} "
                        f"manifest_only={only_man} hash_diff={diff}")
    return problems


# --- 子命令 -------------------------------------------------------------------

def cmd_init(args: list[str]) -> int:
    gate = arg_value(args, "--gate")
    task = arg_value(args, "--task")
    runs_root = Path(arg_value(args, "--runs-root") or "artifacts")
    pinned = arg_value(args, "--run-id")
    if not gate or not task or not GATE_RE.match(gate) or not TASK_RE.match(task):
        print("USAGE-ERROR: init 需要 --gate G<k> 与 --task M<k>-T<n>（格式合法）")
        return 2
    if pinned is not None:
        if not RUNID_RE.match(pinned):
            print("USAGE-ERROR: --run-id 不符合 G<k>-yyyymmdd-hhmmss-4hex 格式")
            return 2
        if pinned.split("-")[0] != gate:
            print("USAGE-ERROR: --run-id 前缀与 --gate 不一致")
            return 2
        run_dir = runs_root / gate / pinned
    else:
        run_dir = runs_root / gate / run_id_new(gate)
    if run_dir.exists():
        print(f"REFUSED: 目标已存在（不破坏已有 run）：{run_dir}")
        return 1
    run_dir.mkdir(parents=True)
    print(f"RUN_DIR={run_dir}")
    print(f"RUN_ID={run_dir.name}")
    print(f"TASK={task}")
    return 0


def cmd_manifest(args: list[str]) -> int:
    rd = arg_value(args, "--run-dir")
    if not rd:
        print("USAGE-ERROR: manifest 需要 --run-dir <dir>")
        return 2
    run_dir = Path(rd)
    if not run_dir.is_dir() or not RUNID_RE.match(run_dir.name):
        print(f"USAGE-ERROR: --run-dir 必须是符合 run-id 命名的已存在目录：{run_dir}")
        return 2

    overrides: dict = {"task": arg_value(args, "--task"),
                       "gate": arg_value(args, "--gate"),
                       "commit": arg_value(args, "--commit"),
                       "notes": arg_value(args, "--notes")}
    mj = arg_value(args, "--machine-json")
    if mj:
        overrides["machine"] = json.loads(Path(mj).read_text(encoding="utf-8"))

    existing = (run_dir / MANIFEST_NAME).exists()
    if not existing and not overrides["task"]:
        print("USAGE-ERROR: 首次生成 manifest 需要 --task M<k>-T<n>")
        return 2
    if overrides["commit"] is not None and not COMMIT_RE.match(overrides["commit"]):
        print("USAGE-ERROR: --commit 非 7~40 位 hex")
        return 2
    if overrides["task"] is not None and not TASK_RE.match(overrides["task"]):
        print("USAGE-ERROR: --task 格式非法")
        return 2

    manifest = build_manifest(run_dir, overrides)
    write_manifest(run_dir, manifest)
    print(f"MANIFEST={run_dir / MANIFEST_NAME}")
    print(f"FILES={len(manifest['files'])}")
    print(f"IDEMPOTENT={'reused-existing-meta' if existing else 'created'}")
    return 0


def cmd_validate(args: list[str]) -> int:
    rd = arg_value(args, "--run-dir")
    if not rd:
        print("USAGE-ERROR: validate 需要 --run-dir <dir>")
        return 2
    run_dir = Path(rd)
    errors = validate_manifest(run_dir)
    for e in errors:
        print(f"[FAIL] {e}")
    print(f"{'VALID' if not errors else 'INVALID'}: "
          f"{run_dir / MANIFEST_NAME} ({len(errors)} errors)")
    return 1 if errors else 0


def cmd_verify(args: list[str]) -> int:
    rd = arg_value(args, "--run-dir")
    if not rd:
        print("USAGE-ERROR: verify 需要 --run-dir <dir>")
        return 2
    run_dir = Path(rd)
    problems = integrity_problems(run_dir)
    for p in problems:
        print(f"[FAIL] {p}")
    print(f"{'INTEGRITY-OK' if not problems else 'INTEGRITY-FAIL'}: {run_dir}")
    return 1 if problems else 0


# --- 自测（合法 / 非法 / 重复执行场景） ---------------------------------------

def make_fixture_run(root: Path, name: str = "G0-20261113-120000-abcd") -> Path:
    rd = root / name
    (rd / "logs").mkdir(parents=True)
    (rd / "a.txt").write_text("alpha\n", encoding="utf-8", newline="\n")
    (rd / "logs" / "x.log").write_text("log line\n", encoding="utf-8", newline="\n")
    return rd


def refresh(run_dir: Path, overrides: dict | None = None) -> None:
    write_manifest(run_dir, build_manifest(run_dir, overrides or {}))


def verify_problems(run_dir: Path) -> list[str]:
    """自测辅助：捕获 cmd_verify 输出中的 [FAIL] 行（即问题列表）。"""
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        cmd_verify(["--run-dir", str(run_dir)])
    return [ln[7:] for ln in buf.getvalue().splitlines() if ln.startswith("[FAIL] ")]


def run_selftest() -> int:
    ok = True

    def check(label: str, cond: bool, detail: str = "") -> None:
        nonlocal ok
        msg = f"{'[PASS]' if cond else '[FAIL]'} selftest: {label}"
        if detail and not cond:
            msg += f" -> {detail}"
        print(msg)
        ok = ok and cond

    with tempfile.TemporaryDirectory(prefix="shutu-collect-ev-") as tmp:
        root = Path(tmp)

        # --- 合法场景 ---
        rd = make_fixture_run(root)
        refresh(rd, {"task": "M0-T6", "commit": "a" * 40, "notes": "t1"})
        errs = validate_manifest(rd)
        check("T1 合法 run 的 manifest 通过 validate", not errs, str(errs[:3]))

        # --- 重复执行：字节级幂等 ---
        bytes1 = (rd / MANIFEST_NAME).read_bytes()
        refresh(rd, {})
        bytes2 = (rd / MANIFEST_NAME).read_bytes()
        refresh(rd, {})
        bytes3 = (rd / MANIFEST_NAME).read_bytes()
        check("T2 重复 manifest 字节级幂等（元数据保留）", bytes1 == bytes2 == bytes3)
        data = json.loads(bytes3.decode("utf-8"))
        check("T3 SHA256SUMS 覆盖全部文件且排除自身",
              data["files"][0]["path"] == "a.txt" and data["files"][1]["path"] == "logs/x.log")

        # --- 非法场景 ---
        bad = json.loads(bytes3.decode("utf-8"))
        bad["files"][0]["sha256"] = "nothex"
        (rd / MANIFEST_NAME).write_text(json.dumps(bad, ensure_ascii=False), encoding="utf-8")
        check("T4 sha256 格式非法被拒", any("sha256" in e for e in validate_manifest(rd)))

        bad["files"][0].pop("path")
        (rd / MANIFEST_NAME).write_text(json.dumps(bad, ensure_ascii=False), encoding="utf-8")
        check("T5 files[].path 缺失被拒", any("path" in e for e in validate_manifest(rd)))

        bad = json.loads(bytes3.decode("utf-8"))
        del bad["run_id"]
        (rd / MANIFEST_NAME).write_text(json.dumps(bad, ensure_ascii=False), encoding="utf-8")
        check("T6 缺 run_id 被拒", any("MISSING: run_id" in e for e in validate_manifest(rd)))

        bad = json.loads(bytes3.decode("utf-8"))
        bad["files"][0]["path"] = "../escape.txt"
        (rd / MANIFEST_NAME).write_text(json.dumps(bad, ensure_ascii=False), encoding="utf-8")
        check("T7 路径逃逸被拒", any("不安全" in e for e in validate_manifest(rd)))

        refresh(rd, {"task": "M0-T6"})

        # --- 完整性：篡改 / 删除 / 新增 ---
        (rd / "a.txt").write_text("tampered\n", encoding="utf-8", newline="\n")
        check("T8 篡改文件被 verify 发现（MODIFIED）",
              any(e.startswith("MODIFIED: a.txt") for e in verify_problems(rd)))
        (rd / "a.txt").write_text("alpha\n", encoding="utf-8", newline="\n")

        (rd / "logs" / "x.log").unlink()
        check("T9 删除已列文件被 verify 发现（MISSING）",
              any(e.startswith("MISSING: logs/x.log") for e in verify_problems(rd)))
        (rd / "logs" / "x.log").write_text("log line\n", encoding="utf-8", newline="\n")

        (rd / "extra.txt").write_text("surprise\n", encoding="utf-8", newline="\n")
        check("T10 新增未列文件被 verify 发现（EXTRA）",
              any(e.startswith("EXTRA: extra.txt") for e in verify_problems(rd)))
        (rd / "extra.txt").unlink()

        # --- SHA256SUMS 与 manifest 不一致 ---
        lines = (rd / SUMS_NAME).read_text(encoding="utf-8").splitlines()
        lines[0] = ("0" * 64) + "  " + lines[0].split("  ", 1)[1]
        (rd / SUMS_NAME).write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        check("T11 SHA256SUMS 与 manifest 不一致被发现（SUMS-MISMATCH）",
              any("SUMS-MISMATCH" in e for e in verify_problems(rd)))
        refresh(rd, {})
        check("T12 修复后 verify 恢复通过", not verify_problems(rd))

        # --- init 拒绝已存在目录（不破坏）---
        target = root / "G0" / "G0-20261113-130000-beef"
        target.mkdir(parents=True)
        (target / "keep.txt").write_text("keep\n", encoding="utf-8", newline="\n")
        rc = cmd_init(["--gate", "G0", "--task", "M0-T6", "--runs-root", str(root),
                       "--run-id", "G0-20261113-130000-beef"])
        check("T13 init 对已存在 run-id 目录拒绝且不破坏",
              rc == 1 and (target / "keep.txt").is_file()
              and sorted(p.name for p in target.iterdir()) == ["keep.txt"])
        rc2 = cmd_init(["--gate", "G1", "--task", "M1-T4", "--runs-root", str(root),
                        "--run-id", "G0-20261113-130000-beef"])
        check("T14 --run-id 前缀与 --gate 不一致被拒", rc2 == 2)

        # --- gate/run-id 一致性与端到端 ---
        rd2 = make_fixture_run(root, name="G1-20261113-140000-cafe")
        refresh(rd2, {"task": "M1-T4"})
        check("T15 gate 与 run-id 前缀一致时通过", not validate_manifest(rd2))

        rd3 = make_fixture_run(root, name="G0-20261113-150000-dead")
        refresh(rd3, {"task": "M0-T6", "commit": "b" * 40, "notes": "e2e"})
        check("T16 端到端 fixture→manifest→validate→verify 全过",
              not validate_manifest(rd3) and not verify_problems(rd3))

    return 0 if ok else 2


def main() -> int:
    args = sys.argv[1:]
    if "--selftest" in args:
        return run_selftest()
    if not args:
        print(__doc__)
        return 2
    cmd, rest = args[0], args[1:]
    table = {"init": cmd_init, "manifest": cmd_manifest,
             "validate": cmd_validate, "verify": cmd_verify}
    if cmd not in table:
        print(f"USAGE-ERROR: 未知子命令 {cmd!r}")
        print(__doc__)
        return 2
    return table[cmd](rest)


if __name__ == "__main__":
    sys.exit(main())
