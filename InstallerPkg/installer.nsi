; ===============================================================================
; IronWatch - Advanced USB Device Monitor
; Comprehensive NSIS Installer Script with Enhanced UI & Experience
; ===============================================================================

!include "MUI2.nsh"
!include "x64.nsh"
!include "WinMessages.nsh"
!include "FileFunc.nsh"

; ===============================================================================
; INSTALLER CONFIGURATION
; ===============================================================================

; Basic Information
Name "IronWatch - Advanced USB Device Monitor"
OutFile "IronWatch-Setup.exe"
Unicode True
SetCompressor /SOLID lzma
SetCompressorDictSize 64

; Installation directory - Program Files (x64)
!ifdef WIN64
  InstallDir "$PROGRAMFILES64\IronWatch"
!else
  InstallDir "$PROGRAMFILES\IronWatch"
!endif

; Get installation folder from registry if available
InstallDirRegKey HKLM "Software\IronWatch" "InstallPath"

; Request admin privileges
RequestExecutionLevel admin

; ===============================================================================
; VERSION INFORMATION
; ===============================================================================

VIProductVersion "1.0.0.0"
VIAddVersionKey "ProductName" "IronWatch"
VIAddVersionKey "Comments" "Advanced USB Device Monitor - Real-time Security & Performance Monitoring"
VIAddVersionKey "CompanyName" "KnivInstitute"
VIAddVersionKey "LegalCopyright" "© 2025 KnivInstitute"
VIAddVersionKey "FileDescription" "IronWatch Advanced Installer"
VIAddVersionKey "FileVersion" "1.0.0.0"
VIAddVersionKey "ProductVersion" "1.0.0.0"
VIAddVersionKey "InternalName" "IronWatch-Setup.exe"
VIAddVersionKey "LegalTrademarks" "IronWatch™"
VIAddVersionKey "OriginalFilename" "IronWatch-Setup.exe"

; ===============================================================================
; MODERN UI CONFIGURATION - ENHANCED VISUALS
; ===============================================================================

!define MUI_ABORTWARNING

; Custom Icons
; !define MUI_ICON "..\assets\icon.png"
; !define MUI_UNICON "..\assets\icon.png"

; Enhanced UI with custom styling

; Welcome & Finish Page Enhancements
!define MUI_WELCOMEPAGE_TITLE "Welcome to IronWatch Setup!"
!define MUI_WELCOMEPAGE_TEXT "This wizard will guide you through the installation of IronWatch, an advanced USB device monitoring tool.$\nClick Next to continue or Cancel to exit."

; Finish Page Customization
!define MUI_FINISHPAGE_TITLE "IronWatch Installation Complete!"
!define MUI_FINISHPAGE_TEXT "IronWatch has been successfully installed on your computer.$\r$\n$\r$\nKey Features Installed:$\r$\n• Real-time USB device monitoring dashboard$\r$\n• Advanced security event detection$\r$\n• Live device connection graphs and statistics$\r$\n• Multi-format export capabilities (JSON, CSV, Table)$\r$\n• Customizable monitoring preferences$\r$\n• System tray integration$\r$\n$\r$\nReady to monitor and secure your USB devices!"
!define MUI_FINISHPAGE_RUN "$INSTDIR\ironwatch.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch IronWatch now"
!define MUI_FINISHPAGE_SHOWREADME "$INSTDIR\README.txt"
!define MUI_FINISHPAGE_SHOWREADME_TEXT "View Quick Start Guide"
!define MUI_FINISHPAGE_LINK "Visit IronWatch Website"
!define MUI_FINISHPAGE_LINK_LOCATION "https://github.com/KnivInstitute/IronWatch"

; Custom License Page Text
!define MUI_LICENSEPAGE_TEXT_TOP "Please review the license terms below:"
!define MUI_LICENSEPAGE_TEXT_BOTTOM "If you accept the terms of the agreement, click I Agree to continue. You must accept the agreement to install IronWatch."

