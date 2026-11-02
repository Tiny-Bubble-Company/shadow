# Shadow installer for Windows (PowerShell 5+).
#   irm https://hub.getshadow.dev/core/install.ps1 | iex
# Joining an organisation: set $env:SHADOW_ENROLL to the join link from your invite first.
# It then enrols this machine with the organisation's Shadow Hub and applies the managed
# setup (scheduled tasks, local CA trust, system proxy).
$ErrorActionPreference = "Stop"
$Repo   = if ($env:SHADOW_REPO) { $env:SHADOW_REPO } else { "https://github.com/Tiny-Bubble-Company/shadow" }
$Dest   = "$env:LOCALAPPDATA\Shadow"
$Arch   = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { throw "32-bit Windows is not supported" }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null

$zip = Join-Path $env:TEMP "shadow.zip"
try {
  Invoke-WebRequest "$Repo/releases/latest/download/shadow-$Arch-pc-windows-msvc.zip" -OutFile $zip -UseBasicParsing
  Expand-Archive $zip -DestinationPath $Dest -Force
} catch {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "No release found and no Rust toolchain (https://rustup.rs)." }
  Write-Host "Building from source with cargo..."
  cargo install --locked --git $Repo shadow-cli --root "$Dest\build"
  Copy-Item "$Dest\build\bin\shadow.exe" "$Dest\shadow.exe" -Force
}

# The system-tray app: protection status and the activity dashboard. Set SHADOW_NO_TRAY=1
# to skip it. `shadow install --apply` starts it at logon when present.
if (-not $env:SHADOW_NO_TRAY -and -not (Test-Path "$Dest\shadow-tray.exe") -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Host "Building the tray app..."
  try {
    cargo install --locked --git $Repo shadow-tray --root "$Dest\build"
    Copy-Item "$Dest\build\bin\shadow-tray.exe" "$Dest\shadow-tray.exe" -Force
  } catch { Write-Warning "Could not build the tray app; Shadow works without it." }
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$Dest*") { [Environment]::SetEnvironmentVariable("Path", "$userPath;$Dest", "User") }
& "$Dest\shadow.exe" init

if ($env:SHADOW_ENROLL) {
  & "$Dest\shadow.exe" hub connect --yes $env:SHADOW_ENROLL
  & "$Dest\shadow.exe" install --apply --system-proxy
  Write-Host ""
  Write-Host "Done. This machine is protected by Shadow and reports to your organisation."
  return
}
Write-Host ""
Write-Host "Installed to $Dest\shadow.exe. Next:"
Write-Host "  shadow install          # dry run"
Write-Host "  shadow install --apply --system-proxy  # scheduled tasks (proxy and tray), trust the local CA (current user),"
Write-Host "                                        # and route browsers and desktop apps (ChatGPT, Claude) through Shadow"
