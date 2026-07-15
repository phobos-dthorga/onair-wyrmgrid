param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,

    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory
)

$ErrorActionPreference = 'Stop'
$installer = (Resolve-Path -LiteralPath $InstallerPath).Path
$destination = [System.IO.Path]::GetFullPath($InstallDirectory)

if (Test-Path -LiteralPath $destination) {
    throw "NSIS smoke-test destination already exists: $destination"
}

$process = Start-Process `
    -FilePath $installer `
    -ArgumentList @('/S', "/D=$destination") `
    -Wait `
    -PassThru

if ($process.ExitCode -ne 0) {
    throw "NSIS installer exited with code $($process.ExitCode)."
}

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

Write-Host "Verified NSIS installation at $destination"
Write-Host "Application: $($application[0].Name)"
Write-Host "Provider:    $($provider[0].Name)"
