param(
    [int]$ChromeDriverPort = 9515,
    [switch]$SkipChromeDriver
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

Set-Location $repoRoot

if (-not $SkipChromeDriver) {
    try {
        Invoke-RestMethod "http://localhost:$ChromeDriverPort/status" | Out-Null
        Write-Host "ChromeDriver already reachable on port $ChromeDriverPort"
    } catch {
        Write-Host "Starting ChromeDriver on port $ChromeDriverPort"
        Start-Process -FilePath "chromedriver" -ArgumentList @("--port=$ChromeDriverPort") -WindowStyle Hidden
        Start-Sleep -Seconds 2
        Invoke-RestMethod "http://localhost:$ChromeDriverPort/status" | Out-Null
    }
}

cargo run