; Welcome page
!insertmacro MUI_PAGE_WELCOME

; License page
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"

; Components page
!insertmacro MUI_PAGE_COMPONENTS

; Directory page
!insertmacro MUI_PAGE_DIRECTORY

; Installation page
!insertmacro MUI_PAGE_INSTFILES

; Finish page
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages
!insertmacro MUI_LANGUAGE "English"

; ===============================================================================
; INSTALLATION SECTIONS - ENHANCED COMPONENTS
; ===============================================================================

Section "IronWatch Core" SecCore
  SectionIn RO  ; Required section
  
  ; Display status
  DetailPrint "Installing IronWatch core components..."
  
  ; Set output path to the installation directory
  SetOutPath $INSTDIR
  
  ; Install main executable with progress feedback
  DetailPrint "Installing main executable..."
  File /oname=ironwatch.exe "dist\IronWatch.exe"
  
  ; Create application data directories
  CreateDirectory "$INSTDIR\data"
  CreateDirectory "$INSTDIR\export"
  CreateDirectory "$INSTDIR\logs"
  CreateDirectory "$INSTDIR\config"
  
  ; Install assets with progress feedback
  DetailPrint "Installing application assets..."
  SetOutPath $INSTDIR\assets

  
  ; Install icon
  File "dist\assets\icon.png"
  
  ; Install license
  DetailPrint "Installing license documentation..."
  SetOutPath $INSTDIR
  IfFileExists "..\LICENSE" +5 0
    DetailPrint "Creating default license file..."
    FileOpen $1 "$INSTDIR\LICENSE.txt" w
    FileWrite $1 "MIT License$\r$\n$\r$\nCopyright (c) 2025 KnivInstitute$\r$\n$\r$\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the Software), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:$\r$\n$\r$\nThe above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.$\r$\n$\r$\nTHE SOFTWARE IS PROVIDED AS IS, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."
    FileClose $1
    Goto +2
  File /oname=LICENSE.txt "..\LICENSE"
  
  ; Create enhanced README with installation info
  DetailPrint "Generating Quick Start Guide..."
  FileOpen $0 "$INSTDIR\README.txt" w
  FileWrite $0 "IronWatch - Advanced USB Device Monitor$\r$\n"
  FileWrite $0 "==============================================$\r$\n$\r$\n"
  FileWrite $0 "INSTALLATION SUCCESSFUL!$\r$\n$\r$\n"
  FileWrite $0 "Thank you for installing IronWatch v1.0.0!$\r$\n$\r$\n"
  
  ; Get system info for README  
  FileWrite $0 "Installation Details:$\r$\n"
  FileWrite $0 "   • Install Path: $INSTDIR$\r$\n$\r$\n"
  
  FileWrite $0 "FEATURES$\r$\n"
  FileWrite $0 "========$\r$\n$\r$\n"
  
  FileWrite $0 "GUI Mode (Default):$\r$\n"
  FileWrite $0 "- Modern GUI Interface: Beautiful, responsive GUI built with egui$\r$\n"
  FileWrite $0 "- Real-time Device Monitoring: Live updates with smooth animations$\r$\n"
  FileWrite $0 "- Interactive Dashboard: Overview of connected devices and statistics$\r$\n"
  FileWrite $0 "- Device Table View: Detailed device information in tabular format$\r$\n"
  FileWrite $0 "- Filtering & Search: Real-time filtering of devices$\r$\n"
  FileWrite $0 "- Settings Panel: Configure monitoring preferences$\r$\n"
  FileWrite $0 "- Dark/Light Theme: Customizable appearance$\r$\n"
  FileWrite $0 "- System Tray Integration: Minimize to system tray$\r$\n$\r$\n"
  
  FileWrite $0 "CLI Mode (Advanced Users):$\r$\n"
  FileWrite $0 "- USB Device Monitoring: Real-time monitoring of USB device connections$\r$\n"
  FileWrite $0 "- Multiple Output Formats: Support for JSON, Table, and CSV output$\r$\n"
  FileWrite $0 "- Filtering: Filter devices by name patterns$\r$\n"
  FileWrite $0 "- Configuration Management: Persistent configuration with JSON settings$\r$\n"
  FileWrite $0 "- Logging: Comprehensive logging with configurable levels$\r$\n$\r$\n"
  
  FileWrite $0 "General Features:$\r$\n"
  FileWrite $0 "- High Performance: Efficient USB monitoring with minimal resource usage$\r$\n"
  FileWrite $0 "- Extensible Architecture: Modular design for easy feature additions$\r$\n"
  FileWrite $0 "- Security Monitoring: Detect suspicious USB device activity$\r$\n"
  FileWrite $0 "- Device Analytics: Comprehensive device statistics and analytics$\r$\n$\r$\n"
  
  FileWrite $0 "TECHNICAL STACK$\r$\n"
  FileWrite $0 "===============$\r$\n"
  FileWrite $0 "- Language: Rust 2021 Edition$\r$\n"
  FileWrite $0 "- GUI Framework: egui with eframe$\r$\n"
  FileWrite $0 "- USB Monitoring: rusb library for cross-platform USB access$\r$\n"
  FileWrite $0 "- Serialization: Serde with JSON support$\r$\n"
  FileWrite $0 "- Concurrency: Tokio async runtime$\r$\n"
  FileWrite $0 "- Time Handling: Chrono for timestamp management$\r$\n"
  FileWrite $0 "- System Integration: notify-rust, tray-icon, winit$\r$\n$\r$\n"
  
  FileWrite $0 "QUICK START$\r$\n"
  FileWrite $0 "===========$\r$\n$\r$\n"
  
  FileWrite $0 "Prerequisites:$\r$\n"
  FileWrite $0 "- Windows 10/11 (64-bit)$\r$\n"
  FileWrite $0 "- Administrator privilege (recommended for comprehensive monitoring)$\r$\n$\r$\n"
  
  FileWrite $0 "GUI Mode (Default):$\r$\n"
  FileWrite $0 "1. Launch IronWatch from Start Menu or Desktop$\r$\n"
  FileWrite $0 "2. The GUI will automatically start monitoring USB devices$\r$\n"
  FileWrite $0 "3. Use the Dashboard tab for overview and statistics$\r$\n"
  FileWrite $0 "4. Switch to Devices tab for detailed device information$\r$\n"
  FileWrite $0 "5. Configure settings in the Settings tab$\r$\n$\r$\n"
  
  FileWrite $0 "CLI Mode (Advanced):$\r$\n"
  FileWrite $0 "Open Command Prompt or PowerShell in the installation directory:$\r$\n"
  FileWrite $0 "• List devices: ironwatch.exe list$\r$\n"
  FileWrite $0 "• Monitor continuously: ironwatch.exe monitor --continuous$\r$\n"
  FileWrite $0 "• JSON output: ironwatch.exe list --format json$\r$\n"
  FileWrite $0 "• Filter devices: ironwatch.exe monitor --filter camera$\r$\n"
  FileWrite $0 "• Show help: ironwatch.exe --help$\r$\n$\r$\n"
  
  FileWrite $0 "Configuration:$\r$\n"
  FileWrite $0 "- monitoring.poll_interval_ms: Device polling interval$\r$\n"
  FileWrite $0 "- monitoring.auto_start: Auto-start monitoring on launch$\r$\n"
  FileWrite $0 "- monitoring.detect_suspicious_activity: Enable security monitoring$\r$\n"
  FileWrite $0 "- output.default_format: Default output format (table/json/csv)$\r$\n"
  FileWrite $0 "- logging.level: Log level (error/warn/info/debug)$\r$\n$\r$\n"
  
  FileWrite $0 "MONITORING CAPABILITIES$\r$\n"
  FileWrite $0 "======================$\r$\n$\r$\n"
  
  FileWrite $0 "Device Detection:$\r$\n"
  FileWrite $0 "- Real-time USB device connection/disconnection events$\r$\n"
  FileWrite $0 "- Comprehensive device information (VID, PID, manufacturer, etc.)$\r$\n"
  FileWrite $0 "- Device classification and filtering$\r$\n"
  FileWrite $0 "- Connection history and statistics$\r$\n$\r$\n"
  
  FileWrite $0 "Security Features:$\r$\n"
  FileWrite $0 "- Suspicious device activity detection$\r$\n"
  FileWrite $0 "- Device whitelist/blacklist management$\r$\n"
  FileWrite $0 "- Security event logging and alerts$\r$\n"
  FileWrite $0 "- Rate limiting to prevent spam events$\r$\n$\r$\n"
  
  FileWrite $0 "Analytics & Reporting:$\r$\n"
  FileWrite $0 "- Device usage statistics$\r$\n"
  FileWrite $0 "- Connection frequency analysis$\r$\n"
  FileWrite $0 "- Export capabilities (JSON, CSV, TXT)$\r$\n"
  FileWrite $0 "- Historical data visualization$\r$\n$\r$\n"
  
  FileWrite $0 "DIRECTORY STRUCTURE$\r$\n"
  FileWrite $0 "==================$\r$\n"
  FileWrite $0 "• Installation: $INSTDIR$\r$\n"
  FileWrite $0 "• Data Storage: data\$\r$\n"
  FileWrite $0 "• Export Output: export\$\r$\n"
  FileWrite $0 "• Log Files: logs\$\r$\n"
  FileWrite $0 "• Assets: assets\$\r$\n$\r$\n"
  
  FileWrite $0 "OUTPUT FORMATS$\r$\n"
  FileWrite $0 "==============$\r$\n$\r$\n"
  
  FileWrite $0 "Table Format (Default):$\r$\n"
  FileWrite $0 "Bus VID:PID  Address Manufacturer         Product                   Class$\r$\n"
  FileWrite $0 "------------------------------------------------------------------------$\r$\n"
  FileWrite $0 "2   1022:15BA 0       AMD                  USB Controller            09$\r$\n"
  FileWrite $0 "3   5986:118C 1       Generic              Integrated Camera         EF$\r$\n$\r$\n"
  
  FileWrite $0 "JSON Format:$\r$\n"
  FileWrite $0 "Complete device information in structured JSON format$\r$\n"
  FileWrite $0 "Includes timestamps, connection status, and metadata$\r$\n$\r$\n"
  
  FileWrite $0 "CSV Format:$\r$\n"
  FileWrite $0 "Comma-separated values for easy import into spreadsheets$\r$\n"
  FileWrite $0 "Perfect for data analysis and reporting$\r$\n$\r$\n"
  
  FileWrite $0 "TROUBLESHOOTING$\r$\n"
  FileWrite $0 "===============$\r$\n$\r$\n"
  
  FileWrite $0 "Common Issues:$\r$\n"
  FileWrite $0 "• Permission Denied: Run as Administrator$\r$\n"
  FileWrite $0 "• No Devices Detected: Check USB drivers and connections$\r$\n"
  FileWrite $0 "• GUI Won't Start: Ensure graphics drivers are up to date$\r$\n"
  FileWrite $0 "• High CPU Usage: Increase polling interval in settings$\r$\n$\r$\n"
  
  FileWrite $0 "SUPPORT & UPDATES:$\r$\n"
  FileWrite $0 "═══════════════════$\r$\n"
  FileWrite $0 "• GitHub: https://github.com/KnivInstitute/IronWatch$\r$\n"
  FileWrite $0 "• Issues: Report bugs and feature requests on GitHub$\r$\n"
  FileWrite $0 "• Documentation: Check README.md for latest information$\r$\n$\r$\n"
  
  FileWrite $0 "IMPORTANT NOTES:$\r$\n"
  FileWrite $0 "══════════════════$\r$\n"
  FileWrite $0 "• IronWatch works best with Administrator privileges$\r$\n"
  FileWrite $0 "• Windows Defender may show warnings (false positive)$\r$\n"
  FileWrite $0 "• Performance impact is minimal on modern systems$\r$\n"
  FileWrite $0 "• Configuration is stored in JSON format for easy editing$\r$\n$\r$\n"
  
  FileWrite $0 "Happy Monitoring! Stay secure with IronWatch!$\r$\n"
  FileClose $0
  DetailPrint "README.txt generation completed successfully"
  
  ; Write the installation path into the registry
  WriteRegStr HKLM "SOFTWARE\IronWatch" "InstallPath" "$INSTDIR"
  WriteRegStr HKLM "SOFTWARE\IronWatch" "Version" "1.0.0"
  WriteRegStr HKLM "SOFTWARE\IronWatch" "Publisher" "KnivInstitute"
  
  ; Write the uninstall keys for Windows Add/Remove Programs
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "DisplayName" "IronWatch"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "DisplayIcon" "$INSTDIR\assets\icon.png"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "Publisher" "KnivInstitute"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "DisplayVersion" "1.0.0"
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "URLInfoAbout" "https://github.com/KnivInstitute/IronWatch"
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "NoModify" 1
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "NoRepair" 1
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch" "EstimatedSize" 8500  ; Size in KB
  
  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
  
