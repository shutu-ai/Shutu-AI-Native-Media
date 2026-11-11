#!/usr/bin/env pwsh
<#
.SYNOPSIS
  M0-T1 只读环境探测脚本（Shutu AI-Native Media）。
.DESCRIPTION
  生成符合 docs/tasks/EVIDENCE_SPEC.md 与基线 §10.3 的 environment.json。
  只读探测：不安装、不修改任何系统状态（SYS_CHANGE 范围之外）。
  任何探测失败都会记录在 probes_failed[]，绝不静默吞错；无法探测的事实
  以 status="unconfirmed" 显式记录，不编造。
.NOTES
  运行：pwsh -NoProfile -File verify-environment.ps1 -OutFile <path>\environment.json
  需要 PowerShell 7+（pwsh）。幂等性：重复运行除 volatile 字段外结果一致。
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutFile,

    [string]$RunId = ""
)

$ErrorActionPreference = 'Continue'
$probesFailed = [System.Collections.Generic.List[string]]::new()

# 每个探测包装：失败时记录探针名 + 错误，返回 $null，绝不抛出中断整体。
function Invoke-Probe {
    param([string]$Name, [scriptblock]$Block)
    try {
        $result = & $Block
        if ($null -ne $result) { return $result }
        $probesFailed.Add("$Name (no result)") | Out-Null
        return $null
    } catch {
        $probesFailed.Add("$Name ($($_.Exception.Message))") | Out-Null
        return $null
    }
}

function Get-VersionOutput {
    param([string]$Exe, [string[]]$ArgList = @('--version'))
    if ([string]::IsNullOrEmpty($Exe)) { return $null }
    $resolved = Get-Command $Exe -ErrorAction SilentlyContinue
    if ($null -eq $resolved) { return $null }
    try {
        $out = & $resolved.Source @ArgList 2>&1 | ForEach-Object { $_.ToString() }
        return ($out -join "`n").Trim()
    } catch { return $null }
}

$psVersion = $PSVersionTable.PSVersion.ToString()

# ---------- OS ----------
$osReg = Invoke-Probe 'os.registry' {
    Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' -ErrorAction Stop
}
$osInfo = Invoke-Probe 'os.cim' { Get-CimInstance Win32_OperatingSystem -ErrorAction Stop }

$osSection = [ordered]@{
    caption         = if ($osInfo) { $osInfo.Caption } else { $null }
    version         = if ($osReg) { "$($osReg.CurrentMajorVersionNumber).$($osReg.CurrentMinorVersionNumber)" } else { $null }
    build           = if ($osReg) { [string]$osReg.CurrentBuildNumber } else { $null }
    ubr             = if ($osReg) { [int]$osReg.UBR } else { $null }
    display_version = if ($osReg) { $osReg.DisplayVersion } else { $null }
    edition         = if ($osReg) { $osReg.ProductName } else { $null }
    edition_id      = if ($osReg) { $osReg.EditionID } else { $null }
    meets_22000     = if ($osReg) { ([int]$osReg.CurrentBuildNumber + [int]$osReg.UBR / 65536) -ge 22000 } else { $null }
}

# ---------- 硬件 ----------
$cpu = Invoke-Probe 'hw.cpu' { Get-CimInstance Win32_Processor -ErrorAction Stop | Select-Object -First 1 }
$gpus = Invoke-Probe 'hw.gpu' {
    Get-CimInstance Win32_VideoController -ErrorAction Stop |
        ForEach-Object { [ordered]@{ name = $_.Name; driver_version = $_.DriverVersion; driver_date = $_.DriverDate } }
}
$cs = Invoke-Probe 'hw.ram' { Get-CimInstance Win32_ComputerSystem -ErrorAction Stop }
$osFreeKB = Invoke-Probe 'hw.ram.free' { (Get-CimInstance Win32_OperatingSystem -ErrorAction Stop).FreePhysicalMemory }
$disks = Invoke-Probe 'hw.disk' {
    Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' -ErrorAction Stop |
        ForEach-Object { [ordered]@{ drive = $_.DeviceID; volume = $_.VolumeName; total_gb = [math]::Round($_.Size / 1GB, 1); free_gb = [math]::Round($_.FreeSpace / 1GB, 1); volatile = $true } }
}

