param(
  [Parameter(Mandatory = $true)]
  [string]$ExtensionId
)

$ErrorActionPreference = "Stop"

$HostDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$LauncherPath = Join-Path $HostDir "ytdlp-host.bat"
$ManifestPath = Join-Path $HostDir "com.catra.ytdlp.json"

if (-not (Test-Path $LauncherPath)) {
  throw "Launcher not found: $LauncherPath"
}

$manifest = @{
  name = "com.catra.ytdlp"
  description = "Catra yt-dlp native messaging host"
  path = $LauncherPath
  type = "stdio"
  allowed_origins = @("chrome-extension://$ExtensionId/")
}

$manifest | ConvertTo-Json -Depth 4 | Set-Content -Path $ManifestPath -Encoding UTF8

$regPath = "HKCU:\SOFTWARE\Google\Chrome\NativeMessagingHosts\com.catra.ytdlp"
New-Item -Path $regPath -Force | Out-Null
Set-ItemProperty -Path $regPath -Name "(default)" -Value $ManifestPath

Write-Host "Installed native messaging host."
Write-Host "Manifest: $ManifestPath"
Write-Host "Registry: $regPath"
Write-Host "Extension ID: $ExtensionId"
