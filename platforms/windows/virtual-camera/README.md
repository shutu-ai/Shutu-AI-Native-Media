# Windows 虚拟摄像头（M0-T2）：官方 Sample 构建 / 注册 / 验证 / 注销 可复现步骤

> 来源：microsoft/Windows-Camera `Samples/VirtualCamera`（Synthetic/SimpleMediaSource 模式），**原样构建，无任何源码修改**。
> 本文档由 run `G0-20261111-182351-1DB8`（2026-11-11）真机执行固话，环境见该 run `environment.json`。
> ⚠️ 本目录是**参考实现验证**（基线 §2.1），不是产品代码；产品 MediaSource 自 M1-T5 起自研。

## 0. 前置条件（本机实测版本）

| 项 | 版本 | 备注 |
|---|---|---|
| Windows | 11 家庭中文版 25H2，Build ≥ 22000 | `MFCreateVirtualCamera` 门槛 |
| VS BuildTools | 2022 17.14（MSVC v143 14.44） | 含 C++ 生成工具与 Windows SDK |
| Windows SDK | 10.0.26100.0 | Sample 所需 |
| nuget.exe | 8.0.0.137 | 本机 NuGet.Config `packageSources` 为空 → restore 需显式 `-Source`（不改配置） |
| PowerShell | pwsh 7+（脚本）+ Windows PowerShell 5.1（WinRT 枚举断言） | |

权限提示：步骤 3/4/6 需要管理员（HKLM 写入、Program Files 写入）；UAC 每次弹窗须用户批准。
红线：全程**不安装内核驱动、不启用测试签名、不修改任何安全策略**。

## 1. 获取源码（固定 commit）

```powershell
pwsh -NoProfile -File platforms\windows\virtual-camera\fetch-sample.ps1 `
    -Destination third_party\Windows-Camera
# 克隆 https://github.com/microsoft/Windows-Camera 并 checkout
# 626f8b19c5f367602f2e89c6b314573d3776c9df，校验 HEAD 后输出来源 JSON（MIT）
```

## 2. 构建（Release x64）

```powershell
pwsh -NoProfile -File platforms\windows\virtual-camera\build-sample.ps1
# 内部：vswhere 定位 VS2022 → vcvars64 → nuget restore（显式 -Source，包：
#   Microsoft.Windows.CppWinRT 3.0.260520.1 / ImplementationLibrary 1.0.260126.7 / VCRTForwarders.140 1.1.0）
#   → set SolutionDir → msbuild 两个 vcxproj（Installer + MediaSource）
```

产物（本 run 实测哈希，见 run `test-results.json`）：
- `x64\Release\VirtualCamera_Installer.exe`（159,232 B）
- `x64\Release\VirtualCameraMediaSource.dll`（1,234,432 B，app-local VCRT forwarders 同目录）

已知无害告警：C4819（CP936 代码页对 UTF-8 头文件）；MSI 工程 `.vdproj` 需 VS IDE 扩展，BuildTools 机器不可构建（见 decision.md §3 偏差）。

## 3. 部署 COM 注册（管理员；替代 vdproj 的 README 文档化路径）

HKLM 键值（与官方 vdproj 逐项一致）：

```
HKLM\SOFTWARE\Classes\CLSID\{7B89B92E-FE71-42D0-8A41-E137D06EA184}
  (default) = VirtualCameraMediaSource
HKLM\SOFTWARE\Classes\CLSID\{7B89B92E-FE71-42D0-8A41-E137D06EA184}\InProcServer32
  (default) = C:\Program Files\Shutu-M0T2-VirtualCameraSample\VirtualCameraMediaSource.dll
  ThreadingModel = Both
