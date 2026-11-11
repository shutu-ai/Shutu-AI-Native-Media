# elevate-run.ps1 — 以管理员运行指定 .cmd（触发一次 UAC），等待完成
# pwsh -NoProfile -File elevate-run.ps1 -CmdPath <path>
param([Parameter(Mandatory = $true)][string]$CmdPath)
if (-not (Test-Path $CmdPath)) { throw "not found: $CmdPath" }
$p = Start-Process -FilePath 'cmd.exe' -ArgumentList '/c', "`"$CmdPath`"" -Verb RunAs -Wait -PassThru
Write-Output ("elevated cmd exit={0}" -f $p.ExitCode)
