# capture-screen.ps1 — 全虚拟屏截图（证据用）
# powershell.exe -NoProfile -ExecutionPolicy Bypass -File capture-screen.ps1 -OutFile <png>
param([Parameter(Mandatory = $true)][string]$OutFile)

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bmp = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bmp.Size)
$g.Dispose()
$bmp.Save($OutFile, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output ("saved {0} ({1}x{2})" -f $OutFile, $bounds.Width, $bounds.Height)
