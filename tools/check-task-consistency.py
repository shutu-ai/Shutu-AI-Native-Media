#!/usr/bin/env python3
"""check-task-consistency.py — 任务体系文档一致性自动检查（ADR-011）。

只依赖 Python 标准库。对 docs/tasks/ 文档族做结构一致性与语义场景检查：
- ID 唯一性、三处集合一致（MASTER ↔ 里程碑文件 ↔ 依赖表）、总数 51；
- 每个任务 11 字段齐全；BLOCKED_BY 引用合法；
- 6 个 Gate 汇总任务 AGG 语义；AUTH 任务不得作为 Gate 的硬依赖；
- 场景 S1：可选 AUTH 任务（M2-T6/M5-T9）NOT_RUN 时各 Gate 报告仍可达；
- 场景 S2：上游失败（HOLD 分支）时 Gate 照常产出 HOLD 报告、下游保持 BLOCKED；
- 场景 S3：G5 双交付等级 GO(LEVEL=L1/L2) 语义落档；
- EVIDENCE_SPEC / EVIDENCE_INDEX / SYS_CHANGE 落档；架构基线 SHA-256 未变。

用法：python tools/check-task-consistency.py   （全部 PASS 退出码 0，否则 1）
"""
from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DOCS = REPO / "docs"
TASKS = DOCS / "tasks"

# --- 期望常量（ADR-011 后的目标态） ------------------------------------------
EXPECTED_TOTAL = 51
EXPECTED_PER_MILESTONE = {"M0": 7, "M1": 8, "M2": 7, "M3": 9, "M4": 6, "M5": 9, "M6": 5}
GATE_TASKS = {"M0-T7": "G0", "M1-T8": "G1", "M2-T7": "G2", "M3-T9": "G3", "M4-T6": "G4", "M5-T8": "G5"}
BASELINE_SHA256 = "cc30728fec5db42764602fbcdc4c0d78002eed38497d2cb58e767456bcf6cf37"

MILESTONE_FILES = {f"M{i}": TASKS / f"M{i}_TASKS.md" for i in range(7)}

results: list[tuple[bool, str, str]] = []  # (ok, label, detail)


def check(label: str, ok: bool, detail: str = "") -> None:
    results.append((bool(ok), label, detail))


def read(p: Path) -> str:
    return p.read_text(encoding="utf-8")


def table_rows(text: str, id_pattern: str) -> list[list[str]]:
    """返回表格行（按 | 切分去空的单元格列表），行首单元格匹配 id_pattern。"""
    rows = []
    for line in text.splitlines():
        s = line.strip()
        if not (s.startswith("|") and s.endswith("|")):
            continue
        cells = [c.strip() for c in s.strip("|").split("|")]
        if cells and re.fullmatch(id_pattern, cells[0]):
            rows.append(cells)
    return rows


def dep_tokens(cell: str) -> list[tuple[str, str]]:
    """提取依赖引用：返回 (kind, target)；kind ∈ {task, agg, gate}。"""
    out: list[tuple[str, str]] = []
    for kind, pat in (("agg", r"AGG:(M\d-T\d+)"), ("gate", r"G\d=GO"), ("task", r"M\d-T\d+")):
        for m in re.finditer(pat, cell):
            if kind == "agg":
                out.append(("agg", m.group(1)))
            elif kind == "gate":
                out.append(("gate", m.group(0)))
            else:
                before = cell[max(0, m.start() - 4): m.start()]
                if before.endswith("AGG:"):
                    continue  # AGG: 前缀里的 ID 不重复记为裸依赖
                out.append(("task", m.group(0)))
    return out


