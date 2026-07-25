[CmdletBinding()]
param(
    [string]$CargoTargetDir,
    [string]$PerlPath = (Join-Path $env:SystemDrive 'Strawberry\perl\bin\perl.exe'),
    [switch]$ValidateOnly
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

. (Join-Path $PSScriptRoot 'windows-build-environment.ps1')

function Restore-DevelopmentDependenciesIfNeeded {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot
    )

    $tauriCommand = Join-Path $RepositoryRoot 'node_modules\.bin\tauri.cmd'
    if (Test-Path -LiteralPath $tauriCommand -PathType Leaf) {
        return
    }

    $lockFile = Join-Path $RepositoryRoot 'package-lock.json'
    if (-not (Test-Path -LiteralPath $lockFile -PathType Leaf)) {
        throw "The local Tauri command is missing and '$lockFile' was not found, so WyrmGrid cannot safely restore its locked development dependencies."
    }

    Write-Host 'Local Tauri dependencies are missing. Restoring them from package-lock.json...'
    & npm ci
    if ($LASTEXITCODE -ne 0) {
        throw "WyrmGrid dependency restoration exited with code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $tauriCommand -PathType Leaf)) {
        throw "npm ci completed, but the local Tauri command was not created at '$tauriCommand'."
    }

    Write-Host 'WyrmGrid development dependencies restored successfully.'
    Write-Host ''
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Enter-WyrmGridWindowsBuildEnvironment `
    -RepositoryRoot $repositoryRoot `
    -CargoTargetDir $CargoTargetDir `
    -PerlPath $PerlPath

if ($ValidateOnly) {
    Write-Host 'Windows development prerequisites validated successfully.'
    return
}

Push-Location -LiteralPath $repositoryRoot
try {
    Restore-DevelopmentDependenciesIfNeeded -RepositoryRoot $repositoryRoot

    & npm run dev
    if ($LASTEXITCODE -ne 0) {
        throw "WyrmGrid development run exited with code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}
