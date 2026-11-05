# Build portable dell'exe. Uso: .\build.ps1
$ErrorActionPreference = "Stop"
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $here

$vs = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
if (-not (Test-Path $vs)) {
    $vs = "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
}

if (Get-Process ACServerControl,ACLauncher -ErrorAction SilentlyContinue) {
    throw "ACServerControl.exe/ACLauncher.exe è in esecuzione: chiudilo prima di compilare."
}

# I path assoluti dei sorgenti (registry cargo) finiscono nei binari:
# li riscriviamo per non esporre il nome utente di chi compila.
$remap = "--remap-path-prefix=$env:USERPROFILE=/build"
if ($env:RUSTFLAGS -notlike "*$remap*") {
    $env:RUSTFLAGS = if ($env:RUSTFLAGS) { "$env:RUSTFLAGS $remap" } else { $remap }
}

$root = Split-Path -Parent $here
$out = Join-Path $root "dist"
$icons = Join-Path $here "src-tauri\icons\icon.ico"
$conf = Join-Path $here "src-tauri\tauri.conf.json"
$managerIco = Join-Path $root "docs\brand\logo-manager.ico"
$launcherIco = Join-Path $root "docs\brand\logo-launcher.ico"

function Invoke-Build {
    (Get-Item $conf).LastWriteTime = Get-Date
    if (Test-Path $vs) {
        cmd /c "call `"$vs`" -arch=x64 -host_arch=x64 >nul && npm run tauri build -- --no-bundle" | Out-Host
    } else {
        npm run tauri build -- --no-bundle | Out-Host
    }
    if ($LASTEXITCODE -ne 0) {
        throw "Build fallita (codice $LASTEXITCODE)."
    }
    $exe = Join-Path $here "src-tauri\target\release\ac-server-control.exe"
    if (-not (Test-Path $exe)) {
        $exe = Join-Path $here "src-tauri\target\release\AC Server Control.exe"
    }
    if (-not (Test-Path $exe)) {
        throw "Exe non prodotto in src-tauri\target\release"
    }
    return $exe
}

New-Item -ItemType Directory -Force -Path $out | Out-Null

Write-Host "Build 1/2 - manager (icona arancio)"
Copy-Item $managerIco $icons -Force
$managerExe = Invoke-Build
Copy-Item $managerExe (Join-Path $out "ACServerControl.exe") -Force

Write-Host "Build 2/2 - launcher (icona ciano)"
Copy-Item $launcherIco $icons -Force
$launcherExe = Invoke-Build
Copy-Item $launcherExe (Join-Path $out "ACLauncher.exe") -Force

Copy-Item $managerIco $icons -Force
(Get-Item $conf).LastWriteTime = Get-Date

$size = [math]::Round((Get-Item (Join-Path $out "ACServerControl.exe")).Length / 1MB, 1)
Write-Host "OK -> dist\ACServerControl.exe + dist\ACLauncher.exe ($size MB)"
