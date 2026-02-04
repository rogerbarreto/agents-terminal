# Run script for Pixel Terminal
# Sets up VS Build Tools environment and runs the application

param(
    [switch]$Release,
    [string]$LogLevel = "info"
)

# Find and source VS Build Tools environment
$vsDevCmd = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
if (-not (Test-Path $vsDevCmd)) {
    $vsDevCmd = "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
}
if (-not (Test-Path $vsDevCmd)) {
    # Try to find any VS installation
    $vsDevCmd = Get-ChildItem "C:\Program Files*\Microsoft Visual Studio" -Recurse -Filter "VsDevCmd.bat" -ErrorAction SilentlyContinue | 
        Select-Object -First 1 -ExpandProperty FullName
}

if ($vsDevCmd -and (Test-Path $vsDevCmd)) {
    Write-Host "Setting up VS environment from: $vsDevCmd" -ForegroundColor Cyan
    cmd /c "`"$vsDevCmd`" -arch=x64 && set" | ForEach-Object {
        if ($_ -match "^([^=]+)=(.*)$") {
            [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
} else {
    Write-Host "Warning: VS Build Tools not found. Build may fail." -ForegroundColor Yellow
}

# Add cargo to PATH
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path

# Set log level
$env:RUST_LOG = $LogLevel

# Change to script directory
Push-Location $PSScriptRoot

try {
    if ($Release) {
        Write-Host "Running Pixel Terminal (release)..." -ForegroundColor Green
        cargo run --release
    } else {
        Write-Host "Running Pixel Terminal (debug)..." -ForegroundColor Green
        cargo run
    }
} finally {
    Pop-Location
}
