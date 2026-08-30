param(
  [Parameter(Mandatory = $true)]
  [string]$BundleRoot
)

$ErrorActionPreference = "Stop"

if ($env:RELEASE_CHANNEL -ne "stable") {
  Write-Output "Skipping Windows signing for prerelease artifacts."
  exit 0
}

foreach ($required in @(
  "WINDOWS_SIGNING_CERTIFICATE",
  "WINDOWS_SIGNING_CERTIFICATE_PASSWORD",
  "WINDOWS_SIGNING_TIMESTAMP_URL"
)) {
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($required))) {
    throw "Stable Windows release requires $required"
  }
}

$certificatePath = Join-Path $env:RUNNER_TEMP "cookbench-signing.pfx"
[IO.File]::WriteAllBytes(
  $certificatePath,
  [Convert]::FromBase64String($env:WINDOWS_SIGNING_CERTIFICATE)
)

try {
  $signtool = (Get-Command signtool.exe -ErrorAction Stop).Source
  $msiFiles = Get-ChildItem -Path $BundleRoot -Filter "*.msi" -File -Recurse
  if ($msiFiles.Count -eq 0) {
    throw "Stable Windows release is missing an MSI installer."
  }

  foreach ($msi in $msiFiles) {
    & $signtool sign /fd SHA256 /f $certificatePath /p $env:WINDOWS_SIGNING_CERTIFICATE_PASSWORD /tr $env:WINDOWS_SIGNING_TIMESTAMP_URL /td SHA256 $msi.FullName
    if ($LASTEXITCODE -ne 0) { throw "signtool failed for $($msi.FullName)" }

    $signature = Get-AuthenticodeSignature -FilePath $msi.FullName
    if ($signature.Status -ne "Valid") {
      throw "Signature validation failed for $($msi.FullName): $($signature.Status)"
    }
  }
}
finally {
  Remove-Item -Force -ErrorAction SilentlyContinue $certificatePath
}
