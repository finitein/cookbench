param(
  [string]$Version = $env:COOKBENCH_VERSION,
  [switch]$AllowPrerelease,
  [switch]$DryRun,
  [string]$Manifest,
  [string]$BaseUrl
)

$ErrorActionPreference = "Stop"
$Repository = "finitein/cookbench"
if ($env:COOKBENCH_ALLOW_PRERELEASE -eq "1") { $AllowPrerelease = $true }
if ($env:COOKBENCH_DRY_RUN -eq "1") { $DryRun = $true }
if (-not [Environment]::Is64BitOperatingSystem) {
  throw "Cookbench is not yet available for Windows x86."
}

if (-not $BaseUrl) {
  if ($Version) {
    if (-not $Version.StartsWith("v")) { $Version = "v$Version" }
    $BaseUrl = "https://github.com/$Repository/releases/download/$Version"
  } else {
    $BaseUrl = "https://github.com/$Repository/releases/latest/download"
  }
}

$Temporary = Join-Path ([IO.Path]::GetTempPath()) ("cookbench-install-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $Temporary | Out-Null
try {
  $ManifestPath = Join-Path $Temporary "release-manifest.json"
  if ($Manifest) {
    if ($Manifest -match '^https?://') { Invoke-WebRequest -UseBasicParsing $Manifest -OutFile $ManifestPath }
    else { Copy-Item $Manifest $ManifestPath }
  } else {
    Invoke-WebRequest -UseBasicParsing "$BaseUrl/release-manifest.json" -OutFile $ManifestPath
  }
  $Release = Get-Content -Raw $ManifestPath | ConvertFrom-Json
  if ($Release.product -ne "Cookbench") { throw "Release manifest product mismatch." }
  if ($Release.channel -ne "stable" -and -not $AllowPrerelease) {
    throw "This Cookbench release is a prerelease and requires -AllowPrerelease."
  }
  $Artifact = @($Release.artifacts) | Where-Object { $_.name -match '-windows-x64\.msi$' } | Select-Object -First 1
  if (-not $Artifact -or $Artifact.sha256 -notmatch '^[0-9a-fA-F]{64}$') {
    throw "Release manifest has no valid Windows x64 artifact."
  }
  Write-Output "Cookbench artifact: $($Artifact.name)"
  Write-Output "SHA-256: $($Artifact.sha256)"
  if ($DryRun) { Write-Output "Dry-run: no download or installation was performed."; return }

  $Package = Join-Path $Temporary $Artifact.name
  Invoke-WebRequest -UseBasicParsing "$BaseUrl/$($Artifact.name)" -OutFile $Package
  $Actual = (Get-FileHash -Algorithm SHA256 $Package).Hash
  if ($Actual -ne $Artifact.sha256.ToUpperInvariant()) { throw "SHA-256 verification failed." }
  $Process = Start-Process msiexec.exe -ArgumentList @('/i', $Package) -Wait -PassThru
  if ($Process.ExitCode -ne 0) { throw "msiexec failed with exit code $($Process.ExitCode)." }
  Write-Output "Cookbench installation completed."
} finally {
  Remove-Item -Recurse -Force $Temporary -ErrorAction SilentlyContinue
}
