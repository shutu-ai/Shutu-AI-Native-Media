# test-lifecycle.ps1 — M0-T2 生命周期驱动（注册/拉流/移除, 提权经 UAC）
# 用官方 VirtualCamera_Installer.exe（原样构建产物, 交互式 stdin 驱动）执行:
#   Phase A: 注册 (SimpleMediaSource, System 生命周期, CurrentUser) + 等待 + TestVCam 真实拉流验证
#   Phase B: 移除
# 用法:
#   powershell.exe -NoProfile -ExecutionPolicy Bypass -File test-lifecycle.ps1 -Phase A -Cycle 1 `
#       -RunDir <run 目录> -BinDir <构建输出目录> -EnumScript <test-enumeration.ps1 路径>
# 说明: -Verb RunAs 每次触发一次 UAC; 日志写入 <RunDir>\logs\; 枚举断言追加写入 <RunDir>\device-enumeration.txt
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('A', 'B')][string]$Phase,
    [Parameter(Mandatory = $true)][int]$Cycle,
    [Parameter(Mandatory = $true)][string]$RunDir,
    [Parameter(Mandatory = $true)][string]$BinDir,
    [Parameter(Mandatory = $true)][string]$EnumScript,
    [string]$PropagateDelaySeconds = '6'
)

$ErrorActionPreference = 'Stop'
$staging = Join-Path $RunDir 'staging'
$logs = Join-Path $RunDir 'logs'
foreach ($d in @($staging, $logs)) { if (-not (Test-Path $d)) { New-Item -ItemType Directory -Path $d | Out-Null } }

$exe = Join-Path $BinDir 'VirtualCamera_Installer.exe'
if (-not (Test-Path $exe)) { throw "Installer exe not found: $exe" }

# --- stdin 序列（官方交互菜单; 明确不盲目重试以防 stdin 错位触发误注册） ------------------
$stdinRegister = "1`r`n1`r`n1`r`n1`r`n5`r`n"     # register→Simple→System→CurrentUser→quit
$stdinStreamTest = "3`r`n1`r`n1`r`n1`r`n0`r`n5`r`n"  # TestVCam→dev1→stream1→mediatype1→exit-loop→quit
$stdinRemove = "2`r`n1`r`n5`r`n"                 # remove→dev1→quit

function Write-StagingFiles {
    Set-Content -Path (Join-Path $staging 'stdin-register.txt') -Value $stdinRegister -NoNewline -Encoding ascii
    Set-Content -Path (Join-Path $staging 'stdin-streamtest.txt') -Value $stdinStreamTest -NoNewline -Encoding ascii
    Set-Content -Path (Join-Path $staging 'stdin-remove.txt') -Value $stdinRemove -NoNewline -Encoding ascii
}

function New-CycleCmd {
    param([string]$Path, [string]$Body)
    Set-Content -Path $Path -Value $Body -Encoding ascii
}

Write-StagingFiles

if ($Phase -eq 'A') {
    $cmdPath = Join-Path $staging ("cycle{0}-a.cmd" -f $Cycle)
    $body = "@echo off`r`n" +
        ('"' + $exe + '" < "' + (Join-Path $staging 'stdin-register.txt') + '" > "' + (Join-Path $logs ("cycle{0}-register.log" -f $Cycle)) + '" 2>&1' + "`r`n") +
        ('if errorlevel 1 echo [register exit !=0] >> "' + (Join-Path $logs ("cycle{0}-register.log" -f $Cycle)) + '" && exit /b 1' + "`r`n") +
        ('timeout /t ' + $PropagateDelaySeconds + ' /nobreak >nul' + "`r`n") +
        ('"' + $exe + '" < "' + (Join-Path $staging 'stdin-streamtest.txt') + '" > "' + (Join-Path $logs ("cycle{0}-streamtest.log" -f $Cycle)) + '" 2>&1' + "`r`n") +
        "exit /b 0`r`n"
    New-CycleCmd $cmdPath $body
    Write-Host ">> [cycle $Cycle / Phase A] 提权执行注册+拉流验证 (UAC) ..."
    Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', $cmdPath -Verb RunAs -Wait | Out-Null
} else {
    $cmdPath = Join-Path $staging ("cycle{0}-b.cmd" -f $Cycle)
    $body = "@echo off`r`n" +
        ('"' + $exe + '" < "' + (Join-Path $staging 'stdin-remove.txt') + '" > "' + (Join-Path $logs ("cycle{0}-remove.log" -f $Cycle)) + '" 2>&1' + "`r`n") +
        "exit /b 0`r`n"
    New-CycleCmd $cmdPath $body
    Write-Host ">> [cycle $Cycle / Phase B] 提权执行移除 (UAC) ..."
    Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', $cmdPath -Verb RunAs -Wait | Out-Null
}

# --- 枚举断言（非提权, 当前用户视角） ----------------------------------------------------
$expect = if ($Phase -eq 'A') { '-ExpectPresent' } else { '-ExpectAbsent' }
$enumOut = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $EnumScript $expect.Split(' ')[0] 2>&1
$exitCode = $LASTEXITCODE
$stamp = ("# lifecycle cycle={0} phase={1} at {2}" -f $Cycle, $Phase, (Get-Date -Format s))
Add-Content -Path (Join-Path $RunDir 'device-enumeration.txt') -Value ($stamp + [Environment]::NewLine + ($enumOut -join [Environment]::NewLine))
Write-Output $enumOut
Write-Host (">> [cycle {0} / Phase {1}] enumeration assert exit={2}" -f $Cycle, $Phase, $exitCode)
exit $exitCode