SectionEnd

Section "Desktop Shortcut" SecDesktop
  DetailPrint "Creating desktop shortcut..."
  ; Create enhanced desktop shortcut with description
  CreateShortcut "$DESKTOP\IronWatch.lnk" "$INSTDIR\ironwatch.exe" "" "$INSTDIR\assets\icon.png" 0 SW_SHOWNORMAL "" "IronWatch - Advanced USB Device Monitor"
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  DetailPrint "Creating Start Menu entries..."
  ; Create start menu folder
  CreateDirectory "$SMPROGRAMS\IronWatch"
  
  ; Create enhanced shortcuts with descriptions
  CreateShortcut "$SMPROGRAMS\IronWatch\IronWatch.lnk" "$INSTDIR\ironwatch.exe" "" "$INSTDIR\assets\icon.png" 0 SW_SHOWNORMAL "" "Advanced USB Device Monitor"
  CreateShortcut "$SMPROGRAMS\IronWatch\IronWatch (CLI).lnk" "cmd.exe" "/k cd /d $\"$INSTDIR$\" && echo IronWatch CLI Mode - Type 'ironwatch.exe --help' for commands" "" 0 SW_SHOWNORMAL "" "IronWatch Command Line Interface"
  CreateShortcut "$SMPROGRAMS\IronWatch\Quick Start Guide.lnk" "$INSTDIR\README.txt" "" "" 0 SW_SHOWNORMAL "" "IronWatch Quick Start Guide"
  CreateShortcut "$SMPROGRAMS\IronWatch\Export Folder.lnk" "$INSTDIR\export" "" "" 0 SW_SHOWNORMAL "" "IronWatch Export Directory"
  CreateShortcut "$SMPROGRAMS\IronWatch\Logs Folder.lnk" "$INSTDIR\logs" "" "" 0 SW_SHOWNORMAL "" "IronWatch Log Files"
  CreateShortcut "$SMPROGRAMS\IronWatch\Uninstall IronWatch.lnk" "$INSTDIR\uninstall.exe" "" "" 0 SW_SHOWNORMAL "" "Uninstall IronWatch"
