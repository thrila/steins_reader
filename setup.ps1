#Requires -RunAsAdministrator
<#
Deploys Steins Reader as a Windows Service.
Run this script from the root of the cloned repo (the folder
containing frontend\ and backend\), e.g.:
  git clone <repo-url> steins-reader
  cd steins-reader
  .\deploy-steins-reader.ps1   (from an elevated PowerShell prompt)
#>

$ErrorActionPreference = 'Stop'

$repo        = $PSScriptRoot
$installDir  = 'C:\Program Files\SteinsReader'
$dataDir     = 'C:\ProgramData\SteinsReader'
$serviceName = 'SteinsReader'

function Write-Step($msg) {
    Write-Host "`n==> $msg" -ForegroundColor Cyan
}

if (-not (Test-Path "$repo\frontend") -or -not (Test-Path "$repo\backend")) {
    Write-Error "Expected frontend\ and backend\ next to this script (repo root: $repo)."
    exit 1
}

# 1. Stop and remove existing service if present (idempotency)
$existing = Get-CimInstance Win32_Service -Filter "Name='$serviceName'" -ErrorAction SilentlyContinue
if ($existing) {
    Write-Step "Existing service '$serviceName' found. Stopping and removing it."
    if ($existing.ProcessId -gt 0) {
        Stop-Process -Id $existing.ProcessId -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
    sc.exe stop $serviceName | Out-Null
    sc.exe delete $serviceName | Out-Null
    for ($i = 0; $i -lt 30; $i++) {
        if (-not (Get-CimInstance Win32_Service -Filter "Name='$serviceName'" -ErrorAction SilentlyContinue)) {
            break
        }
        Start-Sleep -Seconds 1
    }
    if (Get-CimInstance Win32_Service -Filter "Name='$serviceName'" -ErrorAction SilentlyContinue) {
        throw "Service '$serviceName' still exists after delete request."
    }
}

# 2. Build frontend
Write-Step "Building frontend"
Push-Location "$repo\frontend"
if (Test-Path "package-lock.json") {
    npm ci
} else {
    npm install
}
npm run build
Pop-Location

Remove-Item "$repo\backend\dist" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item "$repo\frontend\dist" "$repo\backend\dist" -Recurse -Force

# 3. Build backend release binary
Write-Step "Building backend (release)"
Push-Location "$repo\backend"
cargo build --release
Pop-Location

# 4. Stage install and data directories
Write-Step "Staging files"
New-Item -ItemType Directory -Force $installDir, $dataDir, "$dataDir\logs" | Out-Null
Copy-Item "$repo\backend\target\release\steins-reader.exe" "$installDir\steins-reader.exe" -Force
Remove-Item "$installDir\dist" -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item "$repo\backend\dist" "$installDir\dist" -Recurse -Force

# 5. Write service config
Write-Step "Writing config.json"
# NOTE: Set-Content -Encoding utf8 writes a UTF-8 BOM on Windows PowerShell 5.1.
# serde_json chokes on that BOM ("expected value at line 1 column 1"), so we
# write the file manually with an explicit no-BOM UTF8 encoding instead.
$configJson = @'
{
  "port": 1337,
  "db_path": "C:\\ProgramData\\SteinsReader\\steins_reader.db",
  "frontend_dir": "C:\\Program Files\\SteinsReader\\dist",
  "log_dir": "C:\\ProgramData\\SteinsReader\\logs"
}
'@
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText("$dataDir\config.json", $configJson, $utf8NoBom)

# 6. Register service and enable crash recovery
Write-Step "Registering Windows service"
# Use New-Service (a cmdlet) rather than sc.exe create (an external program):
# sc.exe re-parses its command line as raw text, and PowerShell's own re-quoting
# of arguments containing embedded quotes reliably mangles that parse. New-Service
# takes the binary path as a parameter value directly, with no re-parsing step.
$binPath = "`"$installDir\steins-reader.exe`" --config `"$dataDir\config.json`""
New-Service -Name $serviceName -BinaryPathName $binPath -StartupType Automatic -DisplayName $serviceName | Out-Null

# sc.exe is safe to use here: none of these arguments contain spaces or quotes,
# so there's nothing for PowerShell's re-quoting to mangle.
sc.exe failure $serviceName reset= 86400 actions= restart/5000/restart/5000/restart/5000
sc.exe failureflag $serviceName 1

Write-Step "Starting service"
sc.exe start $serviceName

Start-Sleep -Seconds 2
$service = Get-Service -Name $serviceName -ErrorAction Stop
if ($service.Status -ne 'Running') {
    throw "Service '$serviceName' did not reach Running state. Current status: $($service.Status)"
}

Write-Step "Done. Checking status"
Get-Service -Name $serviceName | Format-List *