```

run 内 `staging\com-register.cmd` + `staging\copy-dll.cmd` 为上述操作的落档脚本（经 UAC 提权执行）。**本步骤属 SYS_CHANGE**：执行前单次向用户确认并记入 decision.md。

## 4. 生命周期循环（官方 Installer 控制台，stdin 驱动）

驱动脚本：`platforms\windows\virtual-camera\test-lifecycle.ps1`（每轮 Phase A/B 各触发一次 UAC）：

```powershell
# Phase A: 注册实例 + TestVCam 真实拉流 + 枚举断言 PRESENT
powershell.exe -NoProfile -ExecutionPolicy Bypass -File platforms\windows\virtual-camera\test-lifecycle.ps1 `
    -Phase A -Cycle <N> -RunDir <run目录> -BinDir third_party\Windows-Camera\Samples\VirtualCamera\x64\Release `
    -EnumScript platforms\windows\virtual-camera\test-enumeration.ps1
# Phase B: 移除实例 + 枚举断言 ABSENT（同上，-Phase B）
```

官方 Installer stdin 序列（**禁盲目重试**——stdin 错位会误触发注册）：
- 注册：`1↵1↵1↵1↵5↵`（register → SimpleMediaSource → System lifetime → CurrentUser → quit）
- 拉流：`3↵1↵1↵1↵0↵5↵`（TestVCam → 设备1 → 流1 → 媒体类型1(NV12 640x480@30) → 退出循环 → quit）
- 移除：`2↵1↵5↵`

通过标准（官方工具自校验）：`Read 30 samples ... Streaming validation passed!`

## 5. 枚举断言（可复用退出码：0 成立 / 2 不成立 / 3 枚举失败）

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File platforms\windows\virtual-camera\test-enumeration.ps1 -ExpectPresent
powershell.exe -NoProfile -ExecutionPolicy Bypass -File platforms\windows\virtual-camera\test-enumeration.ps1 -ExpectAbsent
```

判据：`DEVPKEY_DeviceInterface_IsVirtualCamera`（`{6EDC630D-C2E3-43B7-B2D1-20525A1AF120},3`，与官方 `VCamUtils::IsVirtualCamera` 同源），兜底 `SWD\VCAMDEVAPI` 实例 ID 前缀。设备名形如 `SWVCamMediaSource (Windows 虚拟摄像头)`。

## 6. Windows Camera 渲染验证（真机验收）

1. 启动 `microsoft.windows.camera:`（首启会弹麦克风授权——预览不需要麦克风，选「否」可继续）。
2. UIA 点按 `SwitchCameraButtonId`（"更改摄像头"）循环切换至 `SWVCamMediaSource`（本机切换序列 4 位置，最多 4 次）。
3. 判别合成画面（避免人眼误差）：`artifacts/<run>/staging/analyze-preview.ps1` 像素判别——合成帧行内标准差 = 0.00（物理画面实测 8–13）+ 锯齿剖面 + 位移 1 行/秒（`SimpleFrameGenerator.cpp:85`，`gray=(row+秒)%256`）。
4. 截图留证：`staging\capture-window.ps1`（PrintWindow 窗口级，规避无关窗口内容入镜，符合 §11-5）。

## 7. 完整回滚（SYS_CHANGE 终态）

```text
VirtualCamera_Installer.exe /Uninstall        # 官方清理例程（run staging\cleanup.cmd 落档）
reg delete "HKLM\SOFTWARE\Classes\CLSID\{7B89B92E-FE71-42D0-8A41-E137D06EA184}" /f
rmdir /s /q "C:\Program Files\Shutu-M0T2-VirtualCameraSample"
```

已知坑：**Frame Server（svchost）会缓存已注销 MediaSource 的 DLL** → 目录删除报「拒绝访问」。处置（SYS_CHANGE，需用户确认）：`sc stop frameserver`（按需启动服务，停止=空闲常态）后删除即可。终态断言：

```powershell
test-enumeration.ps1 -ExpectAbsent   # → ASSERT ABSENT OK, COM registered=no
```

## 8. 本次 run 的实测事实（详见 run decision.md）

- 3 轮循环全通过；设备实例 ID 逐轮一致（确定性哈希）。
- Windows Camera 切换序列 4 位置（2 物理 + 1 虚拟 + 1 疑似多配置文件物理入口）→ M0-T3 复核。
- 无害告警：`0x80073D57`（非打包应用调 WinRT）、卸载期 `0x80070002 CustomCaptureSourceClsid`（CustomControl 变体键不存在）。
