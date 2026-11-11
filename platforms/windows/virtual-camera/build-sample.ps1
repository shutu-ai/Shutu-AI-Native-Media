# build-sample.ps1 — M0-T2 Sample 构建脚本（可复现）
# 按官方 README 构建流程，在 VS2022 BuildTools (v143 + SDK 10.0.26100.0) 下构建:
#   1. VirtualCamera_Installer   (Release x64, 官方注册/移除/拉流测试控制台)
#   2. VirtualCameraMediaSource  (Release x64, 合成帧 MediaSource DLL)
# 官方 README 的第 3/4/6 步（VirtualCamera_MSI、Systray/WinRT/Manager App）
# 不在本脚本范围: .vdproj 需 VS IDE Installer Projects 扩展（本机仅 BuildTools），
# GUI 管理器对本任务非必需——偏差与替代路径记录在 run 的 decision.md。
#
# 用法:
#   pwsh -NoProfile -File platforms/windows/virtual-camera/build-sample.ps1 -BuildLog <path>
[CmdletBinding()]
param(
    [string]$RepoDir = (Join-Path (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))) 'third_party\Windows-Camera'),
    [string]$Configuration = 'Release',
    [string]$Platform = 'x64',
    [Parameter(Mandatory = $true)][string]$BuildLog,
    [string]$NuGetExe
)

$ErrorActionPreference = 'Stop'

# --- 定位 MSBuild（显式选 VS2022 BuildTools: 工程要求 v143 工具集） -------------------
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found' }

$msbuild2022 = & $vswhere -version '[17.0,18.0)' -products * -requires Microsoft.Component.MSBuild `
    -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
if (-not $msbuild2022 -or -not (Test-Path $msbuild2022)) { throw 'VS2022 (17.x) MSBuild.exe not found' }
Write-Host ">> MSBuild: $msbuild2022"

$vcvars = Join-Path (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $msbuild2022)))) `
    'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "vcvars64.bat not found at $vcvars" }

$projects = @(
    'VirtualCamera_Installer\VirtualCamera_Installer.vcxproj',
    'VirtualCameraMediaSource\VirtualCameraMediaSource.vcxproj'
)
$sampleRoot = Join-Path $RepoDir 'Samples\VirtualCamera'

# --- 还原 + 构建（vcvars 环境内单次 cmd 调用，完整输出进 BuildLog） --------------------
# 恢复用 nuget.exe CLI + 显式 -Source（本机用户级 NuGet.Config packageSources 为空，
# 不能依赖默认配置；nuget.exe 路径由 -NuGetExe 传入或自动定位 run 目录）
# 注意: 含内嵌引号的行先拼成独立变量再入数组——@('a', 'b' + $x) 的逗号会把 + 表达式拆开
if (-not $NuGetExe) {
    $runTools = Get-ChildItem -Path (Join-Path (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))) 'artifacts') `
        -Recurse -Filter 'nuget.exe' -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $runTools) { throw 'nuget.exe not found; pass -NuGetExe <path>' }
    $NuGetExe = $runTools.FullName
}
Write-Host ">> NuGet: $NuGetExe"

$callLine = 'call "' + $vcvars + '"'
$restoreLine = '"' + $NuGetExe + '" restore "' + (Join-Path $sampleRoot 'VirtualCamera_Installer\VirtualCamera_Installer.vcxproj') + '" -SolutionDirectory "' + $sampleRoot + '" -Source https://api.nuget.org/v3/index.json -NonInteractive'
# 独立构建 vcxproj 时 $(SolutionDir) 未定义→NuGet.targets 在项目目录找 packages;
# 经环境变量注入 SolutionDir（含尾反斜杠, 包目录按 sln 约定落在 Samples\VirtualCamera\packages\）
$solDirLine = 'set "SolutionDir=' + $sampleRoot + '\"'
$cmdLines = @('@echo off', $callLine, $restoreLine, $solDirLine, 'if errorlevel 1 exit /b 1')
foreach ($p in $projects) {
    $full = Join-Path $sampleRoot $p
    $msbLine = 'msbuild "' + $full + '" -p:Configuration=' + $Configuration + ' -p:Platform=' + $Platform + ' -m -v:m -nologo'
    $cmdLines += $msbLine
    $cmdLines += 'if errorlevel 1 exit /b 1'
}
$cmdFile = Join-Path $env:TEMP 'm0t2-build.cmd'
Set-Content -Path $cmdFile -Value $cmdLines -Encoding ascii
Write-Host ">> cmd file: $cmdFile"

try {
    Write-Host ">> msbuild restore+build ($Configuration|$Platform): $($projects -join ', ')"
    & cmd.exe /c $cmdFile 1> $BuildLog 2> ($BuildLog + '.err')
    $exit = $LASTEXITCODE
    # 合并 stderr 到日志尾部（msbuild 错误走 stdout；stderr 一般只有 vcvars 噪声）
    if ((Test-Path ($BuildLog + '.err')) -and ((Get-Item ($BuildLog + '.err')).Length -gt 0)) {
        Add-Content -Path $BuildLog -Value "`n--- stderr ---"
        Get-Content ($BuildLog + '.err') | Add-Content -Path $BuildLog
    }
    Remove-Item ($BuildLog + '.err') -ErrorAction SilentlyContinue
    if ($exit -ne 0) { throw "msbuild failed with exit code $exit (see $BuildLog)" }
}
finally {
    # 失败排查用: 保留 $env:TEMP\m0t2-build.cmd, 不删除
}

# --- 输出工件路径 --------------------------------------------------------------------
$artifacts = @(
    (Join-Path $sampleRoot "x64\$Configuration\VirtualCamera_Installer.exe"),
    (Join-Path $sampleRoot "x64\$Configuration\VirtualCameraMediaSource.dll"),
    (Join-Path $sampleRoot "VirtualCameraMediaSource\x64\$Configuration\VirtualCameraMediaSource.dll")
)
foreach ($a in $artifacts) {
    if (Test-Path $a) {
        $h = (Get-FileHash $a -Algorithm SHA256).Hash.ToLower()
        Write-Output ("ARTIFACT`t{0}`t{1}`t{2}" -f $a, (Get-Item $a).Length, $h)
    }
}
Write-Host ">> build OK, log: $BuildLog"