# ---------- 开发工具链 ----------
$gitV = Invoke-Probe 'tool.git' { Get-VersionOutput 'git' }
$pythonV = Invoke-Probe 'tool.python' { Get-VersionOutput 'python' }
$cmakeV = Invoke-Probe 'tool.cmake' { Get-VersionOutput 'cmake' }
$rustcV = Invoke-Probe 'tool.rustc' { Get-VersionOutput 'rustc' }
$cargoV = Invoke-Probe 'tool.cargo' { Get-VersionOutput 'cargo' }
$rustupV = Invoke-Probe 'tool.rustup' { Get-VersionOutput 'rustup' @('show', 'active-toolchain') }
$rustHost = Invoke-Probe 'tool.rust.host' {
    $vv = (Get-VersionOutput 'rustc' @('-vV'))
    if ($vv -match 'host:\s*(.+)') { $Matches[1].Trim() } else { $null }
}

# ---------- Visual Studio / MSVC ----------
$vswhereCandidates = @(
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe",
    "${env:ProgramFiles}\Microsoft Visual Studio\Installer\vswhere.exe"
)
$vswhere = $vswhereCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
$vsInstances = Invoke-Probe 'vs.instances' {
    if (-not $vswhere) { throw 'vswhere.exe not found' }
    $raw = & $vswhere -all -products * -format json | ConvertFrom-Json
    $raw | ForEach-Object {
        $instPath = $_.installationPath
        $toolsets = @()
        $clBanners = @()
        $msvcRoot = Join-Path $instPath 'VC\Tools\MSVC'
        if (Test-Path $msvcRoot) {
            $toolsets = (Get-ChildItem $msvcRoot -Directory -ErrorAction SilentlyContinue).Name
            foreach ($ts in $toolsets) {
                $cl = Join-Path $msvcRoot "$ts\bin\Hostx64\x64\cl.exe"
                if (Test-Path $cl) {
                    # banner 行取第一条含 C/C++ 的输出行（stdout/stderr 合并顺序不稳定）
                    $banner = (& $cl 2>&1 | Where-Object { $_.ToString() -match 'C/C\+\+' } |
                        Select-Object -First 1 | ForEach-Object { $_.ToString() })
                    $clBanners += [ordered]@{ toolset = $ts; cl_path = $cl; banner = $banner.Trim() }
                }
            }
        }
        [ordered]@{
            name          = $_.displayName
            version       = $_.installationVersion
            product_line  = if ($_.catalog) { $_.catalog.productLineVersion } else { $null }
            path          = $instPath
            msvc_toolsets = $toolsets
            cl_instances  = $clBanners
        }
    }
}
$winSdks = Invoke-Probe 'sdk.versions' {
    $inc = 'C:\Program Files (x86)\Windows Kits\10\Include'
    if (Test-Path $inc) { (Get-ChildItem $inc -Directory -ErrorAction Stop).Name } else { @() }
}
$wdkInstalled = Invoke-Probe 'wdk.detect' {
    # WDK 判据：Windows Kits 下存在内核模式头（km/）或 WDK VSIX
    $km = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Include' -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName 'km') }
    $vsix = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\Vsix' -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match 'WDK' }
    [ordered]@{ installed = (($km.Count + $vsix.Count) -gt 0); km_headers = ($km.Count -gt 0); vsix = $vsix.Name }
}

