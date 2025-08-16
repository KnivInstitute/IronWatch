@echo off
echo Building IronWatch Installer...
echo ================================

REM Check if the executable exists
if not exist "..\target\release\IronWatch.exe" (
    echo Error: IronWatch.exe not found in target\release
    echo Please build the project first using: cargo build --release
    pause
    exit /b 1
)

REM Copy the executable to dist directory
echo Copying executable...
copy "..\target\release\IronWatch.exe" "dist\IronWatch.exe" /Y > nul

REM Also copy the icon
echo Copying assets...
if not exist "dist\assets" mkdir "dist\assets"
copy "..\assets\icon.png" "dist\assets\icon.png" /Y > nul

REM Build the installer
echo Building installer with NSIS...
"C:\Program Files (x86)\NSIS\makensis.exe" installer.nsi

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ================================
    echo Installer built successfully!
    echo Output: IronWatch-Setup.exe
    echo ================================
    echo.
) else (
    echo.
    echo ================================
    echo Error building installer!
    echo ================================
    echo.
)

pause
