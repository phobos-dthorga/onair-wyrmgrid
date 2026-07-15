[CmdletBinding()]
param(
    [string]$CargoTargetDir = (Join-Path $env:LOCALAPPDATA 'WyrmGrid\cargo-target'),
    [string]$PerlPath = (Join-Path $env:SystemDrive 'Strawberry\perl\bin\perl.exe'),
    [switch]$ValidateOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Find-VisualStudioDevShell {
    $vsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'

    if (Test-Path -LiteralPath $vsWhere -PathType Leaf) {
        $installationPath = & $vsWhere `
            -latest `
            -products '*' `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath

        if ($installationPath) {
            $candidate = Join-Path $installationPath 'Common7\Tools\Launch-VsDevShell.ps1'
            if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                return $candidate
            }
        }
    }

    $knownCandidates = @(
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\18\Community\Common7\Tools\Launch-VsDevShell.ps1'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\18\Professional\Common7\Tools\Launch-VsDevShell.ps1'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\18\Enterprise\Common7\Tools\Launch-VsDevShell.ps1'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio\18\BuildTools\Common7\Tools\Launch-VsDevShell.ps1')
    )

    return $knownCandidates | Where-Object {
        Test-Path -LiteralPath $_ -PathType Leaf
    } | Select-Object -First 1
}

$devShell = Find-VisualStudioDevShell
if (-not $devShell) {
    throw 'Visual Studio with the Desktop development with C++ workload was not found.'
}

if (-not (Test-Path -LiteralPath $PerlPath -PathType Leaf)) {
    throw "Strawberry Perl was not found at '$PerlPath'. Install it or pass -PerlPath with the full path to perl.exe."
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw 'npm was not found on PATH. Install the supported Node.js and npm versions first.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$env:OPENSSL_SRC_PERL = (Resolve-Path -LiteralPath $PerlPath).Path
$env:CARGO_TARGET_DIR = $CargoTargetDir

Write-Host "Visual Studio shell: $devShell"
Write-Host "OpenSSL Perl:       $env:OPENSSL_SRC_PERL"
Write-Host "Cargo target:       $env:CARGO_TARGET_DIR"
Write-Host ''

& $devShell -Arch amd64 -HostArch amd64

if ($ValidateOnly) {
    Write-Host 'Windows development prerequisites validated successfully.'
    return
}

Push-Location -LiteralPath $repositoryRoot
try {
    & npm run dev
    if ($LASTEXITCODE -ne 0) {
        throw "WyrmGrid development run exited with code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
