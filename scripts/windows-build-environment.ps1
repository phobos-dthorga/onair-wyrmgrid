Set-StrictMode -Version Latest

function Find-WyrmGridVisualStudioDevShell {
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

function Get-WyrmGridCargoTargetDirectory {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot
    )

    $worktreeName = Split-Path -Leaf $RepositoryRoot
    $cacheName = $worktreeName -replace '[^A-Za-z0-9._-]+', '-'
    if ([string]::IsNullOrWhiteSpace($cacheName)) {
        $cacheName = 'worktree'
    }

    $cacheRoot = Join-Path $env:LOCALAPPDATA 'WyrmGrid\cargo-target'
    return Join-Path $cacheRoot $cacheName
}

function Get-WyrmGridJenkinsCargoTargetDirectory {
    param(
        [Parameter(Mandatory)]
        [string]$JobName,
        [string]$CacheRoot
    )

    if ([string]::IsNullOrWhiteSpace($CacheRoot)) {
        $CacheRoot = Join-Path $env:LOCALAPPDATA 'WyrmGrid\jenkins-cargo-target'
    }

    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $jobBytes = [System.Text.Encoding]::UTF8.GetBytes($JobName)
        $jobHash = [System.BitConverter]::ToString(
            $sha256.ComputeHash($jobBytes)
        ).Replace('-', '').Substring(0, 16).ToLowerInvariant()
    }
    finally {
        $sha256.Dispose()
    }

    return Join-Path $CacheRoot $jobHash
}

function Enter-WyrmGridWindowsBuildEnvironment {
    param(
        [Parameter(Mandatory)]
        [string]$RepositoryRoot,
        [string]$CargoTargetDir,
        [string]$PerlPath = (Join-Path $env:SystemDrive 'Strawberry\perl\bin\perl.exe')
    )

    $devShell = Find-WyrmGridVisualStudioDevShell
    if (-not $devShell) {
        throw 'Visual Studio with the Desktop development with C++ workload was not found.'
    }

    if (-not (Test-Path -LiteralPath $PerlPath -PathType Leaf)) {
        throw "Strawberry Perl was not found at '$PerlPath'. Install it or pass -PerlPath with the full path to perl.exe."
    }

    foreach ($command in @('node', 'npm', 'rustc', 'cargo')) {
        if (-not (Get-Command $command -ErrorAction SilentlyContinue)) {
            throw "$command was not found on PATH. Install the supported Windows development toolchain first."
        }
    }

    if ([string]::IsNullOrWhiteSpace($CargoTargetDir)) {
        $CargoTargetDir = Get-WyrmGridCargoTargetDirectory -RepositoryRoot $RepositoryRoot
    }

    $env:OPENSSL_SRC_PERL = (Resolve-Path -LiteralPath $PerlPath).Path
    $env:CARGO_TARGET_DIR = $CargoTargetDir

    Write-Host "Visual Studio shell: $devShell"
    Write-Host "OpenSSL Perl:       $env:OPENSSL_SRC_PERL"
    Write-Host "Cargo target:       $env:CARGO_TARGET_DIR"
    Write-Host ''

    & $devShell -Arch amd64 -HostArch amd64
}
