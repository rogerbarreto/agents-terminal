# Setup and Build Script for Pixel Terminal
# Run this script to set up the development environment

param(
    [switch]$InstallRust,
    [switch]$Build,
    [switch]$Run,
    [switch]$Release
)

$ErrorActionPreference = "Stop"

function Install-Rust {
    Write-Host "Installing Rust..." -ForegroundColor Cyan
    
    # Download rustup-init
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupPath = "$env:TEMP\rustup-init.exe"
    
    Write-Host "Downloading rustup-init.exe..."
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath
    
    Write-Host "Running rustup-init.exe..."
    & $rustupPath -y
    
    # Refresh PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
    
    Write-Host "Rust installed successfully!" -ForegroundColor Green
}

function Get-CargoPath {
    $cargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
    if (Test-Path $cargoPath) {
        return $cargoPath
    }
    
    # Try PATH
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cargo) {
        return $cargo.Source
    }
    
    return $null
}

function Build-Project {
    param([bool]$IsRelease = $false)
    
    $cargo = Get-CargoPath
    if (-not $cargo) {
        Write-Host "Cargo not found. Run with -InstallRust to install Rust." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "Building Pixel Terminal..." -ForegroundColor Cyan
    
    Push-Location $PSScriptRoot
    try {
        if ($IsRelease) {
            & $cargo build --release
        } else {
            & $cargo build
        }
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Build successful!" -ForegroundColor Green
        } else {
            Write-Host "Build failed!" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

function Run-Project {
    param([bool]$IsRelease = $false)
    
    $cargo = Get-CargoPath
    if (-not $cargo) {
        Write-Host "Cargo not found. Run with -InstallRust to install Rust." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "Running Pixel Terminal..." -ForegroundColor Cyan
    
    Push-Location $PSScriptRoot
    try {
        if ($IsRelease) {
            & $cargo run --release
        } else {
            & $cargo run
        }
    } finally {
        Pop-Location
    }
}

# Main logic
if ($InstallRust) {
    Install-Rust
}

if ($Build) {
    Build-Project -IsRelease $Release
}

if ($Run) {
    Run-Project -IsRelease $Release
}

if (-not $InstallRust -and -not $Build -and -not $Run) {
    Write-Host @"
Pixel Terminal Build Script

Usage:
    .\build.ps1 -InstallRust    Install Rust toolchain
    .\build.ps1 -Build          Build the project (debug)
    .\build.ps1 -Build -Release Build the project (release)
    .\build.ps1 -Run            Run the project (debug)
    .\build.ps1 -Run -Release   Run the project (release)

Examples:
    .\build.ps1 -InstallRust -Build -Run
    .\build.ps1 -Build -Release
"@ -ForegroundColor Yellow
}
