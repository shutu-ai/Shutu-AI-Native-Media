# test-enumeration.ps1 — M0-T2 虚拟摄像头枚举断言（自动化测试）
#
# 枚举 Windows 视频捕获设备（WinRT DeviceInformation, 与官方 Sample 同一 selector），
# 按 DEVPKEY_DeviceInterface_IsVirtualCamera ({6EDC630D-C2E3-43B7-B2D1-20525A1AF120},3)
# 判定虚拟摄像头（与官方 VCamUtils::IsVirtualCamera 同源），断言:
#   -ExpectPresent: 目标虚拟摄像头存在       → 退出码 0；不存在 → 2
#   -ExpectAbsent : 目标虚拟摄像头不存在     → 退出码 0；仍存在 → 2
#   枚举本身失败 → 3。退出码可被 CI/脚本复用。
# 诊断输出: 所有摄像头设备行 + COM 注册状态（HKLM CLSID InProcServer32）。
#
# 运行环境: Windows PowerShell 5.1（WinRT 投影原生支持）:
#   powershell.exe -NoProfile -ExecutionPolicy Bypass -File test-enumeration.ps1 -ExpectPresent
# 只读操作——不修改任何系统状态。
[CmdletBinding()]
param(
    [switch]$ExpectPresent,
    [switch]$ExpectAbsent,
    [string]$NameFilter = 'SWVCamMediaSource',
    [string]$ComClsid = '{7B89B92E-FE71-42D0-8A41-E137D06EA184}'
)

$ErrorActionPreference = 'Stop'
if (-not ($ExpectPresent -or $ExpectAbsent)) { Write-Error '必须指定 -ExpectPresent 或 -ExpectAbsent'; exit 3 }

# --- WinRT 异步辅助（PS 5.1） -----------------------------------------------------------
if ($PSVersionTable.PSVersion.Major -ge 7) { Write-Error '请在 Windows PowerShell 5.1 (powershell.exe) 下运行（WinRT 投影）'; exit 3 }
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await-WinRt($WinRtOperation, [type]$ResultType) {
    $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
    $task = $asTask.Invoke($null, @($WinRtOperation))
    $task.Wait(-1) | Out-Null
    return $task.Result
}

[Windows.Devices.Enumeration.DeviceInformation, Windows.Devices.Enumeration, ContentType = WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceInformationCollection, Windows.Devices.Enumeration, ContentType = WindowsRuntime] | Out-Null
[Windows.Media.Devices.MediaDevice, Windows.Media.Devices, ContentType = WindowsRuntime] | Out-Null

# --- 枚举视频捕获设备 -------------------------------------------------------------------
$isVcamPropKey = '{6EDC630D-C2E3-43B7-B2D1-20525A1AF120} 3'
try {
    $selector = [Windows.Media.Devices.MediaDevice]::GetVideoCaptureSelector()
    $propKeys = [string[]]@($isVcamPropKey)
    $op = [Windows.Devices.Enumeration.DeviceInformation]::FindAllAsync($selector, $propKeys)
    $devices = Await-WinRt $op ([Windows.Devices.Enumeration.DeviceInformationCollection])
} catch {
    Write-Error "枚举失败: $($_.Exception.Message)"
    exit 3
}

$vcams = @()
Write-Output ("# enumeration at {0}  devices={1}" -f (Get-Date -Format s), $devices.Count)
foreach ($d in $devices) {
    $isVcam = $false
    foreach ($kv in $d.Properties) {
        if ($kv.Key -eq $isVcamPropKey -and $null -ne $kv.Value) { $isVcam = [bool]$kv.Value }
    }
    # 兜底: OS 虚拟摄像头总线设备实例 ID 前缀 (SWD\VCAMDEVAPI)
    if (-not $isVcam) {
        $devId = ''
        foreach ($kv in $d.Properties) { if ($kv.Key -eq 'System.Devices.DeviceInstanceId') { $devId = [string]$kv.Value } }
        if ($devId -like 'SWD\VCAMDEVAPI*') { $isVcam = $true }
    }
    Write-Output ("DEV`t{0}`t{1}`t{2}" -f ($(if ($isVcam) { 'VCAM' } else { 'PHYS' })), $d.Name, $d.Id)
    if ($isVcam -and $d.Name -like "*$NameFilter*") { $vcams += $d }
}

# --- COM 注册诊断（只读） ---------------------------------------------------------------
$comReg = 'registered=unknown'
try {
    $k = Get-ItemProperty -Path ("HKLM:\SOFTWARE\Classes\CLSID\" + $ComClsid + "\InProcServer32") -ErrorAction Stop
    $comReg = "registered=yes path=$($k.'(default)')"
} catch { $comReg = 'registered=no' }
Write-Output "COM`t$comReg"

# --- 断言 --------------------------------------------------------------------------------
if ($ExpectPresent) {
    if ($vcams.Count -ge 1) {
        Write-Output ("ASSERT`tPRESENT`tOK`tcount={0} names={1}" -f $vcams.Count, ($vcams.Name -join ', '))
        exit 0
    }
    Write-Output 'ASSERT`tPRESENT`tFAIL`tdevice not found'
    exit 2
}
if ($ExpectAbsent) {
    if ($vcams.Count -eq 0) {
        Write-Output ("ASSERT`tABSENT`tOK`tcount=0")
        exit 0
    }
    Write-Output ("ASSERT`tABSENT`tFAIL`tdevice still present: {0}" -f ($vcams.Name -join ', '))
    exit 2
}