# ---------- GStreamer ----------
# root 解析链：进程环境变量 → 注册表机器环境变量（长会话进程可能未继承安装器写入的新变量）
#               → 已知默认安装路径。三级都失败才判 not-installed。
function Resolve-GstRoot {
    param([string]$VarName, [string]$DefaultPath)
    $candidates = @()
    $procVal = [Environment]::GetEnvironmentVariable($VarName, 'Process')
    $machVal = [Environment]::GetEnvironmentVariable($VarName, 'Machine')
    if ($procVal) { $candidates += $procVal }
    if ($machVal) { $candidates += $machVal }
    if ($DefaultPath) { $candidates += $DefaultPath }
    foreach ($c in $candidates) {
        if ($c -and (Test-Path (Join-Path $c 'bin\gst-launch-1.0.exe'))) { return $c }
    }
    # 变量存在但 bin 缺失也如实返回（供排查），否则 null
    if ($procVal) { return $procVal }
    if ($machVal) { return $machVal }
    return $null
}
$gstRootMsvc = Invoke-Probe 'gst.root.msvc' {
    Resolve-GstRoot -VarName 'GSTREAMER_1_0_ROOT_MSVC_X86_64' -DefaultPath 'C:\gstreamer\1.0\msvc_x86_64'
}
# MinGW 版未安装是合法结果（null = 未检出），只在异常时记入 probes_failed
$gstRootMingw = try {
    Resolve-GstRoot -VarName 'GSTREAMER_1_0_ROOT_X86_64' -DefaultPath 'C:\gstreamer\1.0\x86_64'
} catch {
    $probesFailed.Add("gst.root.mingw ($($_.Exception.Message))") | Out-Null
    $null
}
$gstLaunch = $null
$gstVersion = $null
if ($gstRootMsvc -and (Test-Path (Join-Path $gstRootMsvc 'bin\gst-launch-1.0.exe'))) {
    $gstLaunch = Join-Path $gstRootMsvc 'bin\gst-launch-1.0.exe'
    $gstVersion = Invoke-Probe 'gst.version' {
        (& $gstLaunch --version 2>&1 | Select-Object -First 1 | ForEach-Object { $_.ToString() }).Trim()
    }
}
$gstSection = [ordered]@{
    installed           = [bool]$gstVersion
    variant             = if ($gstVersion) { 'msvc' } else { $null }
    version_output      = $gstVersion
    root                = $gstRootMsvc
    mingw_root_detected = if ($gstRootMingw) { $gstRootMingw } else { $null }
    gst_launch_path     = $gstLaunch
    root_source         = if ($gstVersion) {
        if ([Environment]::GetEnvironmentVariable('GSTREAMER_1_0_ROOT_MSVC_X86_64', 'Process')) { 'process-env' }
        elseif ([Environment]::GetEnvironmentVariable('GSTREAMER_1_0_ROOT_MSVC_X86_64', 'Machine')) { 'machine-env-registry' }
        else { 'default-path-fallback' }
    } else { $null }
}

# ---------- 物理设备与隐私权限 ----------
$cameras = Invoke-Probe 'device.camera' {
    Get-PnpDevice -Class Camera -ErrorAction Stop |
        ForEach-Object { [ordered]@{ name = $_.FriendlyName; status = $_.Status } }
}
$audioDevices = Invoke-Probe 'device.audio' {
    Get-CimInstance Win32_SoundDevice -ErrorAction Stop |
        ForEach-Object { [ordered]@{ name = $_.Name; status = $_.Status } }
}
$consent = Invoke-Probe 'privacy.consent' {
    $root = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore'
    [ordered]@{
        camera     = if (Test-Path "$root\camera") { (Get-ItemProperty "$root\camera").Value } else { 'not-found' }
        microphone = if (Test-Path "$root\microphone") { (Get-ItemProperty "$root\microphone").Value } else { 'not-found' }
    }
}

