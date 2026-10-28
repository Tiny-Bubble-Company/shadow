<#
.SYNOPSIS
  Registers Shadow to run in the background on Windows.

.DESCRIPTION
  Shadow is a per-user proxy, so it is registered as a Scheduled Task that starts at
  logon and restarts on failure (no admin rights needed). Pass -Uninstall to remove it.
  To run it as a real Windows service for all users instead, wrap `shadow.exe run`
  with a service host such as NSSM or WinSW.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File install-service.ps1 -Exe "$env:LOCALAPPDATA\Shadow\shadow.exe"
#>
param(
  [string]$Exe = "$env:LOCALAPPDATA\Shadow\shadow.exe",
  [string]$Listen = "127.0.0.1:8877",
  [string]$DataDir = "$env:APPDATA\shadow",
  [switch]$TrustCa,
  [switch]$Uninstall
)
$ErrorActionPreference = "Stop"
$TaskName = "Shadow"

if ($Uninstall) {
  Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
  certutil -user -delstore Root "Shadow Local CA" | Out-Null
  [Environment]::SetEnvironmentVariable("HTTPS_PROXY", $null, "User")
  [Environment]::SetEnvironmentVariable("NODE_EXTRA_CA_CERTS", $null, "User")
  Write-Host "Shadow removed."
  exit 0
}

if (-not (Test-Path $Exe)) { throw "shadow.exe not found at $Exe" }
& $Exe init | Out-Host

$action   = New-ScheduledTaskAction -Execute $Exe -Argument "run --listen $Listen"
$trigger  = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
            -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero) -Hidden
Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Settings $settings `
  -Description "Shadow - endpoint privacy layer for AI clouds" -Force | Out-Null
[Environment]::SetEnvironmentVariable("SHADOW_HOME", $DataDir, "User")
Start-ScheduledTask -TaskName $TaskName

if ($TrustCa) {
  certutil -user -addstore Root "$DataDir\ca\ca.pem" | Out-Host
  [Environment]::SetEnvironmentVariable("HTTPS_PROXY", "http://$Listen", "User")
  [Environment]::SetEnvironmentVariable("NO_PROXY", "localhost,127.0.0.1", "User")
  [Environment]::SetEnvironmentVariable("NODE_EXTRA_CA_CERTS", "$DataDir\ca\ca.pem", "User")
  [Environment]::SetEnvironmentVariable("REQUESTS_CA_BUNDLE", "$DataDir\ca\bundle.pem", "User")
  [Environment]::SetEnvironmentVariable("SSL_CERT_FILE", "$DataDir\ca\bundle.pem", "User")
}
Write-Host "Shadow is running on http://$Listen (task '$TaskName')."
