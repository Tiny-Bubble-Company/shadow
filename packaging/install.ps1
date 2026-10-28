# Shadow installer for Windows (PowerShell 5+).
#   irm https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.ps1 | iex
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

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$Dest*") { [Environment]::SetEnvironmentVariable("Path", "$userPath;$Dest", "User") }
& "$Dest\shadow.exe" init
Write-Host ""
Write-Host "Installed to $Dest\shadow.exe. Next:"
Write-Host "  shadow install          # dry run"
Write-Host "  shadow install --apply  # scheduled task + trust the local CA (current user)"
