Param(
  [string]$Distro = "Ubuntu-22.04",
  [switch]$SkipFrontend,
  [switch]$SkipBuild,
  [switch]$PackageOnly,
  [string]$Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-RepoRoot {
  $root = Split-Path -Parent $PSScriptRoot
  if (-not $root) {
    $root = $PSScriptRoot
  }
  return $root
}

function Resolve-WslRepoRoot {
  param(
    [string]$WindowsPath,
    [string]$DefaultDistro
  )
  $cleanPath = $WindowsPath -replace '^Microsoft\.PowerShell\.Core\\FileSystem::', ''
  $cleanPath = $cleanPath.Trim()
  $normalized = $cleanPath -replace '/', '\'
  $patterns = @(
    '^(?:\\\\|\\)?wsl\.localhost\\(?<distro>[^\\]+)\\(?<path>.*)$',
    '^(?:\\\\|\\)?wsl\$\\(?<distro>[^\\]+)\\(?<path>.*)$'
  )
  foreach ($pattern in $patterns) {
    $match = [regex]::Match($normalized, $pattern)
    if ($match.Success) {
      $distro = $match.Groups['distro'].Value
      $path = $match.Groups['path'].Value
      $wslPath = "/$($path -replace '\\\\','/')"
      return @{ Path = $wslPath; Distro = $distro }
    }
  }
  if ($normalized -match '^[A-Za-z]:\\wsl\.localhost\\(?<distro>[^\\]+)\\(?<path>.*)$') {
    $distro = $matches['distro']
    $path = $matches['path']
    $wslPath = "/$($path -replace '\\\\','/')"
    return @{ Path = $wslPath; Distro = $distro }
  }
  $wslPath = (wsl.exe -d $DefaultDistro -- wslpath -u "$cleanPath").Trim()
  if (-not $wslPath) {
    throw "Failed to convert $cleanPath to WSL path."
  }
  return @{ Path = $wslPath; Distro = $DefaultDistro }
}

function Get-VersionFromCargo {
  param([string]$CargoTomlPath)
  $match = Select-String -Path $CargoTomlPath -Pattern '^\s*version\s*=\s*"([^"]+)"' | Select-Object -First 1
  if (-not $match) {
    throw "Could not determine version from $CargoTomlPath"
  }
  return $match.Matches[0].Groups[1].Value
}

function Get-WixExe {
  $cmd = Get-Command wix.exe -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  $candidate = "C:\Program Files\WiX Toolset v6.0\bin\wix.exe"
  if (Test-Path $candidate) { return $candidate }
  $userTool = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
  if (Test-Path $userTool) { return $userTool }
  return $null
}

function Ensure-Command {
  param([string]$Name)
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Name"
  }
}

if ($PackageOnly) {
  $SkipFrontend = $true
  $SkipBuild = $true
}

$RepoRoot = Get-RepoRoot
$BackendDir = Join-Path $RepoRoot "backend"
$FrontendDir = Join-Path $RepoRoot "frontend"
$DistDir = Join-Path $RepoRoot "dist"
$WixWxs = Join-Path $RepoRoot "installer\windows\librarian.wxs"
$InnoIss = Join-Path $RepoRoot "installer\windows\librarian.iss"
$CargoToml = Join-Path $BackendDir "Cargo.toml"

if (-not $Version) {
  $Version = Get-VersionFromCargo -CargoTomlPath $CargoToml
}

Write-Host "Building Librarian Windows distro version: $Version"

if (-not $SkipFrontend) {
  Ensure-Command wsl.exe
  $resolved = Resolve-WslRepoRoot -WindowsPath $RepoRoot -DefaultDistro $Distro
  $RepoRootUnix = $resolved.Path
  $Distro = $resolved.Distro
  Write-Host "Building frontend in WSL..."
  wsl.exe -d $Distro -- bash -lc "cd '$RepoRootUnix/frontend' && pnpm install && pnpm run build"
}

if (-not $SkipBuild) {
  Ensure-Command cargo.exe
  Write-Host "Building Windows release binary..."
  $env:CARGO_INCREMENTAL = "0"
  Push-Location $BackendDir
  cargo build --release --features embed-frontend --target x86_64-pc-windows-msvc
  Pop-Location
}

New-Item -ItemType Directory -Force -Path (Join-Path $DistDir "windows\x86_64") | Out-Null
Copy-Item -Force (Join-Path $BackendDir "target\x86_64-pc-windows-msvc\release\librarian.exe") (Join-Path $DistDir "windows\x86_64\librarian.exe")

$temp = Join-Path $env:TEMP "librarian-dist"
Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $temp | Out-Null
Copy-Item -Recurse -Force (Join-Path $DistDir "windows\x86_64") (Join-Path $temp "windows\x86_64") | Out-Null
Copy-Item -Force $WixWxs (Join-Path $temp "librarian.wxs") | Out-Null
Copy-Item -Force $InnoIss (Join-Path $temp "librarian.iss") | Out-Null

Write-Host "Packaging Windows zip..."
Compress-Archive -Path (Join-Path $temp "windows\x86_64\librarian.exe") -DestinationPath (Join-Path $temp "librarian-$Version-windows-x86_64.zip") -Force

Write-Host "Building MSI via WiX..."
$wix = Get-WixExe
Push-Location $temp
if ($wix) {
  & $wix build -d Version="$Version" -d SourceDir="$temp\windows\x86_64" -o "$temp\librarian-$Version-windows-x86_64.msi" "$temp\librarian.wxs"
} elseif ((Get-Command candle.exe -ErrorAction SilentlyContinue) -and (Get-Command light.exe -ErrorAction SilentlyContinue)) {
  candle.exe -dVersion="$Version" -dSourceDir="$temp\windows\x86_64" "$temp\librarian.wxs" -out "librarian-x86_64.wixobj"
  light.exe "librarian-x86_64.wixobj" -out "$temp\librarian-$Version-windows-x86_64.msi"
} else {
  throw "WiX tools not found (wix.exe or candle.exe/light.exe)."
}
Pop-Location

Write-Host "Building EXE installer via Inno Setup..."
Ensure-Command iscc.exe
Push-Location $temp
iscc.exe /DMyAppVersion="$Version" /DSourceDir="$temp\windows\x86_64" /DMyAppArch="x64" "$temp\librarian.iss"
Pop-Location

Copy-Item -Force (Join-Path $temp "librarian-$Version-windows-x86_64.zip") $DistDir
Copy-Item -Force (Join-Path $temp "librarian-$Version-windows-x86_64.msi") $DistDir
Copy-Item -Force (Join-Path $temp "librarian-$Version-windows-x86_64-setup.exe") $DistDir

Write-Host "Windows distro build complete. Artifacts are in $DistDir"