SectionEnd

Section "USB Device Drivers" SecDrivers
  DetailPrint "Configuring USB device access..."
  ; Note: IronWatch uses libusb/rusb which should work with Windows' built-in drivers
  ; This section could be expanded to install specific drivers if needed
  
  ; Create a batch file for driver troubleshooting
  FileOpen $0 "$INSTDIR\check_usb_drivers.bat" w
  FileWrite $0 "@echo off$\r$\n"
  FileWrite $0 "echo IronWatch USB Driver Check$\r$\n"
  FileWrite $0 "echo =============================$\r$\n"
  FileWrite $0 "echo.$\r$\n"
  FileWrite $0 "echo Checking USB devices...$\r$\n"
  FileWrite $0 "wmic path Win32_USBHub get DeviceID,Description$\r$\n"
  FileWrite $0 "echo.$\r$\n"
  FileWrite $0 "echo If you see USB devices listed above, IronWatch should work properly.$\r$\n"
  FileWrite $0 "echo If not, please check Windows Device Manager for USB issues.$\r$\n"
  FileWrite $0 "pause$\r$\n"
  FileClose $0
SectionEnd

Section "Performance Optimization" SecOptimization
  DetailPrint "Optimizing system integration..."
  ; Set high priority for better monitoring performance
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\ironwatch.exe\PerfOptions" "CpuPriorityClass" "3"
  
  ; Create Windows Event Log source
  WriteRegStr HKLM "SYSTEM\CurrentControlSet\Services\EventLog\Application\IronWatch" "EventMessageFile" "$INSTDIR\ironwatch.exe"
  WriteRegDWORD HKLM "SYSTEM\CurrentControlSet\Services\EventLog\Application\IronWatch" "TypesSupported" 7
  
  ; Create a PowerShell script for advanced USB monitoring
  FileOpen $0 "$INSTDIR\advanced_usb_check.ps1" w
  FileWrite $0 "# IronWatch Advanced USB Check$\r$\n"
  FileWrite $0 "Write-Host 'IronWatch Advanced USB Device Check' -ForegroundColor Green$\r$\n"
  FileWrite $0 "Write-Host '=======================================' -ForegroundColor Green$\r$\n"
  FileWrite $0 "Write-Host ''$\r$\n"
  FileWrite $0 "Get-WmiObject -Class Win32_USBControllerDevice | ForEach-Object { [wmi]($$_.Dependent) } | Select-Object Name, DeviceID, Manufacturer | Format-Table -AutoSize$\r$\n"
  FileWrite $0 "Write-Host ''$\r$\n"
  FileWrite $0 "Write-Host 'USB Hub Information:' -ForegroundColor Yellow$\r$\n"
  FileWrite $0 "Get-WmiObject -Class Win32_USBHub | Select-Object Name, DeviceID | Format-Table -AutoSize$\r$\n"
  FileWrite $0 "Read-Host 'Press Enter to continue...'$\r$\n"
  FileClose $0
