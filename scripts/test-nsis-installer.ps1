param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,

    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory,

    [string]$PreviousInstallerPath = ''
)

$ErrorActionPreference = 'Stop'
$installer = (Resolve-Path -LiteralPath $InstallerPath).Path
$destination = [System.IO.Path]::GetFullPath($InstallDirectory)
$appDataDirectory = Join-Path $env:APPDATA 'io.github.phobosdthorga.onairwyrmgrid'
$sentinelPath = Join-Path $appDataDirectory 'ci-upgrade-sentinel.txt'

if (Test-Path -LiteralPath $destination) {
    throw "NSIS smoke-test destination already exists: $destination"
}

function Install-WyrmGridSetup {
    param([Parameter(Mandatory = $true)][string]$SetupPath)

    $process = Start-Process `
        -FilePath $SetupPath `
        -ArgumentList @('/S', "/D=$destination") `
        -Wait `
        -PassThru

    if ($process.ExitCode -ne 0) {
        throw "NSIS installer $SetupPath exited with code $($process.ExitCode)."
    }
}

function Get-InstalledComponents {
    if (-not (Test-Path -LiteralPath $destination -PathType Container)) {
        throw "NSIS installer did not create the requested installation directory."
    }

    $executables = @(Get-ChildItem -LiteralPath $destination -Filter '*.exe' -File -Recurse)
    $provider = @($executables | Where-Object { $_.Name -like 'wyrmgrid-simconnect-provider*.exe' })
    $application = @(
        $executables | Where-Object {
            $_.Name -notlike 'wyrmgrid-simconnect-provider*.exe' -and
            $_.Name -notmatch '^unins.*\.exe$'
        }
    )

    if ($application.Count -lt 1) {
        throw "NSIS installation does not contain the WyrmGrid application executable."
    }

    if ($provider.Count -ne 1) {
        throw "NSIS installation must contain exactly one SimConnect provider sidecar; found $($provider.Count)."
    }

    return @{
        Application = $application[0]
        Provider = $provider[0]
    }
}

$previousApplicationHash = $null
$sentinel = $null
if ($PreviousInstallerPath) {
    $previousInstaller = (Resolve-Path -LiteralPath $PreviousInstallerPath).Path
    Install-WyrmGridSetup -SetupPath $previousInstaller
    $previousComponents = Get-InstalledComponents
    $previousApplicationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $previousComponents.Application.FullName).Hash

    [System.IO.Directory]::CreateDirectory($appDataDirectory) | Out-Null
    $sentinel = [guid]::NewGuid().ToString('N')
    Set-Content -LiteralPath $sentinelPath -Value $sentinel -NoNewline
    Write-Host "Installed previous release: $($previousComponents.Application.Name)"
}

Install-WyrmGridSetup -SetupPath $installer
$components = Get-InstalledComponents

if ($PreviousInstallerPath) {
    $currentApplicationHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $components.Application.FullName).Hash
    if ($currentApplicationHash -eq $previousApplicationHash) {
        throw "The new setup did not replace the previous application executable."
    }
    if (-not (Test-Path -LiteralPath $sentinelPath -PathType Leaf)) {
        throw "The in-place setup upgrade removed the application-data directory."
    }
    if ((Get-Content -Raw -LiteralPath $sentinelPath) -ne $sentinel) {
        throw "The in-place setup upgrade altered existing application data."
    }
    Write-Host "Verified in-place NSIS upgrade and preserved application data."
}

Write-Host "Verified NSIS installation at $destination"
Write-Host "Application: $($components.Application.Name)"
Write-Host "Provider:    $($components.Provider.Name)"
