# 许可与分发登记（licensing.md）

> **版本**：v0.1（2026-11-11，M0-T5 建立）
> **定位**：`docs/licensing-sbom.json`（产品级逐包 SBOM，M1-T1 起持续维护、M5-T3 封版）的**人类可读伴生文档**——登记许可结论、分发边界与 vendor 询价触发条件。逐包机器可读条目以 SBOM 为准；冲突时以 SBOM 与官方条款原文为准。
> **依据**：run `G0-20261111-212232-D720`（M0-T5 书面研究）；全部官方来源访问日期 **2026-11-11**（引用清单见该 run `decision.md` §11）。

## 1. 边界声明（先读）

1. **不默认随产品捆绑任何第三方虚拟音频/摄像头驱动**。Track A 交付形态 = 产品文档指引 + 用户自行下载/安装/授权第三方工具；Shutu 不分发、不代装、不代付费。
2. 未来任何捆绑/集成评估 ⇒ 先走本文件对应 vendor 条款通道 + SBOM 逐包登记 + 用户显式批准（AUTH/SYS_CHANGE 语义按 AGENTS §3 执行）。
3. 兼容性/稳定性/时延属 vendor 责任域：兼容性矩阵只写实测结果（基线 §10.2）。
4. 口径红线：Track A（第三方音频桥接）的成功**不得描述为「自研虚拟麦克风驱动完成」**（基线 §2.2/§5.2）。

## 2. SysVAD / Windows-driver-samples（Track B 参考，未构建）

| 项 | 结论 |
|---|---|
| 源码许可 | **MIT**（microsoft/Windows-driver-samples 仓库根 LICENSE） |
| 关键限制 | MIT 只覆盖源码再分发；**驱动要可安装于零售系统必须经微软签名**（HLK dashboard signed / attestation / preproduction / test 四路径，官方 driver-signing-offerings，ms.date 2026-03-23） |
| 构建条件 | VS+SDK+WDK，**v142（VS 2019）工具集** + WIL submodule；原样 sample 有 HLK 预期失败项 ⇒ 原样不可过 HLK 提交 |
| 本项目状态 | research-only；Track B 启动须 pin commit + AUTH_REQUIRED（M2-T6，当前 PENDING） |

## 3. WDK / EWDK（工具链，未安装）

| 项 | 结论 |
|---|---|
| WDK | Microsoft 专有许可；版本配套（VS 2026→WDK 28000.2526；VS 2022→WDK 26100.6584；SDK/WDK build 号一致）；本机按 M0-T1 决策**不安装** |
| EWDK | **独立许可条款**（下载页即 enterprise-wdk-license-2026 条款页）；自包含 ISO 命令行构建环境；隔离测试机候选构建路径——**采用前须审全文条款** |
| 分发 | 均为构建/测试工具，不随产品分发；WDK 内可再分发组件在实际使用时逐项登记 SBOM |

## 4. Track A 第三方候选（许可与分发，调研完成 2026-11-11）

| 候选 | vendor | 许可 | 终端用户成本 | 捆绑/分发通道 | 待办（若评估捆绑） |
|---|---|---|---|---|---|
| **VB-CABLE** | VB-Audio（V. Burel，©1998-2026） | Donationware（自定价） | 免费/捐赠 | webshop **Volume Licensing**（官方原文 "Change the quantity in the cart to get Volume Licensing prices"） | vendor 书面条款（专用分发条款页不可达，调研限制已登记）+ 量购报价 |
| **VoiceMeeter** | VB-Audio | Donationware License（30 天后邀购，可继续评估） | 免费/捐赠 | 同 VB-Audio 条款体系（Volume Licensing） | 同上 |
| **Virtual Audio Cable（VAC）** | Eugene Muzychenko | 商业：Home $30 / Business $50（单份；量购 $15–50）；一次性付款**永久授权** | 付费 | **官方 custom（proprietary）version**——可自定义名称与密钥、仅可与客户应用一同使用；另有 source code license | custom/source license 商业协议 + 报价 |

- 三候选均为**第三方内核级音频驱动/虚拟设备**：用户安装需管理员权限 + 重启（产品文档须如实告知）。
- 隐私事实（vendor 官方声明，未实测验证）：VoiceMeeter 不联网、不收集用户数据（COO France）；VAC 驱动与应用不访问网络功能。
- **清单 ≠ 已验证兼容**：实测留 M2-T3；结果进兼容性矩阵与 NFR-02（Track A 实测口径）。

## 5. 与任务体系的挂钩

| 任务 | 引用本文件的内容 |
|---|---|
| M2-T3（Track A 音频桥接） | §4 候选清单 + §1 边界声明（实测前重读） |
| M2-T6 / M2-T7（Track B / G2 显式结论） | §2 SysVAD 签名限制 + run decision.md §6/§8 |
| M5-T3（SBOM 封版/签名计划） | §3 WDK/EWDK 条款 + run decision.md §4 签名矩阵（EV/Partner Center 前置） |
| M5-T6（音频交付形态决策） | §1/§4（Track A 口径与捆绑通道） |

## 6. 维护规则

- 新增依赖/候选 ⇒ 先登记 `docs/licensing-sbom.json`（逐包），必要时在本文件补人类可读结论（AGENTS §6：不得以「项目根仓库开源」推断全部权重/插件可免费商用）。
- vendor 条款变更 ⇒ 更新本文件与 SBOM 并注明变更日期与来源；**不得静默覆盖**历史结论（追加新条目）。
- 版本历史：v0.1（2026-11-11，M0-T5 初建：SysVAD/WDK/EWDK + Track A 三候选）。