SectionEnd

Section "System Integration" SecIntegration
  DetailPrint "Setting up system integration..."
  
  ; Add to Windows PATH (optional, for CLI access)
  ; Note: This is commented out by default to avoid PATH pollution
  ; Uncomment if you want CLI access from anywhere
  ; EnVar::SetHKLM
  ; EnVar::AddValue "PATH" "$INSTDIR"
  
  ; Create file associations for IronWatch export files
  WriteRegStr HKCR ".iwlog" "" "IronWatch.LogFile"
  WriteRegStr HKCR "IronWatch.LogFile" "" "IronWatch Log File"
  WriteRegStr HKCR "IronWatch.LogFile\DefaultIcon" "" "$INSTDIR\assets\icon.png"
  WriteRegStr HKCR "IronWatch.LogFile\shell\open\command" "" '"$INSTDIR\ironwatch.exe" "%1"'
  
  ; Register for Windows notification system
  WriteRegStr HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Notifications\Settings\IronWatch" "ShowInActionCenter" "1"
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Notifications\Settings\IronWatch" "Enabled" 1
SectionEnd

; ===============================================================================
; COMPONENT DESCRIPTIONS - ENHANCED INFO
; ===============================================================================

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} "Core IronWatch application files, configuration, and documentation (Required - 8.5MB)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create a desktop shortcut for quick access to IronWatch"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu folder with shortcuts to application, CLI, guide, and settings"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDrivers} "Configure USB device access and create driver troubleshooting tools"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecOptimization} "System integration optimizations for better performance and Windows Event Log integration"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecIntegration} "Advanced system integration including file associations and notifications"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; Installation function
Function .onInit
  ; Check if we're on 64-bit Windows
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "This application requires 64-bit Windows. Installation will be aborted."
    Abort
  ${EndIf}
  
  ; Check for existing installation
  ReadRegStr $R0 HKLM "SOFTWARE\IronWatch" "InstallPath"
  StrCmp $R0 "" done
  
  MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION \
  "IronWatch is already installed at $R0.$\n$\nClick OK to replace the existing installation, or Cancel to exit." \
  IDOK done
  Abort
  
  done:
