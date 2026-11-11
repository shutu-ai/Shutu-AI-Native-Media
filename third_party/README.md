# third_party/

第三方上游仓库克隆目录（克隆体不入库——见 `.gitignore`；来源、commit、许可证逐包登记于 run 级 `licensing-sbom.json`，索引见 `docs/evidence/EVIDENCE_INDEX.md`）。

| 目录 | 上游 | 固定 commit | 许可证 | 用途 | 状态 |
|---|---|---|---|---|---|
| `Windows-Camera/` | https://github.com/microsoft/Windows-Camera | `626f8b19c5f367602f2e89c6b314573d3776c9df` | MIT | M0-T2：官方 VirtualCamera Sample 原样构建验证 | M0-T2 已完成（run G0-20261111-182351-1DB8）；复现步骤见 `platforms/windows/virtual-camera/README.md` |
