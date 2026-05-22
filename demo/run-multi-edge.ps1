# Launch two Aegis daemons for the multi-edge demo (us-east + eu-west).
# Run from the repository root in PowerShell.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
Set-Location $root

New-Item -ItemType Directory -Force -Path data | Out-Null

Write-Host "Building aegis-daemon..."
cargo build --bin aegis-daemon | Out-Null

$daemon = Join-Path $root "target\debug\aegis-daemon.exe"

Write-Host "Starting us-east on :5140 / :7321 ..."
Start-Process -FilePath $daemon -ArgumentList @(
    "--config", "configs\aegis.us-east.demo.toml"
) -WindowStyle Normal

Start-Sleep -Seconds 2

Write-Host "Starting eu-west on :5142 / :7322 ..."
Start-Process -FilePath $daemon -ArgumentList @(
    "--config", "configs\aegis.eu-west.demo.toml"
) -WindowStyle Normal

Write-Host ""
Write-Host "Both gateways are starting in separate windows."
Write-Host "Verify:"
Write-Host "  curl.exe http://127.0.0.1:7321/api/status"
Write-Host "  curl.exe http://127.0.0.1:7322/api/status"
Write-Host ""
Write-Host "Send traffic:"
Write-Host "  python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 50 --duration 10"
Write-Host "  python demo\log_spammer.py --target tcp://127.0.0.1:5142 --pattern routine --rate 200 --duration 10"
Write-Host ""
Write-Host "Stop: close both daemon windows or run:"
Write-Host "  Get-Process aegis-daemon | Stop-Process -Force"