FunctionEnd

; Uninstaller section
Section "Uninstall"
  ; Stop IronWatch if running
  nsExec::ExecToLog 'taskkill /F /IM ironwatch.exe'
  
  ; Remove files
  Delete "$INSTDIR\ironwatch.exe"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\check_usb_drivers.bat"
  Delete "$INSTDIR\advanced_usb_check.ps1"
  
  ; Remove assets
  Delete "$INSTDIR\assets\icon.png"
  RMDir "$INSTDIR\assets"
  
  ; Remove export directory (but preserve user data)
  IfFileExists "$INSTDIR\export\*.*" 0 no_exports
    MessageBox MB_YESNO|MB_ICONQUESTION "Remove exported data files? This will delete all your exported reports." IDNO no_exports
    Delete "$INSTDIR\export\*.*"
  no_exports:
  RMDir "$INSTDIR\export"
  
  ; Remove log files (ask user first)
  IfFileExists "$INSTDIR\logs\*.*" 0 no_logs
    MessageBox MB_YESNO|MB_ICONQUESTION "Remove log files? This will delete all monitoring history." IDNO no_logs
    Delete "$INSTDIR\logs\*.log"
    Delete "$INSTDIR\logs\*.*"
  no_logs:
  RMDir "$INSTDIR\logs"
  
  ; Remove data directory
  RMDir "$INSTDIR\data"
  
  ; Remove shortcuts
  Delete "$DESKTOP\IronWatch.lnk"
  Delete "$SMPROGRAMS\IronWatch\IronWatch.lnk"
  Delete "$SMPROGRAMS\IronWatch\IronWatch (CLI).lnk"
  Delete "$SMPROGRAMS\IronWatch\Quick Start Guide.lnk"
  Delete "$SMPROGRAMS\IronWatch\Configuration.lnk"
  Delete "$SMPROGRAMS\IronWatch\Export Folder.lnk"
  Delete "$SMPROGRAMS\IronWatch\Logs Folder.lnk"
  Delete "$SMPROGRAMS\IronWatch\Uninstall IronWatch.lnk"
  RMDir "$SMPROGRAMS\IronWatch"
  
  ; Remove registry keys
  DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\IronWatch"
  DeleteRegKey HKLM "SOFTWARE\IronWatch"
  DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\ironwatch.exe"
  DeleteRegKey HKLM "SYSTEM\CurrentControlSet\Services\EventLog\Application\IronWatch"
  DeleteRegKey HKCR ".iwlog"
  DeleteRegKey HKCR "IronWatch.LogFile"
  DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\CurrentVersion\Notifications\Settings\IronWatch"
  
  ; Remove installation directory if empty
  RMDir "$INSTDIR"
  
  ; Success message
  MessageBox MB_OK "IronWatch has been successfully removed from your computer.$\r$\n$\r$\nThank you for using IronWatch!"
  
SectionEnd

; Uninstaller function
Function un.onInit
  MessageBox MB_ICONQUESTION|MB_YESNO|MB_DEFBUTTON2 "Are you sure you want to completely remove IronWatch and all of its components?" IDYES +2
  Abort
FunctionEnd
