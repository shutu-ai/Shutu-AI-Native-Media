# fetch-sample.ps1 — M0-T2 Sample 获取脚本（可复现）
# 克隆 microsoft/Windows-Camera 官方仓库并固定到已记录的 commit，
# 输出 provenance 摘要（URL / commit / 许可证）供 SBOM 与复现文档引用。
#
# 用法:
#   pwsh -NoProfile -File platforms/windows/virtual-camera/fetch-sample.ps1 `
#        [-RepoUrl <url>] [-Commit <sha>] [-CloneDir <path>]
#
# 语义: 只克隆与校验，不构建、不注册、不修改任何系统状态。
[CmdletBinding()]
param(
    [string]$RepoUrl = 'https://github.com/microsoft/Windows-Camera.git',
    [string]$Commit = '626f8b19c5f367602f2e89c6b314573d3776c9df',
    [string]$CloneDir = (Join-Path (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $PSScriptRoot))) 'third_party\Windows-Camera')
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path (Join-Path $CloneDir '.git'))) {
    Write-Host ">> git clone $RepoUrl -> $CloneDir"
    git clone $RepoUrl $CloneDir
    if ($LASTEXITCODE -ne 0) { throw "git clone failed with exit code $LASTEXITCODE" }
}

git -C $CloneDir fetch origin $Commit 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Warning 'fetch commit failed (shallow clone?); trying full fetch'; git -C $CloneDir fetch origin | Out-Null }

Write-Host ">> git checkout $Commit"
git -C $CloneDir checkout $Commit
if ($LASTEXITCODE -ne 0) { throw "git checkout failed with exit code $LASTEXITCODE" }

$head = (git -C $CloneDir rev-parse HEAD).Trim()
if ($head -ne $Commit) { throw "HEAD=$head does not match pinned commit $Commit" }

$licenseHash = (Get-FileHash (Join-Path $CloneDir 'LICENSE') -Algorithm SHA256).Hash.ToLower()

[pscustomobject]@{
    repo_url      = $RepoUrl
    commit        = $head
    license_file  = 'LICENSE'
    license_spdx  = 'MIT'
    license_sha256 = $licenseHash
    sample_path   = 'Samples/VirtualCamera'
    cloned_at     = (Get-Date).ToString('s')
} | ConvertTo-Json -Compress | Write-Output
