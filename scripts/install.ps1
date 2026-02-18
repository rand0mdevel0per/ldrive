$ErrorActionPreference = "Stop"

$REPO = "rand0mdevel0per/ldrive"
$INSTALL_DIR = "$env:USERPROFILE\.ldrive"
$BIN_DIR = "$env:USERPROFILE\.local\bin"

Write-Host "🚀 Installing LDrive..." -ForegroundColor Cyan

# Create directories
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null

# Get latest release
Write-Host "📦 Downloading latest release..." -ForegroundColor Yellow
$release = Invoke-RestMethod "https://api.github.com/repos/$REPO/releases/latest"
$asset = $release.assets | Where-Object { $_.name -like "*ldrive-node-windows-x86_64.exe" }

if (-not $asset) {
    Write-Host "❌ No Windows release found" -ForegroundColor Red
    exit 1
}

# Download
$exePath = "$INSTALL_DIR\ldrive-node.exe"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $exePath

Write-Host ""
Write-Host "✅ LDrive installed to $exePath" -ForegroundColor Green
Write-Host ""
Write-Host "📝 Next steps:" -ForegroundColor Cyan
Write-Host "1. Get your token from https://ldrive-web.pages.dev"
Write-Host "2. Run: ldrive-node.exe storage --token YOUR_TOKEN --storage-path $INSTALL_DIR\data --quota 50GB"
Write-Host ""
Write-Host "💡 Add $INSTALL_DIR to your PATH to use 'ldrive-node' command"
