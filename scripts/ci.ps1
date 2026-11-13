# scripts/ci.ps1 — 本地 CI（M1-T1 步骤 3：fmt --check + clippy + test，全部真实 MSVC 工具链）
# 用法: pwsh -NoProfile -File scripts/ci.ps1
# 说明: 仅进程级设置 PATH（不污染系统状态）；PKG_CONFIG 变量由 .cargo/config.toml 提供。
$ErrorActionPreference = "Stop"
$gstBin = "C:\gstreamer\1.0\msvc_x86_64\bin"

if (-not (Test-Path "$gstBin\gstreamer-1.0-0.dll")) {
    throw "GStreamer MSVC SDK not found at $gstBin (see docs/platform-windows.md)"
}
$env:PATH = "$gstBin;$env:PATH"
Set-Location (Join-Path $PSScriptRoot "..")

Write-Host "== cargo fmt --all --check =="
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "fmt check FAILED (fix with: cargo fmt --all)" }

Write-Host "== cargo clippy --workspace --all-targets -- -D warnings =="
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "clippy FAILED" }

Write-Host "== cargo test --workspace =="
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "test FAILED" }

Write-Host "CI GREEN: fmt + clippy(-D warnings) + test"