def sha256(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def section(text: str, start_head: str, end_head: str) -> str:
    """截取 start_head 标题之后到 end_head 标题之前的文本。"""
    seg = text.split(start_head, 1)
    if len(seg) != 2:
        return ""
    seg = seg[1].split(end_head, 1)
    return seg[0]


# --- 0. 基线零改动 -------------------------------------------------------------
baseline = DOCS / "ARCHITECTURE_BASELINE.md"
check("BASELINE-SHA256 基线未被改动", sha256(baseline) == BASELINE_SHA256,
      f"actual={sha256(baseline)[:12]}… pinned={BASELINE_SHA256[:12]}…")

# --- 1. 读入与解析 -------------------------------------------------------------
master_text = read(TASKS / "MASTER_TASK_PLAN.md")
deps_text = read(TASKS / "TASK_DEPENDENCIES.md")
matrix_text = read(TASKS / "ACCEPTANCE_MATRIX.md")
agents_text = read(REPO / "AGENTS.md")
m5_text = read(TASKS / "M5_TASKS.md")

m3 = section(master_text, "## 3.", "**状态统计**")
master_rows = table_rows(m3, r"M\d-T\d+")
master_ids = [r[0] for r in master_rows]
master_status = {r[0]: (r[4] if len(r) > 4 else "") for r in master_rows}
master_name = {r[0]: (r[1] if len(r) > 1 else "") for r in master_rows}

# 只解析 §2「任务边表」（§3 授权登记表是另一张表，列结构不同，不参与判读）
edge_table = section(deps_text, "## 2.", "## 3.")
deps_rows = table_rows(edge_table, r"M\d-T\d+")
deps_ids = [r[0] for r in deps_rows]
deps_type = {r[0]: (r[3] if len(r) > 3 else "") for r in deps_rows}
deps_blocked_by = {r[0]: (r[2] if len(r) > 2 else "") for r in deps_rows}

milestone_sections: dict[str, dict[str, str]] = {}
milestone_ids: list[str] = []
for mid, path in MILESTONE_FILES.items():
    text = read(path)
    heads = list(re.finditer(r"^##\s+(M\d-T\d+)\s*[·]", text, re.M))
    secs = {}
    for i, h in enumerate(heads):
        end = heads[i + 1].start() if i + 1 < len(heads) else len(text)
        secs[h.group(1)] = text[h.end():end]
    milestone_sections[mid] = secs
    milestone_ids += secs.keys()

all_ids = set(master_ids) | set(deps_ids) | set(milestone_ids)

# --- 2. 结构一致性 -------------------------------------------------------------
dupes = [i for i in sorted(all_ids)
         if master_ids.count(i) > 1 or milestone_ids.count(i) > 1 or deps_ids.count(i) > 1]
check("ID-UNIQUE 无重复 Task ID", not dupes, f"dupes={dupes}")

check("SET-EQUAL MASTER↔里程碑↔依赖表 三处 ID 集合一致",
      set(master_ids) == set(deps_ids) == set(milestone_ids),
      f"master-only={set(master_ids) - set(deps_ids)}, deps-only={set(deps_ids) - set(master_ids)}, "
      f"ms-only={set(milestone_ids) - set(master_ids)}")

counts = {m: len(v) for m, v in milestone_sections.items()}
check("COUNT-51 总数=51 且各里程碑计数正确",
      len(all_ids) == EXPECTED_TOTAL and counts == EXPECTED_PER_MILESTONE,
      f"total={len(all_ids)}, per={counts}")

for mid, secs in milestone_sections.items():
    bad = [tid for tid, body in secs.items()
           if set(re.findall(r"\*\*(\d+)\. ", body)) != {str(i) for i in range(1, 12)}]
    check(f"FIELDS-{mid} 每任务 11 字段齐全", not bad, f"missing/extra fields in: {bad}")

refs_bad: list[str] = []
for tid, cell in deps_blocked_by.items():
    for kind, tgt in dep_tokens(cell):
        if kind in ("task", "agg") and tgt not in all_ids:
            refs_bad.append(f"{tid}->{tgt}")
        if kind == "gate" and not re.fullmatch(r"G[0-5]=GO", tgt):
            refs_bad.append(f"{tid}->{tgt}")
check("REFS-VALID BLOCKED_BY 引用全部合法", not refs_bad, f"bad refs: {refs_bad}")

# --- 3. Gate 语义（ADR-011 D1/D2/D4） ------------------------------------------
gate_types = {g: deps_type.get(g, "") for g in GATE_TASKS}
check("GATE-AGG 六个 Gate 任务类型均为 AGG", all("AGG" in t for t in gate_types.values()),
      f"types={gate_types}")

t7 = deps_blocked_by.get("M2-T7", "")
check("S1-A M2-T7 依赖为 AGG:M2-T6（非硬依赖）",
      "AGG:M2-T6" in t7 and not re.search(r"(?<!AGG:)M2-T6", t7), f"M2-T7 BLOCKED_BY={t7}")

check("S1-B MASTER 中 M2-T6 标注可选+NOT_RUN",
      "可选" in master_name.get("M2-T6", "") and "NOT_RUN" in master_status.get("M2-T6", ""),
      f"name={master_name.get('M2-T6')}, status={master_status.get('M2-T6')}")

t9_type = deps_type.get("M5-T9", "")
check("S1-C M5-T9 存在且 AUTH、条件触发（NOT_RUN 合法）",
      "M5-T9" in all_ids and "AUTH" in t9_type and "NOT_RUN" in t9_type,
      f"M5-T9 type={t9_type}, blocked_by={deps_blocked_by.get('M5-T9')}")

check("S1-D M5-T3 不再含 AUTH（无需授权）", "AUTH" not in deps_type.get("M5-T3", ""),
      f"M5-T3 type={deps_type.get('M5-T3')}")

agg_t9 = ("AGG:M5-T9" in deps_blocked_by.get("M5-T6", "")) and \
         ("AGG:M5-T9" in deps_blocked_by.get("M5-T8", ""))
check("S1-E M5-T6/M5-T8 以 AGG:M5-T9 依赖（签名执行不阻塞汇总）", agg_t9,
      f"M5-T6={deps_blocked_by.get('M5-T6')}, M5-T8={deps_blocked_by.get('M5-T8')}")

hard_auth: list[str] = []
for g in GATE_TASKS:
    for kind, tgt in dep_tokens(deps_blocked_by.get(g, "")):
        if kind == "task" and "AUTH" in deps_type.get(tgt, ""):
            hard_auth.append(f"{g}->{tgt}")
check("S1-F AUTH 任务不作为任何 Gate 的硬依赖", not hard_auth, f"hard deps: {hard_auth}")

gates_sec = section(deps_text, "## 1.", "## 2.")
rules_sec = section(deps_text, "## 0.", "## 1.")
check("S2-A DEPS §0 含 AGGREGATE 报告必达规则", "任意终态" in rules_sec and "报告" in rules_sec)
check("S2-B DEPS §0 含 NOT_RUN 合法终态规则", "NOT_RUN" in rules_sec)
check("S2-C DEPS Gate 边 G2 含 NOT_RUN 显式结论语义", "NOT_RUN" in gates_sec)
check("S2-D DEPS Gate 边 G5 含 GO(LEVEL=L1/L2) 判定语法", "GO(LEVEL=L1" in gates_sec)

check("S3-A MASTER §6 含 NOT_RUN 状态词汇", "NOT_RUN" in section(master_text, "## 6.", "## 7."))
check("S3-B M5-T8 任务文档含 Beta-L1/L2 判定逻辑",
      "Beta-L1" in m5_text and "GO(LEVEL=L1" in m5_text)
check("S3-C 验收矩阵 §15.2 含等级列（L2-only / L1+L2）",
      "L2-only" in matrix_text and "L1+L2" in matrix_text)
check("S3-D 验收矩阵头部含 GO(LEVEL=L1) 判定分级说明", "GO(LEVEL=L1" in matrix_text)
check("S3-E 编号注记（M5-T9 编号例外）已文档化",
      "编号注记" in deps_text and "编号注记" in m5_text)

# --- 4. 状态纪律 ---------------------------------------------------------------
STATE_VOCAB = re.compile(r"^(未开始|进行中|已完成|HOLD|STOP|BLOCKED|NOT_RUN)")
RUNID_PAT = re.compile(r"G\d-\d{8}-\d{6}-[0-9a-f]{4}")
vocab_ok = all(STATE_VOCAB.match(s.strip()) for s in master_status.values())
done_ids = [k for k, v in master_status.items() if v.strip().startswith("已完成")]
done_have_run = all(RUNID_PAT.search(master_status[k]) for k in done_ids)
check("STATE-VOCABULARY 状态词合法且已完成项含 run-id 证据引用", vocab_ok and done_have_run,
      f"bad vocab: {[k for k, v in master_status.items() if not STATE_VOCAB.match(v.strip())]}, "
      f"已完成缺 run-id: {[k for k in done_ids if not RUNID_PAT.search(master_status[k])]}")

# --- 5. 证据与 SYS_CHANGE 机制（ADR-011 D5/D6） ---------------------------------
ev_spec = TASKS / "EVIDENCE_SPEC.md"
ev_index = DOCS / "evidence" / "EVIDENCE_INDEX.md"
check("EVIDENCE-SPEC 规范文件存在", ev_spec.exists())
check("EVIDENCE-INDEX 索引骨架存在", ev_index.exists())
check("EVIDENCE-REFS 三文档引用 EVIDENCE_SPEC",
      all("EVIDENCE_SPEC" in t for t in (agents_text, matrix_text, master_text)))
check("SYS_CHANGE-REFS AGENTS/MASTER/DEPS 含 SYS_CHANGE 机制",
      all("SYS_CHANGE" in t for t in (agents_text, master_text, deps_text)))
check("SYS_CHANGE-AUTH-DISTINCT AGENTS 明确驱动/签名/安全策略不降级为 SYS_CHANGE",
      ("不得降级" in agents_text) or ("不降级" in agents_text))

# --- 6. 场景模拟：Gate 报告可达性 -------------------------------------------------
TERMINAL = {"已完成", "HOLD", "STOP", "BLOCKED", "NOT_RUN"}


def simulate(forced_states: dict[str, str]) -> dict[str, object]:
    """按 ADR-011 语义模拟到不动点。

    规则：① 非 Gate 任务属里程碑 k≥1 时，须 G(k-1)=GO 才能启动（G(k)≠GO ⇒ 下游 BLOCKED）；
    ② TASK_DEP 硬依赖要求上游「已完成」；AGG 依赖/AGG 行要求上游任意终态；
    ③ Gate 行类型为 AGG：上游全部到达终态即产出报告；有 HOLD/STOP/BLOCKED 上游 ⇒ 判 HOLD，
       否则 GO（NOT_RUN 的可选 AUTH 上游不影响 GO——结论显式记录即可）。
    """
    state: dict[str, str] = dict(forced_states)
    gate_verdict: dict[str, str] = {}

    def deps_ok(tid: str, row_is_agg: bool) -> bool:
        for kind, tgt in dep_tokens(deps_blocked_by.get(tid, "")):
            if kind == "gate":
                if gate_verdict.get(tgt.split("=")[0]) != "GO":
                    return False
            elif row_is_agg or kind == "agg":
                if state.get(tgt) not in TERMINAL:
                    return False
            elif state.get(tgt) != "已完成":  # 硬 TASK_DEP
                return False
        return True

    changed = True
    while changed:
        changed = False
        for tid in sorted(all_ids):
            if tid in state or tid in GATE_TASKS:
                continue
            ms = int(tid[1])
            if ms >= 1 and gate_verdict.get(f"G{ms - 1}") != "GO":
                continue  # 上游门未 GO，整个里程碑 BLOCKED
            if not deps_ok(tid, row_is_agg=False):
                continue
            state[tid] = "NOT_RUN" if "AUTH" in deps_type.get(tid, "") else "已完成"
            changed = True
        for gtid, gname in GATE_TASKS.items():
            if gname in gate_verdict:
                continue
            if "AGG" not in deps_type.get(gtid, ""):
                continue
            ms = int(gtid[1])
            if ms >= 1 and gate_verdict.get(f"G{ms - 1}") != "GO":
                continue
            if not deps_ok(gtid, row_is_agg=True):
                continue
            upstream = [t for kind, t in dep_tokens(deps_blocked_by.get(gtid, ""))
                        if kind in ("task", "agg")]
            holdish = [t for t in upstream if state.get(t) in {"HOLD", "STOP", "BLOCKED"}]
            gate_verdict[gname] = "HOLD" if holdish else "GO"
            state[gtid] = "已完成"
            changed = True
    return {"reports": gate_verdict, "state": state}


# S1：M2-T6 / M5-T9 等可选 AUTH 任务 NOT_RUN，其余正常 ⇒ 六个 Gate 全部产出报告
sim1 = simulate({})
missing1 = [g for g in GATE_TASKS.values() if g not in sim1["reports"]]
check("S1-G 场景1（AUTH 任务 NOT_RUN）：六个 Gate 报告全部可达", not missing1,
      f"reports={sim1['reports']}, missing={missing1}")
check("S1-H 场景1：G5 判定为 GO（L1 路径在 M5-T9=NOT_RUN 下可达）",
      sim1["reports"].get("G5") == "GO", f"G5={sim1['reports'].get('G5')}")

# S2：上游失败分支（M0-T3=HOLD）⇒ G0 照常产出 HOLD 报告，G1–G5 无报告（下游 BLOCKED）
sim2 = simulate({"M0-T3": "HOLD"})
check("S2-E 场景2（上游 HOLD）：G0 仍产出报告且判 HOLD",
      sim2["reports"].get("G0") == "HOLD", f"reports={sim2['reports']}")
check("S2-F 场景2：下游 Gate（G1–G5）不产出报告（G(k)≠GO ⇒ 下游 BLOCKED）",
      all(g not in sim2["reports"] for g in ("G1", "G2", "G3", "G4", "G5")),
      f"unexpected reports={[g for g in sim2['reports'] if g != 'G0']}")

# --- 汇总 ----------------------------------------------------------------------
fails = [r for r in results if not r[0]]
width = max(len(lbl) for _, lbl, _ in results)
for ok, lbl, detail in results:
    mark = "PASS" if ok else "FAIL"
    line = f"[{mark}] {lbl}"
    if detail and not ok:
        line += f"  | {detail}"
    print(line)
print("-" * 72)
print(f"TOTAL={len(results)}  PASS={len(results) - len(fails)}  FAIL={len(fails)}")
sys.exit(1 if fails else 0)