# ---------- 安全软件 / VPN / 管控线索（R-11） ----------
$avProducts = Invoke-Probe 'security.av' {
    Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct -ErrorAction Stop |
        ForEach-Object { [ordered]@{ name = $_.displayName; product_state = $_.productState } }
}
$fwProfiles = Invoke-Probe 'security.firewall' {
    Get-NetFirewallProfile -ErrorAction Stop |
        ForEach-Object { [ordered]@{ profile = $_.Name; enabled = $_.Enabled } }
}
$vpnAdapters = Invoke-Probe 'security.vpn' {
    Get-NetAdapter -ErrorAction Stop |
        Where-Object { $_.Status -eq 'Up' } |
        ForEach-Object { [ordered]@{ name = $_.Name; description = $_.InterfaceDescription; status = $_.Status } }
}

# ---------- 账户（§5.1-7 多用户可建性线索） ----------
$localUsers = Invoke-Probe 'account.users' {
    (Get-LocalUser -ErrorAction Stop | Where-Object { $_.Enabled } | Select-Object -ExpandProperty Name)
}
$isAdmin = Invoke-Probe 'account.is_admin' {
    ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole('Administrators')
}

# ---------- 无法从本机探测的事实（显式 unconfirmed，不编造） ----------
$unconfirmed = @(
    [ordered]@{
        key   = 'second_isolated_driver_test_machine'
        value = $null
        status = 'unconfirmed'
        note  = '本机无法探测其他物理设备是否存在；需用户答复。影响 M2-T6（Track B）测试机策略（M0-T5 输出）。'
    }
)

# ---------- 汇总 ----------
$env = [ordered]@{
    schema_version = '1.0'
    tool           = [ordered]@{ name = 'verify-environment'; version = '1.0'; engine = "PowerShell $psVersion" }
    run_id         = $RunId
    generated_at   = (Get-Date -Format 'yyyy-MM-ddTHH:mm:sszzz')   # volatile
    machine        = [ordered]@{
        hostname = $env:COMPUTERNAME
        os       = $osSection
        cpu      = [ordered]@{
            name    = if ($cpu) { $cpu.Name.Trim() } else { $null }
            cores   = if ($cpu) { [int]$cpu.NumberOfCores } else { $null }
            threads = if ($cpu) { [int]$cpu.NumberOfLogicalProcessors } else { $null }
        }
        gpu      = @($gpus)
        ram_gb   = [ordered]@{
            total    = if ($cs) { [math]::Round($cs.TotalPhysicalMemory / 1GB, 1) } else { $null }
            free     = if ($osFreeKB) { [math]::Round($osFreeKB / 1MB, 1) } else { $null }
            volatile = $true
        }
        disk     = @($disks)
    }
    toolchain      = [ordered]@{
        git    = $gitV
        python = $pythonV
        cmake  = $cmakeV
        rust   = [ordered]@{
            rustc            = $rustcV
            cargo            = $cargoV
            rustup_toolchain = $rustupV
            default_host     = $rustHost
        }
        visual_studio = @{
            instances = @($vsInstances)
        }
        windows_sdks  = @($winSdks)
        wdk           = $wdkInstalled
        gstreamer     = $gstSection
    }
    devices        = [ordered]@{
        cameras      = @($cameras)
        audio        = @($audioDevices)
        privacy_hint = $consent
    }
    security_software = [ordered]@{
        av_products = @($avProducts)
        firewall    = @($fwProfiles)
        adapters_up = @($vpnAdapters)
        note        = 'R-11 线索登记（AV/防火墙/活动网卡）；企业管控（VPN 客户端、组策略）需人工复核。'
    }
    accounts       = [ordered]@{
        local_users_enabled = @($localUsers)
        current_shell_is_admin = $isAdmin
    }
    unconfirmed_facts = @($unconfirmed)
    probes_failed    = @($probesFailed)
}

# ---------- 写出 ----------
$outDir = Split-Path -Parent $OutFile
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}
$json = $env | ConvertTo-Json -Depth 12
Set-Content -Path $OutFile -Value $json -Encoding UTF8
Write-Host "environment.json written: $OutFile"
if ($probesFailed.Count -gt 0) {
    Write-Warning "probes failed: $($probesFailed -join '; ')"
    exit 2
}
exit 0
