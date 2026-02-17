@echo off
setlocal enabledelayedexpansion

set REPO=rand0mdevel0per/ldrive
set INSTALL_DIR=%USERPROFILE%\.ldrive
set BIN_DIR=%USERPROFILE%\.local\bin

echo Installing LDrive...

mkdir "%INSTALL_DIR%" 2>nul
mkdir "%BIN_DIR%" 2>nul

echo Downloading latest release...
curl -s https://api.github.com/repos/%REPO%/releases/latest > release.json
for /f "tokens=*" %%a in ('findstr "browser_download_url.*ldrive-node-windows-x86_64.exe" release.json') do set DOWNLOAD_LINE=%%a
for /f "tokens=2 delims=:, " %%a in ("%DOWNLOAD_LINE%") do set DOWNLOAD_URL=%%~a
del release.json

curl -L %DOWNLOAD_URL% -o "%INSTALL_DIR%\ldrive-node.exe"

echo.
echo LDrive installed to %INSTALL_DIR%\ldrive-node.exe
echo.
echo Add %INSTALL_DIR% to your PATH to use 'ldrive-node' command
echo.
echo Next steps:
echo 1. Get your token from https://ldrive-web.pages.dev
echo 2. Run: ldrive-node.exe storage --token YOUR_TOKEN --storage-path %INSTALL_DIR%\data --quota 50GB
