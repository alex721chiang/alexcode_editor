<#
.SYNOPSIS
  Build AlexCode (Release), run windeployqt, and produce AlexCode-Windows-vX.Y.Z.zip.

.NOTES
  Qt's moc cannot generate files under a non-ASCII (e.g. CJK) path, so the build
  output defaults to an ASCII temp path. Requires cmake / ninja / a compiler on PATH,
  and -QtDir pointing at Qt (with bin\windeployqt.exe).
.EXAMPLE
  pwsh scripts/package.ps1 -QtDir C:\Qt\6.8.3\mingw_64
#>
param(
    [string]$QtDir    = "C:\Qt\6.8.3\mingw_64",
    [string]$BuildDir = (Join-Path $env:TEMP "AlexCode_pkg_build"),
    [string]$OutDir   = $null
)

# Note: do NOT set ErrorActionPreference=Stop globally — cmake writes warnings to
# stderr which PowerShell 5.1 would treat as terminating. Check $LASTEXITCODE instead.
$repo = Split-Path -Parent $PSScriptRoot                 # repo root (parent of scripts/)
if (-not $OutDir) { $OutDir = Join-Path $repo "dist" }

# Parse the version from CMakeLists.txt
$cml = Get-Content (Join-Path $repo "CMakeLists.txt") -Raw
if ($cml -notmatch 'project\(AlexCode\s+VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)') {
    throw "Version not found (project(AlexCode VERSION ...) in CMakeLists.txt)"
}
$version = $Matches[1]
Write-Host "AlexCode version: $version"

# Put Qt bin first on PATH (windeployqt + runtime DLLs)
$env:PATH = "$QtDir\bin;" + $env:PATH

Write-Host "Configure + build ($BuildDir) ..."
cmake -S $repo -B $BuildDir -G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_PREFIX_PATH=$QtDir
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed ($LASTEXITCODE)" }
cmake --build $BuildDir
if ($LASTEXITCODE -ne 0) { throw "Build failed ($LASTEXITCODE)" }

# Locate the built exe (Ninja: src\; VS multi-config: src\Release\)
$exe = Get-ChildItem $BuildDir -Recurse -Filter AlexCode.exe | Select-Object -First 1
if (-not $exe) { throw "AlexCode.exe not found after build" }

# Deploy into a clean staging folder
$stage = Join-Path $BuildDir "stage\AlexCode-Windows-v$version"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item $exe.FullName $stage
& "$QtDir\bin\windeployqt.exe" --release --compiler-runtime --no-translations (Join-Path $stage "AlexCode.exe")
Set-Content (Join-Path $stage "portable.ini") "; AlexCode portable marker - data stored in .\data" -Encoding ascii

# Zip it
New-Item -ItemType Directory -Force $OutDir | Out-Null
$zip = Join-Path $OutDir "AlexCode-Windows-v$version.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $stage -DestinationPath $zip
Write-Host "Done: $zip ($([math]::Round((Get-Item $zip).Length/1MB,1)) MB)"
